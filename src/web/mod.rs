use axum::response::IntoResponse;
use axum::Router;
use axum_login::AuthManagerLayerBuilder;
use axum_messages::MessagesManagerLayer;
use sqlx::{migrate, MySqlPool};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::task::AbortHandle;
use tower::service_fn;
use tower_sessions::cookie::Key;
use tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::MySqlStore;
use tracing::{debug, info};
use crate::Config;
use crate::auth::AuthBackend;
use crate::error::AppError;
use crate::pokedex::update_pokedex_database;

mod login;
mod create;
mod dex;
mod user;
mod index;
mod r#static;

#[derive(Clone)]
pub struct AppState {
    database: MySqlPool,
}

pub struct App {
    state: AppState,
    config: Config,
}

impl App {
    pub async fn new(config: Config) -> Result<Self, AppError> {

        info!("Creating database connection.");
        let pool = MySqlPool::connect(config.database_url.as_str()).await?;
        info!("Running database migrations.");
        migrate!().run(&pool).await?;
        info!("Updating pokedexes.");
        update_pokedex_database(pool.clone()).await?;

        Ok(Self {
            state: AppState {
                database: pool,
            },
            config,
        })
    }

    pub async fn serve(self) -> Result<(), anyhow::Error> {

        // Session layer
        // This uses `tower-sessions` to establish a layer that will provide the session
        // as a request extension.
        let session_store = MySqlStore::new(self.state.database.clone());
        debug!("Running session backend migrations.");
        session_store.migrate().await?;
        let session_deletion_task = tokio::task::spawn(
            session_store
                .clone()
                .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
        );
        // Generate a cryptographic key to sign the session cookie.
        // TODO: Source this from somewhere so we can have multiple instances of this service.
        let key = Key::generate();
        let session_layer = SessionManagerLayer::new(session_store)
            // It would be safe to send the cookie over unencrypted communication
            .with_secure(false)
            .with_expiry(Expiry::OnInactivity(time::Duration::days(1)))
            // Encrypt and sign the cookie. The user does not need its contents.
            .with_private(key);

        // Auth layer
        // This combines the session layer with our auth backend to establish the auth
        // service which will provide the auth session as a request extension.
        let backend = AuthBackend::new(self.state.database.clone());
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        // Create the app
        let app = Router::new()
            .merge(user::router())
            .with_state(self.state.clone())
            .merge(index::router())
            .merge(login::router())
            .merge(r#static::router())
            .fallback(async || AppError::NotFound.into_response())
            .layer(MessagesManagerLayer)
            .layer(auth_layer);

        let listener = TcpListener::bind(self.config.bind_addr.as_str()).await?;
        
        info!("Starting server on {}.", self.config.bind_addr);

        // Ensure we use a shutdown signal to abort the deletion task.
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(shutdown_signal(session_deletion_task.abort_handle()))
            .await?;

        // A join error here just means it was shut down with a signal.
        // That's not an error, but is expected. 
        // We only care about the inner error.
        session_deletion_task.await.ok().transpose()?;

        Ok(())
    }
}

/// Gracefully shuts down the session deletion task when the service exits.
async fn shutdown_signal(deletion_task_abort_handle: AbortHandle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { deletion_task_abort_handle.abort() },
        _ = terminate => { deletion_task_abort_handle.abort() },
    }
}