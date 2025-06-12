use askama::Template;
use axum::response::{Html, IntoResponse};
use axum::Router;
use axum::routing::get;
use crate::error::AppError;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexPageTemplate {

}

pub fn router() -> Router {
    Router::new()
        .route("/", get(get::index))
        .route("/index.html", get(get::index))
}

mod get {
    
    use super::*;

    /// Handler for GET requests to '/'.
    /// Will return the index page as HTML.
    pub async fn index() -> Result<impl IntoResponse, AppError> {
        Ok(Html(IndexPageTemplate {}.render()?))
    }
}