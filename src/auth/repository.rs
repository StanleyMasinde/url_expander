use chrono::{NaiveDateTime, Utc};
use sqlx::{MySql, MySqlPool, Transaction};

use super::models::{RefreshToken, User};

#[derive(Clone)]
pub struct AuthRepository {
    pool: MySqlPool,
}

impl AuthRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn create_user(
        &self,
        name: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<User, sqlx::Error> {
        let result = sqlx::query!(
            r#"INSERT INTO users (name, email, password_hash) VALUES (?, ?, ?)"#,
            name,
            email,
            password_hash
        )
        .execute(&self.pool)
        .await?;

        let user_id = result.last_insert_id();
        self.get_user_by_id(user_id).await
    }

    pub async fn get_user_by_id(&self, user_id: u64) -> Result<User, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT
                id,
                name,
                email,
                password_hash,
                created_at as `created_at!: NaiveDateTime`,
                updated_at as `updated_at!: NaiveDateTime`
            FROM users
            WHERE id = ?
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: row.id,
            name: row.name,
            email: row.email,
            password_hash: row.password_hash,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT
                id,
                name,
                email,
                password_hash,
                created_at as `created_at!: NaiveDateTime`,
                updated_at as `updated_at!: NaiveDateTime`
            FROM users
            WHERE email = ?
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| User {
            id: row.id,
            name: row.name,
            email: row.email,
            password_hash: row.password_hash,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn save_refresh_token(
        &self,
        user_id: u64,
        token_hash: &str,
        expires_at: NaiveDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (user_id, token_hash, expires_at, revoked)
            VALUES (?, ?, ?, FALSE)
            "#,
            user_id,
            token_hash,
            expires_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_refresh_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT
                id,
                user_id,
                token_hash,
                expires_at,
                revoked as `revoked!: bool`,
                created_at as `created_at!: NaiveDateTime`
            FROM refresh_tokens
            WHERE token_hash = ?
            "#,
            token_hash
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| RefreshToken {
            id: row.id,
            user_id: row.user_id,
            token_hash: row.token_hash,
            expires_at: row.expires_at,
            revoked: row.revoked,
            created_at: row.created_at,
        }))
    }

    pub async fn revoke_refresh_token(&self, token_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE refresh_tokens SET revoked = TRUE WHERE token_hash = ?"#,
            token_hash
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn rotate_refresh_token(
        &self,
        old_token_hash: &str,
        new_token_hash: &str,
        user_id: u64,
        new_expires_at: NaiveDateTime,
    ) -> Result<(), sqlx::Error> {
        let mut tx: Transaction<'_, MySql> = self.pool.begin().await?;

        sqlx::query!(
            r#"UPDATE refresh_tokens SET revoked = TRUE WHERE token_hash = ?"#,
            old_token_hash
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (user_id, token_hash, expires_at, revoked)
            VALUES (?, ?, ?, FALSE)
            "#,
            user_id,
            new_token_hash,
            new_expires_at,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn cleanup_expired_refresh_tokens(&self) -> Result<(), sqlx::Error> {
        let now = Utc::now().naive_utc();
        sqlx::query!(
            r#"DELETE FROM refresh_tokens WHERE expires_at < ? OR revoked = TRUE"#,
            now
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
