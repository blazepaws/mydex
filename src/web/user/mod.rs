mod pokedex;
mod settings;

use askama::Template;
use axum::response::{IntoResponse, Html, Redirect};
use axum::Router;
use axum::routing::get;
use axum::extract::{Path, State};
use sqlx::{query_as, query_scalar};

use crate::error::AppError;
use crate::web::AppState;
use crate::auth::AuthSession;

#[derive(sqlx::FromRow, Debug)]
struct PokedexProgress {
    id: String,
    name: String,
    description: String,
    num_entries: i32,
    collected: i64,
    thumbnail_url: String,
}

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate {
    username: String,
    pokedexes: Vec<PokedexProgress>,
    is_own_profile: bool,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/user/{username}", get(get::profile))
        .route("/user", get(get::redirect_to_profile))
        .merge(pokedex::router())
}

mod get {
    
    use super::*;
    
    /// Redirect the user to their own profile if logged in, otherwise redirect to login.
    pub async fn redirect_to_profile(auth_session: AuthSession) -> Result<impl IntoResponse, AppError> {
        match auth_session.user {
            None => Ok(Redirect::to("/login?next=/profile")),
            Some(u) => Ok(Redirect::to(format!("/user/{}", u.name).as_str())),
        }
    }
    
    pub async fn profile(
        Path(username): Path<String>,
        State(state): State<AppState>,
        auth_session: AuthSession,
    ) -> Result<impl IntoResponse, AppError> {
        
        // Check if the user exists and get their user ID.
        let user_id = query_scalar!("select user_id from user where name like ?", username)
            .fetch_optional(&state.database).await?;
        let user_id = user_id.ok_or(AppError::NotFound)?;
        
        // If the user is on their own profile, they may edit it.
        let is_own_profile = match auth_session.user {
            None => false,
            Some(user) => user.user_id == user_id,
        };
        
        // Query Pokédex progress of the user to show on their profile.
        let pokedexes = query_as!(
            PokedexProgress,
            "select 
                pokedex.id, 
                pokedex.name, 
                pokedex.description, 
                pokedex.thumbnail_url, 
                num_entries,
                count(*) as collected
             from user_pokedex, pokedex 
             where user_id = ? and user_pokedex.pokedex_id = pokedex.name
             group by name",
            user_id,
        ).fetch_all(&state.database).await?;
        
        Ok(Html(ProfileTemplate {
            username,
            pokedexes,
            is_own_profile,
        }.render()?))
    }
}