use std::backtrace::Backtrace;
use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use thiserror::Error;
use tokio::task::JoinError;
use tracing::error;

/// This is an HTML string that we can send the user as a last resort when we can't even render the
/// error page. Generally this is not used.
static FALLBACK_ERROR_PAGE: &'static str = "<html><head><title>Error</title></head><body><h1>Internal Server Error</h1></body></html>";

/// The custom error type of the web app as a whole.
/// Most errors generate an impact on the user, so we strongly
/// associate it with an HTTP response.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to render HTML template: {0}.")]
    TemplateRender(#[from] askama::Error),
    #[error("Failed to execute query. {source}")]
    Sqlx {
        #[from]
        source: sqlx::Error,
        backtrace: Backtrace,
    },
    #[error(transparent)]
    JoinError(#[from] JoinError),
    #[error("Not found.")]  
    NotFound,
    #[error("Unauthorized.")]
    Unauthorized,
    #[error("Already exists.")]
    AlreadyExists,
}

impl AppError {

    /// The HTTP status code appropriate for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::TemplateRender(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Sqlx { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::JoinError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound => StatusCode::NOT_FOUND,  
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::AlreadyExists => StatusCode::BAD_REQUEST,
        }
    }

    /// This method returns a string that describes the general error.
    /// It is intended to be simple to understand and not contain any internal information.
    /// This is safe to send to clients.
    pub fn user_facing_error(&self) -> &'static str {
        match self {
            AppError::TemplateRender(_) => "Internal Server Error",
            AppError::Sqlx { .. } => "Internal Server Error",
            AppError::JoinError(_) => "Internal Server Error",
            AppError::NotFound => "Not Found",
            AppError::Unauthorized => "Unauthorized",
            AppError::AlreadyExists => "The resource already exists",
        }
    }
}

// Convert the error to an HTTP response for axum.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Render the error page and return it along with a status to the user.
        let status_code = self.status_code();
        let error_page = ErrorPageTemplate {
            status_code: status_code.as_u16(),
            message: self.user_facing_error(),
        }.render();
        match error_page {
            Ok(error_page) => (status_code, Html(error_page)).into_response(),
            Err(err) => {
                // Something went wrong with rendering the error template too.
                // Fall back to the static fallback page string.
                error!("Failed to render template 'error.html': {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, Html(FALLBACK_ERROR_PAGE)).into_response()
            }
        }
    }
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorPageTemplate<'a> {
    status_code: u16,
    message: &'a str,
}

