use crate::domain::models::Secret;
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SecretRepository: Send + Sync {
    async fn save(&self, secret: Secret) -> Result<()>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Secret>>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Secret>>;
    async fn list_all(&self) -> Result<Vec<Secret>>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}
