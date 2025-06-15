use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use async_trait::async_trait;
use axum_login::{AuthUser, AuthnBackend, AuthzBackend, UserId};
use password_auth::verify_password;
use password_hash::PasswordHash;
use serde::{Deserialize, Serialize};
use sqlx::{query_as, FromRow, MySqlPool};
use time::UtcDateTime;
use tokio::task;
use crate::error::AppError;

/// Unsafe version of the user struct that contains the actual password hash.
/// This contains the algorithm parameters and the password salt.
/// The algorithm parameters aren't strictly private, but we don't need to disclose them
/// unnecessarily. The salt, however, is private and should not leak!
/// Using this struct should be limited to when we query the database and absolutely
/// HAVE to access the password hash (e.g., for authentication).
///
/// To prevent accidentally leaking the password hash,
/// convert to a `User` as soon as possible.
#[derive(Clone, FromRow)]
pub struct UnsafeUser {
    pub user_id: i32,
    pub name: String,
    pub creation_date: UtcDateTime,
    password: String,
}

impl Debug for UnsafeUser {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("user_id", &self.user_id)
            .field("name", &self.name)
            .field("creation_date", &self.creation_date)
            .field("password", &"[redacted]")
            .finish()
    }
}

impl Into<User> for UnsafeUser {
    fn into(self) -> User {

        // Create the session hash from the password hash.
        // This should not be fallible unless something is seriously wrong with our database.
        // The session hash removes the salt from the string.
        // We do encrypt the resulting cookies, but better safe than sorry.
        let password_hash = PasswordHash::new(self.password.as_str())
            .expect(format!("Invalid password hash in the database for user {}!", self.user_id).as_str());
        let password_hash = password_hash.hash
            .expect(format!("No password hash in the database for user {}", self.user_id).as_str());
        let session_hash = password_hash.as_bytes().to_vec();

        User {
            user_id: self.user_id,
            name: self.name,
            creation_date: self.creation_date,
            session_hash,
        }
    }
}

/// This is the normal user struct used everywhere.
#[derive(Clone)]
pub struct User {
    pub user_id: i32,
    pub name: String,
    pub creation_date: UtcDateTime,
    pub session_hash: Vec<u8>,
}

impl Debug for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("user_id", &self.user_id)
            .field("name", &self.name)
            .field("creation_date", &self.creation_date)
            .field("session_hash", &"[redacted]")
            .finish()
    }
}

impl AuthUser for User {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.user_id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.session_hash.as_slice()
    }
}


#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub next: Option<String>,
}

#[derive(Clone)]
pub struct AuthBackend(MySqlPool);

impl AuthBackend {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[async_trait]
impl AuthnBackend for AuthBackend {
    type User = User;
    type Credentials = Credentials;
    type Error = AppError;

    async fn authenticate(&self, creds: Self::Credentials) -> Result<Option<Self::User>, Self::Error> {
        let user = query_as!(
            UnsafeUser, 
            "select * from user where user.name = ?", 
            creds.username
        ).fetch_optional(&self.0).await?;

        let user = task::spawn_blocking(|| {
            // Since the password verification may take some time, we offload it to 
            // a compute worker. This way we avoid blocking IO workers.
            user
                .filter(|user| verify_password(creds.password, &user.password).is_ok())
                .map(|user| user.into())
        }).await?;
        Ok(user)
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let user = query_as!(
            UnsafeUser, 
            "select * from user where user.user_id = ?", 
            user_id
        ).fetch_optional(&self.0).await?;
        Ok(user.map(|u| u.into()))
    }
}

#[async_trait]
impl AuthzBackend for AuthBackend {
    type Permission = Permission;

    async fn get_group_permissions(&self, user: &Self::User) -> Result<HashSet<Self::Permission>, Self::Error> {
        let res = sqlx::query_scalar!(
            "select permission as 'permission: Permission'
            from group_permission
            join user_group on user_group.`group` = group_permission.`group`
            where user_group.user_id = ?",
            user.user_id
        )
            .fetch_all(&self.0)
            .await?;
        Ok(HashSet::from_iter(res.into_iter()))
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum Permission {
    AddRole,
    RemoveRole,
    AddPokedexToOtherProfiles,
    RemovePokedexFromOtherProfiles,
}
pub type AuthSession = axum_login::AuthSession<AuthBackend>;
