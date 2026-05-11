use crate::domain::models::Secret;
use crate::domain::repositories::SecretRepository;
use anyhow::Result;
use async_trait::async_trait;
use sea_orm::{entity::prelude::*, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "secrets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub encrypted_value: Vec<u8>,
    pub nonce: Vec<u8>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub struct SqliteSecretRepository {
    db: DatabaseConnection,
}

impl SqliteSecretRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SecretRepository for SqliteSecretRepository {
    async fn save(&self, secret: Secret) -> Result<()> {
        let active_model = ActiveModel {
            id: Set(secret.id),
            name: Set(secret.name),
            encrypted_value: Set(secret.encrypted_value),
            nonce: Set(secret.nonce),
            created_at: Set(secret.created_at),
            updated_at: Set(secret.updated_at),
        };
        Entity::insert(active_model).exec(&self.db).await?;
        Ok(())
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Secret>> {
        let model = Entity::find()
            .filter(Column::Name.eq(name))
            .one(&self.db)
            .await?;

        Ok(model.map(|m| Secret {
            id: m.id,
            name: m.name,
            encrypted_value: m.encrypted_value,
            nonce: m.nonce,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Secret>> {
        let model = Entity::find_by_id(id).one(&self.db).await?;
        Ok(model.map(|m| Secret {
            id: m.id,
            name: m.name,
            encrypted_value: m.encrypted_value,
            nonce: m.nonce,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }))
    }

    async fn list_all(&self) -> Result<Vec<Secret>> {
        let models = Entity::find().all(&self.db).await?;
        Ok(models.into_iter().map(|m| Secret {
            id: m.id,
            name: m.name,
            encrypted_value: m.encrypted_value,
            nonce: m.nonce,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }).collect())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        Entity::delete_by_id(id).exec(&self.db).await?;
        Ok(())
    }
}
