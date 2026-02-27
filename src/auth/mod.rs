pub mod error;
pub mod models;
pub mod repository;
pub mod service;

use std::sync::Arc;

use dotenvy::dotenv;
use log::{error, info};
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};

use crate::config::AuthConfig;

use self::{repository::AuthRepository, service::AuthService};

pub async fn build_auth_service() -> Result<Arc<AuthService>, String> {
    let _ = dotenv();

    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL must be set for auth".to_string())?;

    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .map_err(|error| format!("failed to connect to database: {error}"))?;

    run_migrations(&pool).await?;

    let config = AuthConfig::from_env()?;
    let repository = AuthRepository::new(pool);

    info!("Authentication service initialized");
    Ok(Arc::new(AuthService::new(repository, config)))
}

async fn run_migrations(pool: &MySqlPool) -> Result<(), String> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|error| {
            error!("failed to run migrations: {}", error);
            format!("migration failed: {error}")
        })
}
