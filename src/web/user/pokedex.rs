use axum::Router;
use axum::routing::{put, get};
use crate::web::AppState;
use axum::response::IntoResponse;
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/user/{username}/pokedex/{pokedex_id}", put(put::pokedex))
        .route("/user/{username}/pokedex/{pokedex_id}", get(get::pokedex))
}

mod put {
    
    use super::*;
    use axum::extract::{Path, State};
    use axum_login::AuthzBackend;
    use http::StatusCode;
    use sqlx::{query, query_scalar};
    use tracing::info;
    use crate::auth::{AuthSession, Permission};
    use crate::web::AppState;

    /// Add a new pokedex to a user's profile
    pub async fn pokedex(
        Path((username, pokedex_id)): Path<(String, String)>,
        State(state): State<AppState>,
        auth_session: AuthSession,
    ) -> Result<impl IntoResponse, AppError> {
        
        // Check user auth
        let user = match auth_session.user {
            None => return Err(AppError::Unauthorized),
            Some(user) => user,
        };
        if user.name != username {
            if !auth_session.backend.has_perm(&user, Permission::AddPokedexToOtherProfiles).await? {
                // A user can't edit the profile of another used unless they have the 
                // required permission.
                return Err(AppError::Unauthorized);
            }
        }
        
        // Check if user already has this pokedex
        let already_has_pokedex = query_scalar!(
            "
            select count(*) 
            from pokedex, user_pokedex, user
            where 
                pokedex.id = user_pokedex.pokedex_id and 
                pokedex.id = ? and 
                user.name = ? and
                user_pokedex.user_id = user.user_id  
            ",
            pokedex_id,
            username
        ).fetch_one(&state.database).await? > 0;
        if already_has_pokedex {
            return Err(AppError::AlreadyExists);
        }
        
        // Insert the pokedex
        query!(
            "insert into user_pokedex values ((select user.user_id from user where user.name = ?), ?)", 
            username, 
            pokedex_id
        ).execute(&state.database).await?;
        
        info!("User {} has added pokedex {} to {}'s profile.", user.name, pokedex_id, username);
        
        Ok(StatusCode::OK)
    }
}

mod get {
    use axum::response::Html;
    use super::*;
    
    /// Get a user's pokedex progress
    pub async fn pokedex() -> Result<impl IntoResponse, AppError> {
        Ok(Html(""))
    }
}