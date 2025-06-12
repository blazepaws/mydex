use axum::Router;
use crate::web::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/user/{username}/pokedex/{pokedex_id}")
}

mod put {
    use axum::extract::{Path, State};
    use axum::response::IntoResponse;
    use axum_login::AuthzBackend;
    use http::StatusCode;
    use sqlx::query_scalar;
    use crate::auth::{AuthSession, Permission, User};
    use crate::error::AppError;
    use crate::web::AppState;

    async fn pokedex(
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
            from pokedex, user_pokedex
            where 
                pokedex.id = user_pokedex.pokedex_id and 
                pokedex.id = ? and 
                user_pokedex.user_id = ?   
            ",
            pokedex_id,
            user.user_id
        ).fetch_one(&state.database).await? > 0;
        if already_has_pokedex {
            return Err(AppError::AlreadyExists);
        }
        
        // Insert the pokedex
        
        
        Ok(StatusCode::OK)
    }
}