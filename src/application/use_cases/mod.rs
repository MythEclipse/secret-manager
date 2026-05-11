use crate::domain::models::Secret;
use crate::domain::repositories::SecretRepository;
use anyhow::Result;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use chrono::Utc;
use rand::{rngs::OsRng, RngCore};
use uuid::Uuid;

use zeroize::Zeroize;

pub struct SecretService<R: SecretRepository> {
    repository: R,
    encryption_key: [u8; 32],
}

impl<R: SecretRepository> Drop for SecretService<R> {
    fn drop(&mut self) {
        self.encryption_key.zeroize();
    }
}

impl<R: SecretRepository> SecretService<R> {
    pub fn new(repository: R, encryption_key: [u8; 32]) -> Result<Self> {
        Ok(Self {
            repository,
            encryption_key,
        })
    }

    pub async fn add_secret(&self, name: String, value: &str) -> Result<()> {
        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|e| anyhow::anyhow!("cipher init error: {}", e))?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let encrypted_value = cipher.encrypt(nonce, value.as_bytes())
            .map_err(|e| anyhow::anyhow!("encryption error: {}", e))?;

        let secret = Secret {
            id: Uuid::new_v4(),
            name,
            encrypted_value,
            nonce: nonce_bytes.to_vec(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.repository.save(secret).await
    }

    pub async fn get_secret(&self, name: &str) -> Result<Option<String>> {
        let secret = self.repository.find_by_name(name).await?;
        match secret {
            Some(s) => {
                let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
                    .map_err(|e| anyhow::anyhow!("cipher init error: {}", e))?;
                let nonce = Nonce::from_slice(&s.nonce);
                let decrypted = cipher.decrypt(nonce, s.encrypted_value.as_ref())
                    .map_err(|e| anyhow::anyhow!("decryption error: {}", e))?;
                Ok(Some(String::from_utf8(decrypted)?))
            }
            None => Ok(None),
        }
    }

    pub async fn list_secrets(&self) -> Result<Vec<String>> {
        let secrets = self.repository.list_all().await?;
        Ok(secrets.into_iter().map(|s| s.name).collect())
    }
}
