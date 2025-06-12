use askama::Template;
use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Redirect, Html},
    routing::{get, post},
    Form, Router,
};
use axum_messages::{Message, Messages};
use serde::Deserialize;

use crate::error::AppError;
use crate::auth::{AuthSession, Credentials};

/// The login page HTML template.
#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    messages: Vec<Message>,
    next: Option<String>,
}

/// This allows us to extract the "next" field from the query string. 
/// We use this to redirect after login.
#[derive(Debug, Deserialize)]
struct NextUrl {
    next: Option<String>,
}

/// Build a router for all login-related routes.
pub fn router() -> Router<()> {
    Router::new()
        .route("/login", post(post::login))
        .route("/login", get(get::login))
        .route("/logout", get(get::logout))
}


mod get {
    
    use super::*;

    pub async fn login(
        messages: Messages,
        Query(NextUrl { next }): Query<NextUrl>,
    ) -> Result<impl IntoResponse, AppError> {
        Ok(Html(LoginTemplate {
            messages: messages.into_iter().collect(),
            next,
        }.render()?))
    }

    pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.logout().await {
            Ok(_) => Redirect::to("/").into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }

}

mod post {
    use tracing::error;
    use super::*;
    
    pub async fn login(
        mut auth_session: AuthSession,
        messages: Messages,
        Form(creds): Form<Credentials>,
    ) -> impl IntoResponse {
        let user = match auth_session.authenticate(creds.clone()).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                messages.error("Invalid credentials");

                let mut login_url = "/login".to_string();
                if let Some(next) = creds.next {
                    login_url = format!("{}?next={}", login_url, next);
                };

                return Redirect::to(&login_url).into_response();
            }
            Err(err) => {
                error!("Auth backend error: {}", err);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response()
            },
        };

        if auth_session.login(&user).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        
        messages.success(format!("Successfully logged in as {}", user.name));

        if let Some(ref next) = creds.next {
            Redirect::to(next)
        } else {
            Redirect::to("/")
        }.into_response()
    }

}