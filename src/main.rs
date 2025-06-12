use std::process::exit;
use envconfig::Envconfig;
use tracing::error;
use tracing_subscriber::EnvFilter;
use crate::web::App;

mod web;
mod error;
mod auth;
mod pokedex;

#[derive(Envconfig)]
pub struct Config {
    #[envconfig(from = "DATABASE_URL")]
    pub database_url: String,
    #[envconfig(from = "BIND_ADDR", default = "127.0.0.1:8000")]
    pub bind_addr: String,
}

async fn run() -> Result<(), anyhow::Error> {
    let config = Config::init_from_env()?;
    let app = App::new(config).await?;
    app.serve().await
}

#[tokio::main]
async fn main() {

    // Set up logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or(
            // Debug configuration. Overwrite this in production by providing the env var.
            "mydex=debug,axum_session_auth=debug,axum_session=warn,sqlx=warn,tower_http=debug".into(),
        )))
        .init();

    // Run the actual server and log any fatal errors before exiting
    if let Err(err) = run().await {
        error!("Fatal error: {err}");
        exit(1);
    }
}
