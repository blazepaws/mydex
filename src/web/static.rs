use axum::Router;
use tower::service_fn;
use tower_http::services::ServeDir;
use axum::response::{IntoResponse, Response};
use std::convert::Infallible;
use axum::extract::Request;

use crate::error::AppError;
use crate::web::r#static::get::not_found;

/// Builds a router for our statically served resources.
/// This includes images, stylesheets, and precompiled HTML.
pub fn router() -> Router {
    let image_serve_dir = ServeDir::new("web/images/")
        .not_found_service(service_fn(not_found));
    let static_serve_dir = ServeDir::new("web/resources/")
        .not_found_service(service_fn(not_found));
    Router::new()
        .nest_service("/image", image_serve_dir)
        .nest_service("/resource", static_serve_dir)
}

mod get {
    
    use super::*;

    /// Serves a static 404 page
    pub async fn not_found(_: Request) -> Result<Response, Infallible> {
        Ok(AppError::NotFound.into_response())
    }
}