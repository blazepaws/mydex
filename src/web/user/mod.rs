mod pokedex;
mod settings;

use askama::Template;
use axum::response::{Html, IntoResponse, Redirect};
use axum::Router;
use axum::routing::get;
use axum::extract::{Path, State};
use sqlx::{query_as, query_scalar, FromRow};

use crate::error::AppError;
use crate::web::AppState;
use crate::auth::AuthSession;

/// Information about a Pokédex a user has added to their profile.
#[derive(sqlx::FromRow, Debug)]
struct PokedexProgress {
    id: String,
    name: String,
    description: String,
    num_entries: i32,
    collected: i64,
    thumbnail_url: String,
}

/// The description of a Pokédex the user has not yet added to their profile.
#[derive(FromRow, Debug)]
struct PokedexDescription {
    id: String,
    name: String,
    description: String,
    num_entries: i32,
    thumbnail_url: String,
}

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate {
    username: String,
    own_pokedexes: Vec<PokedexProgress>,
    other_pokedexes: Vec<PokedexDescription>,
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
            None => Ok(Redirect::to("/login")),
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
        let own_pokedexes = query_as!(
            PokedexProgress,
            "
            select
                pokedex.id,
                pokedex.name,
                pokedex.description,
                pokedex.thumbnail_url,
                num_entries,
                coalesce(counts.collected, 0) as collected
            from
                user_pokedex
                left join
                (
                    select user_pokedex_progress.pokedex_id, count(*) as collected
                    from user_pokedex_progress
                    where user_pokedex_progress.user_id = ?
                    group by user_pokedex_progress.pokedex_id
                ) counts
                on counts.pokedex_id = user_pokedex.pokedex_id,
                pokedex
            where user_id = ? and user_pokedex.pokedex_id = pokedex.id
            ",
            user_id,
            user_id
        ).fetch_all(&state.database).await?;
        
        let other_pokedexes = if is_own_profile {
            // Query list of pokedexes the user does not have
            sqlx::query_as!(
                PokedexDescription, 
                "
                select pokedex.id, pokedex.name, pokedex.description, pokedex.num_entries, pokedex.thumbnail_url
                from pokedex
                where pokedex.id not in (
                    select user_pokedex.pokedex_id
                    from user, user_pokedex
                    where user.name like ? and user.user_id = user_pokedex.user_id
                )
                ",
                username
            ).fetch_all(&state.database).await?
        } else {
            vec![]
        };
        
        Ok(Html(ProfileTemplate {
            username,
            own_pokedexes,
            other_pokedexes,
            is_own_profile,
        }.render()?))
    }
}