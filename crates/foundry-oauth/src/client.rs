//! OAuth client

use crate::{OAuthProvider, OAuthUser, Result};
use std::collections::HashMap;

pub struct OAuthClient {
    providers: HashMap<String, Box<dyn OAuthProvider>>,
}

impl OAuthClient {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn register_provider(&mut self, provider: Box<dyn OAuthProvider>) {
        let name = provider.name().to_string();
        self.providers.insert(name, provider);
    }

    pub fn get_provider(&self, name: &str) -> Option<&dyn OAuthProvider> {
        self.providers.get(name).map(|p| p.as_ref())
    }

    pub async fn authenticate(&self, provider_name: &str, code: &str) -> Result<OAuthUser> {
        let provider = self.get_provider(provider_name)
            .ok_or_else(|| crate::OAuthError::ProviderError("Provider not found".to_string()))?;

        let token = provider.exchange_code(code).await?;
        provider.get_user(&token).await
    }
}

impl Default for OAuthClient {
    fn default() -> Self {
        Self::new()
    }
}
