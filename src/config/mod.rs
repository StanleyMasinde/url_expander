use std::env;

use jsonwebtoken::Algorithm;

#[derive(Clone)]
pub enum JwtMode {
    Hs256 {
        secret: String,
    },
    Rs256 {
        private_key_pem: String,
        public_key_pem: String,
    },
}

#[derive(Clone)]
pub struct AuthConfig {
    pub access_token_ttl_secs: i64,
    pub refresh_token_ttl_secs: i64,
    pub jwt_issuer: String,
    pub env: String,
    pub jwt_mode: JwtMode,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, String> {
        let env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

        let access_token_ttl_secs = env::var("ACCESS_TOKEN_TTL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15 * 60);

        let refresh_token_ttl_secs = env::var("REFRESH_TOKEN_TTL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30 * 24 * 60 * 60);

        let jwt_issuer = env::var("JWT_ISSUER").unwrap_or_else(|_| "link_expander".to_string());

        let jwt_mode = if env == "production" {
            let private_key_pem = env::var("JWT_PRIVATE_KEY_PEM")
                .map_err(|_| "JWT_PRIVATE_KEY_PEM is required in production".to_string())?;
            let public_key_pem = env::var("JWT_PUBLIC_KEY_PEM")
                .map_err(|_| "JWT_PUBLIC_KEY_PEM is required in production".to_string())?;
            JwtMode::Rs256 {
                private_key_pem: normalize_pem(private_key_pem),
                public_key_pem: normalize_pem(public_key_pem),
            }
        } else {
            let secret = env::var("JWT_HS256_SECRET")
                .unwrap_or_else(|_| "dev-insecure-secret-change-me".to_string());
            JwtMode::Hs256 { secret }
        };

        Ok(Self {
            access_token_ttl_secs,
            refresh_token_ttl_secs,
            jwt_issuer,
            env,
            jwt_mode,
        })
    }

    pub fn algorithm(&self) -> Algorithm {
        match self.jwt_mode {
            JwtMode::Hs256 { .. } => Algorithm::HS256,
            JwtMode::Rs256 { .. } => Algorithm::RS256,
        }
    }

    pub fn require_https(&self) -> bool {
        self.env == "production"
    }
}

fn normalize_pem(value: String) -> String {
    value.replace("\\n", "\n")
}
