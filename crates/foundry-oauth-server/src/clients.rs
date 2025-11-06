//! OAuth2 Client Repository
//!
//! Manages OAuth2 client registration, authentication, and validation

use crate::errors::{OAuth2Error, OAuth2Result};
use crate::models::Client;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Client Repository Trait
#[async_trait]
pub trait ClientRepository: Send + Sync {
    /// Find client by ID
    async fn find(&self, client_id: Uuid) -> OAuth2Result<Option<Client>>;

    /// Find client by credentials (client_id + secret)
    async fn find_by_credentials(
        &self,
        client_id: Uuid,
        client_secret: &str,
    ) -> OAuth2Result<Option<Client>>;

    /// Store a new client
    async fn store(&self, client: Client) -> OAuth2Result<Client>;

    /// Update client
    async fn update(&self, client: Client) -> OAuth2Result<Client>;

    /// Delete client
    async fn delete(&self, client_id: Uuid) -> OAuth2Result<()>;

    /// Revoke client
    async fn revoke(&self, client_id: Uuid) -> OAuth2Result<()>;

    /// List all clients
    async fn list(&self) -> OAuth2Result<Vec<Client>>;
}

/// In-Memory Client Repository (for development/testing)
pub struct InMemoryClientRepository {
    clients: Arc<RwLock<HashMap<Uuid, Client>>>,
    secrets: Arc<RwLock<HashMap<Uuid, String>>>, // Hashed secrets
}

impl InMemoryClientRepository {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            secrets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Hash client secret using Argon2
    fn hash_secret(&self, secret: &str) -> OAuth2Result<String> {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let argon2 = Argon2::default();

        argon2
            .hash_password(secret.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to hash secret: {}", e)))
    }

    /// Verify client secret
    fn verify_secret(&self, secret: &str, hash: &str) -> bool {
        let Ok(parsed_hash) = PasswordHash::new(hash) else {
            return false;
        };

        Argon2::default()
            .verify_password(secret.as_bytes(), &parsed_hash)
            .is_ok()
    }
}

impl Default for InMemoryClientRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ClientRepository for InMemoryClientRepository {
    async fn find(&self, client_id: Uuid) -> OAuth2Result<Option<Client>> {
        let clients = self.clients.read()
            .map_err(|e| OAuth2Error::InternalError(format!("Lock poisoned: {}", e)))?;
        Ok(clients.get(&client_id).cloned())
    }

    async fn find_by_credentials(
        &self,
        client_id: Uuid,
        client_secret: &str,
    ) -> OAuth2Result<Option<Client>> {
        let clients = self.clients.read()
            .map_err(|e| OAuth2Error::InternalError(format!("Lock poisoned: {}", e)))?;
        let secrets = self.secrets.read()
            .map_err(|e| OAuth2Error::InternalError(format!("Lock poisoned: {}", e)))?;

        let Some(client) = clients.get(&client_id) else {
            return Ok(None);
        };

        // Public clients don't have secrets
        if !client.is_confidential() {
            return Err(OAuth2Error::InvalidClient(
                "Public client cannot authenticate with secret".to_string(),
            ));
        }

        // Verify secret
        let Some(stored_hash) = secrets.get(&client_id) else {
            return Ok(None);
        };

        if self.verify_secret(client_secret, stored_hash) {
            Ok(Some(client.clone()))
        } else {
            Ok(None)
        }
    }

    async fn store(&self, mut client: Client) -> OAuth2Result<Client> {
        // Hash secret if confidential client
        if let Some(secret) = &client.secret {
            let hashed = self.hash_secret(secret)?;
            let mut secrets = self.secrets.write()
                .map_err(|e| OAuth2Error::InternalError(format!("Lock poisoned: {}", e)))?;
            secrets.insert(client.id, hashed);

            // Don't store plaintext secret in client object
            client.secret = Some("***REDACTED***".to_string());
        }

        let mut clients = self.clients.write()
            .map_err(|e| OAuth2Error::InternalError(format!("Lock poisoned: {}", e)))?;
        clients.insert(client.id, client.clone());

        Ok(client)
    }

    async fn update(&self, client: Client) -> OAuth2Result<Client> {
        let mut clients = self.clients.write()
            .map_err(|e| OAuth2Error::InternalError(format!("Lock poisoned: {}", e)))?;

        if !clients.contains_key(&client.id) {
            return Err(OAuth2Error::InvalidClient("Client not found".to_string()));
        }

        clients.insert(client.id, client.clone());
        Ok(client)
    }

    async fn delete(&self, client_id: Uuid) -> OAuth2Result<()> {
        let mut clients = self.clients.write()
            .map_err(|e| OAuth2Error::InternalError(format!("Lock poisoned: {}", e)))?;
        let mut secrets = self.secrets.write()
            .map_err(|e| OAuth2Error::InternalError(format!("Lock poisoned: {}", e)))?;

        clients.remove(&client_id);
        secrets.remove(&client_id);

        Ok(())
    }

    async fn revoke(&self, client_id: Uuid) -> OAuth2Result<()> {
        let mut clients = self.clients.write()
            .map_err(|e| OAuth2Error::InternalError(format!("Lock poisoned: {}", e)))?;

        let Some(client) = clients.get_mut(&client_id) else {
            return Err(OAuth2Error::InvalidClient("Client not found".to_string()));
        };

        client.revoked = true;
        Ok(())
    }

    async fn list(&self) -> OAuth2Result<Vec<Client>> {
        let clients = self.clients.read()
            .map_err(|e| OAuth2Error::InternalError(format!("Lock poisoned: {}", e)))?;
        Ok(clients.values().cloned().collect())
    }
}

/// Client Builder
pub struct ClientBuilder {
    name: Option<String>,
    redirect_uris: Vec<String>,
    grants: Vec<String>,
    scopes: Vec<String>,
    confidential: bool,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            redirect_uris: Vec::new(),
            grants: vec!["authorization_code".to_string(), "refresh_token".to_string()],
            scopes: vec!["*".to_string()],
            confidential: true,
        }
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn redirect_uri(mut self, uri: String) -> Self {
        self.redirect_uris.push(uri);
        self
    }

    pub fn redirect_uris(mut self, uris: Vec<String>) -> Self {
        self.redirect_uris = uris;
        self
    }

    pub fn grant(mut self, grant: String) -> Self {
        if !self.grants.contains(&grant) {
            self.grants.push(grant);
        }
        self
    }

    pub fn grants(mut self, grants: Vec<String>) -> Self {
        self.grants = grants;
        self
    }

    pub fn scope(mut self, scope: String) -> Self {
        if !self.scopes.contains(&scope) {
            self.scopes.push(scope);
        }
        self
    }

    pub fn scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    pub fn public(mut self) -> Self {
        self.confidential = false;
        self
    }

    pub fn confidential(mut self) -> Self {
        self.confidential = true;
        self
    }

    pub fn build(self) -> OAuth2Result<Client> {
        let name = self
            .name
            .ok_or_else(|| OAuth2Error::InvalidRequest("Client name required".to_string()))?;

        if self.redirect_uris.is_empty() {
            return Err(OAuth2Error::InvalidRequest(
                "At least one redirect URI required".to_string(),
            ));
        }

        let client = if self.confidential {
            Client::new(name, self.redirect_uris)
        } else {
            Client::public(name, self.redirect_uris)
        };

        Ok(Client {
            grants: self.grants,
            scopes: self.scopes,
            ..client
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_find_client() {
        let repo = InMemoryClientRepository::new();

        let client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        let client_id = client.id;

        let stored = repo.store(client).await.unwrap();
        assert_eq!(stored.secret, Some("***REDACTED***".to_string()));

        let found = repo.find(client_id).await.unwrap().unwrap();
        assert_eq!(found.name, "Test App");
    }

    #[tokio::test]
    async fn test_authenticate_client() {
        let repo = InMemoryClientRepository::new();

        let client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        let client_id = client.id;
        let secret = client.secret.clone().unwrap();

        repo.store(client).await.unwrap();

        // Valid credentials
        let authenticated = repo
            .find_by_credentials(client_id, &secret)
            .await
            .unwrap();
        assert!(authenticated.is_some());

        // Invalid credentials
        let failed = repo
            .find_by_credentials(client_id, "wrong_secret")
            .await
            .unwrap();
        assert!(failed.is_none());
    }

    #[tokio::test]
    async fn test_public_client() {
        let repo = InMemoryClientRepository::new();

        let client = Client::public(
            "Public App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        let client_id = client.id;

        repo.store(client).await.unwrap();

        // Public client cannot authenticate with secret
        let result = repo
            .find_by_credentials(client_id, "any_secret")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revoke_client() {
        let repo = InMemoryClientRepository::new();

        let client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        let client_id = client.id;

        repo.store(client).await.unwrap();
        repo.revoke(client_id).await.unwrap();

        let revoked = repo.find(client_id).await.unwrap().unwrap();
        assert!(revoked.revoked);
    }

    #[tokio::test]
    async fn test_client_builder() {
        let client = ClientBuilder::new()
            .name("My App".to_string())
            .redirect_uri("http://localhost/callback".to_string())
            .grant("client_credentials".to_string())
            .scope("users:read".to_string())
            .public()
            .build()
            .unwrap();

        assert_eq!(client.name, "My App");
        assert!(client.grants.contains(&"client_credentials".to_string()));
        assert!(client.scopes.contains(&"users:read".to_string()));
        assert!(!client.is_confidential());
    }

    #[tokio::test]
    async fn test_list_clients() {
        let repo = InMemoryClientRepository::new();

        let client1 = Client::new("App 1".to_string(), vec!["http://localhost".to_string()]);
        let client2 = Client::new("App 2".to_string(), vec!["http://localhost".to_string()]);

        repo.store(client1).await.unwrap();
        repo.store(client2).await.unwrap();

        let clients = repo.list().await.unwrap();
        assert_eq!(clients.len(), 2);
    }
}
