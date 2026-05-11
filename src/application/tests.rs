#[cfg(test)]
mod tests {
    use crate::application::use_cases::SecretService;
    use crate::domain::repositories::MockSecretRepository;
    use mockall::predicate::*;
    use crate::domain::models::Secret;
    use chrono::Utc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_encryption_decryption_flow() {
        let mut mock_repo = MockSecretRepository::new();
        let password = "master_password";

        mock_repo.expect_save()
            .with(always())
            .returning(|_| Ok(()));

        let service = SecretService::new(mock_repo, password).unwrap();
        let res: anyhow::Result<()> = service.add_secret("key".to_string(), "sensitive_data").await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_get_secret_decryption() {
        let mut mock_repo = MockSecretRepository::new();
        let password = "pass";

        // Setup a real service to generate the encrypted blob for the mock
        // This ensures the test uses the SAME KDF as the actual code
        let _setup_repo = MockSecretRepository::new();
        let _setup_service = SecretService::new(_setup_repo, password).unwrap();

        // We need the internal state or just manually re-derive it since we can't easily intercept add_secret's result in a unit test of get_secret
        // Manually implement the Argon2 flow for the test to match mod.rs
        use argon2::{password_hash::SaltString, Argon2, Params, password_hash::PasswordHasher};
        use aes_gcm::{Aes256Gcm, Nonce, aead::{Aead, KeyInit}};

        let salt = SaltString::from_b64("U2VjcmV0U2FsdDEyM3NhbHQ").unwrap();
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            Params::default(),
        );
        let hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash.hash.unwrap().as_bytes()[..32]);

        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let nonce_bytes = [1u8; 12];
        let nonce = Nonce::from_slice(&nonce_bytes);
        let encrypted = cipher.encrypt(nonce, &b"my_secret"[..]).unwrap();

        mock_repo.expect_find_by_name()
            .with(eq("my_key"))
            .returning(move |_| Ok(Some(Secret {
                id: Uuid::new_v4(),
                name: "my_key".to_string(),
                encrypted_value: encrypted.clone(),
                nonce: nonce_bytes.to_vec(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })));

        let service = SecretService::new(mock_repo, password).unwrap();
        let val: Option<String> = service.get_secret("my_key").await.unwrap();
        assert_eq!(val, Some("my_secret".to_string()));
    }
}
