use std::{sync::Arc, time::Duration};

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::{Duration as ChronoDuration, Utc};
use dashmap::DashMap;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::{Rng, distr::Alphanumeric};
use sha2::{Digest, Sha256};

use crate::config::{AuthConfig, JwtMode};

use super::{error::AuthError, models::AuthTokensResponse, repository::AuthRepository};

#[derive(Clone)]
pub struct AuthService {
    repository: AuthRepository,
    config: AuthConfig,
    login_limiter: Arc<LoginLimiter>,
}

#[derive(Clone)]
struct LoginLimiter {
    by_email: Arc<DashMap<String, Vec<chrono::DateTime<Utc>>>>,
    by_ip: Arc<DashMap<String, Vec<chrono::DateTime<Utc>>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AccessTokenClaims {
    sub: u64,
    iss: String,
    exp: i64,
    iat: i64,
}

impl LoginLimiter {
    fn new() -> Self {
        Self {
            by_email: Arc::new(DashMap::new()),
            by_ip: Arc::new(DashMap::new()),
        }
    }

    fn check_and_record(&self, email: &str, ip: &str) -> Result<(), AuthError> {
        let email_allowed =
            Self::record_and_check(&self.by_email, email, 5, Duration::from_secs(900));
        let ip_allowed = Self::record_and_check(&self.by_ip, ip, 20, Duration::from_secs(900));

        if email_allowed && ip_allowed {
            Ok(())
        } else {
            Err(AuthError::RateLimited)
        }
    }

    fn record_and_check(
        map: &DashMap<String, Vec<chrono::DateTime<Utc>>>,
        key: &str,
        max_attempts: usize,
        window: Duration,
    ) -> bool {
        let now = Utc::now();
        let threshold =
            now - ChronoDuration::from_std(window).unwrap_or_else(|_| ChronoDuration::minutes(15));

        let mut entry = map.entry(key.to_string()).or_default();
        entry.retain(|timestamp| *timestamp > threshold);
        entry.push(now);

        entry.len() <= max_attempts
    }
}

impl AuthService {
    pub fn new(repository: AuthRepository, config: AuthConfig) -> Self {
        Self {
            repository,
            config,
            login_limiter: Arc::new(LoginLimiter::new()),
        }
    }

    pub async fn register(
        &self,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthTokensResponse, AuthError> {
        validate_registration_input(name, email, password)?;

        let email_lower = email.to_lowercase();
        let existing = self
            .repository
            .find_user_by_email(&email_lower)
            .await
            .map_err(|_| AuthError::Internal)?;
        if existing.is_some() {
            return Err(AuthError::Conflict {
                message: "An account with this email already exists",
            });
        }

        let password_hash = hash_password(password)?;
        let user = self
            .repository
            .create_user(name.trim(), &email_lower, &password_hash)
            .await
            .map_err(|_| AuthError::Internal)?;

        self.issue_and_store_tokens(user.id).await
    }

    pub async fn login(
        &self,
        email: &str,
        password: &str,
        ip: &str,
    ) -> Result<AuthTokensResponse, AuthError> {
        validate_email(email)?;

        self.login_limiter
            .check_and_record(&email.to_lowercase(), ip)?;

        let user = self
            .repository
            .find_user_by_email(&email.to_lowercase())
            .await
            .map_err(|_| AuthError::Internal)?
            .ok_or(AuthError::InvalidCredentials)?;

        if !verify_password(password, &user.password_hash) {
            return Err(AuthError::InvalidCredentials);
        }

        self.issue_and_store_tokens(user.id).await
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<AuthTokensResponse, AuthError> {
        if refresh_token.trim().is_empty() {
            return Err(AuthError::Unauthorized);
        }

        let old_hash = hash_refresh_token(refresh_token);
        let stored = self
            .repository
            .find_refresh_token(&old_hash)
            .await
            .map_err(|_| AuthError::Internal)?
            .ok_or(AuthError::Unauthorized)?;

        if stored.revoked || stored.expires_at < Utc::now().naive_utc() {
            return Err(AuthError::Unauthorized);
        }

        let access_token = self.generate_access_token(stored.user_id)?;
        let new_refresh_token = generate_refresh_token();
        let new_hash = hash_refresh_token(&new_refresh_token);
        let expires_at =
            Utc::now().naive_utc() + ChronoDuration::seconds(self.config.refresh_token_ttl_secs);

        self.repository
            .rotate_refresh_token(&old_hash, &new_hash, stored.user_id, expires_at)
            .await
            .map_err(|_| AuthError::Internal)?;

        Ok(AuthTokensResponse {
            access_token,
            refresh_token: new_refresh_token,
            expires_in: self.config.access_token_ttl_secs,
            token_type: "Bearer".to_string(),
        })
    }

    pub async fn logout(&self, refresh_token: &str) -> Result<(), AuthError> {
        if refresh_token.trim().is_empty() {
            return Err(AuthError::Unauthorized);
        }

        self.repository
            .revoke_refresh_token(&hash_refresh_token(refresh_token))
            .await
            .map_err(|_| AuthError::Internal)?;

        Ok(())
    }

    pub fn validate_access_token(&self, token: &str) -> Result<u64, AuthError> {
        let mut validation = Validation::new(self.config.algorithm());
        validation.validate_exp = true;
        validation.set_issuer(&[self.config.jwt_issuer.as_str()]);

        let decoded = decode::<AccessTokenClaims>(token, &self.decoding_key()?, &validation)
            .map_err(|_| AuthError::Unauthorized)?;

        Ok(decoded.claims.sub)
    }

    pub async fn user_profile(&self, user_id: u64) -> Result<(u64, String, String), AuthError> {
        let user = self
            .repository
            .get_user_by_id(user_id)
            .await
            .map_err(|_| AuthError::Unauthorized)?;

        Ok((user.id, user.email, user.name))
    }

    async fn issue_and_store_tokens(&self, user_id: u64) -> Result<AuthTokensResponse, AuthError> {
        let access_token = self.generate_access_token(user_id)?;
        let refresh_token = generate_refresh_token();
        let refresh_hash = hash_refresh_token(&refresh_token);
        let expires_at =
            Utc::now().naive_utc() + ChronoDuration::seconds(self.config.refresh_token_ttl_secs);

        self.repository
            .save_refresh_token(user_id, &refresh_hash, expires_at)
            .await
            .map_err(|_| AuthError::Internal)?;

        Ok(AuthTokensResponse {
            access_token,
            refresh_token,
            expires_in: self.config.access_token_ttl_secs,
            token_type: "Bearer".to_string(),
        })
    }

    fn generate_access_token(&self, user_id: u64) -> Result<String, AuthError> {
        let now = Utc::now();
        let claims = AccessTokenClaims {
            sub: user_id,
            iss: self.config.jwt_issuer.clone(),
            iat: now.timestamp(),
            exp: (now + ChronoDuration::seconds(self.config.access_token_ttl_secs)).timestamp(),
        };

        encode(
            &Header::new(self.config.algorithm()),
            &claims,
            &self.encoding_key()?,
        )
        .map_err(|_| AuthError::Internal)
    }

    fn encoding_key(&self) -> Result<EncodingKey, AuthError> {
        match &self.config.jwt_mode {
            JwtMode::Hs256 { secret } => Ok(EncodingKey::from_secret(secret.as_bytes())),
            JwtMode::Rs256 {
                private_key_pem, ..
            } => EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
                .map_err(|_| AuthError::Internal),
        }
    }

    fn decoding_key(&self) -> Result<DecodingKey, AuthError> {
        match &self.config.jwt_mode {
            JwtMode::Hs256 { secret } => Ok(DecodingKey::from_secret(secret.as_bytes())),
            JwtMode::Rs256 { public_key_pem, .. } => {
                DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
                    .map_err(|_| AuthError::Internal)
            }
        }
    }

    pub fn is_https_required(&self) -> bool {
        self.config.require_https()
    }
}

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthError::Internal)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn generate_refresh_token() -> String {
    let mut rng = rand::rng();
    (&mut rng)
        .sample_iter(Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

fn validate_registration_input(name: &str, email: &str, password: &str) -> Result<(), AuthError> {
    validate_name(name)?;
    validate_email(email)?;
    validate_password(password)?;
    Ok(())
}

fn validate_name(name: &str) -> Result<(), AuthError> {
    let trimmed = name.trim();
    if (2..=255).contains(&trimmed.len()) {
        Ok(())
    } else {
        Err(AuthError::Validation {
            field: "name",
            message: "Name must be between 2 and 255 characters",
        })
    }
}

fn validate_email(email: &str) -> Result<(), AuthError> {
    if email_address::EmailAddress::is_valid(email.trim()) {
        Ok(())
    } else {
        Err(AuthError::Validation {
            field: "email",
            message: "Please provide a valid email address",
        })
    }
}

fn validate_password(password: &str) -> Result<(), AuthError> {
    if password.len() >= 8 {
        Ok(())
    } else {
        Err(AuthError::Validation {
            field: "password",
            message: "Password must be at least 8 characters long",
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};

    use super::{AccessTokenClaims, hash_password, verify_password};

    #[test]
    fn password_hash_and_verify() {
        let password = "strong-password";
        let hash = hash_password(password).expect("hash should be generated");

        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn jwt_generation_and_validation() {
        let key = "test-secret";
        let now = Utc::now();
        let claims = AccessTokenClaims {
            sub: 42,
            iss: "link_expander".to_string(),
            exp: (now + Duration::minutes(15)).timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(key.as_bytes()),
        )
        .expect("token should be encoded");

        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["link_expander"]);

        let decoded = decode::<AccessTokenClaims>(
            &token,
            &DecodingKey::from_secret(key.as_bytes()),
            &validation,
        )
        .expect("token should decode");

        assert_eq!(decoded.claims.sub, 42);
    }

    #[test]
    fn token_expiration_is_enforced() {
        let key = "test-secret";
        let now = Utc::now();
        let claims = AccessTokenClaims {
            sub: 1,
            iss: "link_expander".to_string(),
            exp: (now - Duration::seconds(1)).timestamp(),
            iat: (now - Duration::minutes(2)).timestamp(),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(key.as_bytes()),
        )
        .expect("token should be encoded");

        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["link_expander"]);

        let result = decode::<AccessTokenClaims>(
            &token,
            &DecodingKey::from_secret(key.as_bytes()),
            &validation,
        );

        assert!(result.is_err());
    }
}
