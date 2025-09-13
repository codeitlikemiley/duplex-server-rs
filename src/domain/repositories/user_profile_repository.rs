use async_trait::async_trait;
use uuid::Uuid;

use crate::models::UserProfile;

#[async_trait]
pub trait UserProfileRepository {
    async fn save_profile(&self, profile: UserProfile) -> Result<(), sqlx::Error>;
    async fn find_profile_by_user_id(&self, user_id: Uuid) -> Result<Option<UserProfile>, sqlx::Error>;
    async fn update_profile(&self, profile: UserProfile) -> Result<(), sqlx::Error>;
    async fn delete_profile(&self, user_id: Uuid) -> Result<(), sqlx::Error>;
    async fn profile_exists(&self, user_id: Uuid) -> Result<bool, sqlx::Error>;
}