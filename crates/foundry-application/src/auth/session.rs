use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use super::guard::{AuthError, Authenticatable, Credentials, Guard, Provider};

#[derive(Clone, Debug)]
pub struct Session {
    pub id: String,
    pub user_id: Option<i64>,
    pub data: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    pub fn new(id: impl Into<String>, ttl: Duration) -> Self {
        let id = id.into();
        let now = Utc::now();
        Self {
            id,
            user_id: None,
            data: HashMap::new(),
            created_at: now,
            last_activity: now,
            expires_at: now
                + chrono::Duration::from_std(ttl).unwrap_or_else(|_| chrono::Duration::hours(2)),
        }
    }

    pub fn refresh(&mut self, ttl: Duration) {
        let now = Utc::now();
        self.last_activity = now;
        self.expires_at =
            now + chrono::Duration::from_std(ttl).unwrap_or_else(|_| chrono::Duration::hours(2));
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn put(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.data.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    pub fn remove(&mut self, key: &str) {
        self.data.remove(key);
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

/// Trait for session storage backends
#[async_trait::async_trait]
pub trait SessionStore: Send + Sync {
    async fn create(&self, session_id: impl Into<String> + Send) -> Session;
    async fn load(&self, session_id: &str) -> Option<Session>;
    async fn save(&self, session: Session);
    async fn remove(&self, session_id: &str);
}

/// In-memory session store implementation
#[derive(Clone)]
pub struct InMemorySessionStore {
    ttl: Duration,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl InMemorySessionStore {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl SessionStore for InMemorySessionStore {
    async fn create(&self, session_id: impl Into<String> + Send) -> Session {
        let mut session = Session::new(session_id.into(), self.ttl);
        session.refresh(self.ttl);
        self.sessions
            .write()
            .await
            .insert(session.id.clone(), session.clone());
        session
    }

    async fn load(&self, session_id: &str) -> Option<Session> {
        let mut guard = self.sessions.write().await;
        match guard.get_mut(session_id) {
            Some(session) if !session.is_expired() => {
                session.refresh(self.ttl);
                Some(session.clone())
            }
            Some(_) => {
                guard.remove(session_id);
                None
            }
            None => None,
        }
    }

    async fn save(&self, session: Session) {
        self.sessions
            .write()
            .await
            .insert(session.id.clone(), session);
    }

    async fn remove(&self, session_id: &str) {
        self.sessions.write().await.remove(session_id);
    }
}

pub struct SessionGuard<P: Provider, S: SessionStore> {
    provider: Arc<P>,
    store: Arc<S>,
    session: RwLock<Session>,
    current_user: RwLock<Option<P::User>>,
}

impl<P: Provider, S: SessionStore> SessionGuard<P, S>
where
    P::User: Authenticatable,
{
    pub async fn new(
        provider: Arc<P>,
        store: Arc<S>,
        session_id: impl Into<String>,
    ) -> Result<Self, AuthError> {
        let session_id = session_id.into();
        let session = match store.load(&session_id).await {
            Some(session) => session,
            None => store.create(session_id).await,
        };

        let current_user = if let Some(user_id) = session.user_id {
            provider.retrieve_by_id(user_id).await?
        } else {
            None
        };

        Ok(Self {
            provider,
            store,
            session: RwLock::new(session),
            current_user: RwLock::new(current_user),
        })
    }

    async fn persist(&self) {
        let session = self.session.read().await.clone();
        self.store.save(session).await;
    }
}

#[async_trait]
impl<P, S> Guard for SessionGuard<P, S>
where
    P: Provider + 'static,
    P::User: Authenticatable,
    S: SessionStore + 'static,
{
    type User = P::User;

    async fn check(&self) -> bool {
        self.current_user.read().await.is_some()
    }

    async fn user(&self) -> Option<Self::User> {
        self.current_user.read().await.clone()
    }

    async fn attempt(&self, credentials: Credentials) -> Result<Self::User, AuthError> {
        let user = self
            .provider
            .retrieve_by_credentials(&credentials)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        if !user.is_active() {
            return Err(AuthError::Unauthorized);
        }

        if !self
            .provider
            .validate_credentials(&user, &credentials.password)
            .await
        {
            return Err(AuthError::InvalidCredentials);
        }

        self.login(user.clone()).await?;
        Ok(user)
    }

    async fn login(&self, user: Self::User) -> Result<(), AuthError> {
        {
            let mut session = self.session.write().await;
            session.user_id = Some(user.get_auth_id());
            session.refresh(self.store.ttl);
        }
        {
            let mut guard = self.current_user.write().await;
            *guard = Some(user);
        }
        self.persist().await;
        Ok(())
    }

    async fn logout(&self) -> Result<(), AuthError> {
        {
            let mut session = self.session.write().await;
            session.user_id = None;
            session.clear();
        }
        self.current_user.write().await.take();
        self.persist().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::user::{InMemoryUserProvider, PasswordHash, User};
    use tokio::time::Duration as TokioDuration;

    #[tokio::test]
    async fn session_guard_attempt_and_logout() {
        let provider = InMemoryUserProvider::with_users(vec![User {
            id: 1,
            email: "alice@example.com".into(),
            name: "Alice".into(),
            password_hash: PasswordHash::hash("secret").unwrap(),
            is_active: true,
            email_verified_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }]);
        let provider = Arc::new(provider);
        let store = Arc::new(InMemorySessionStore::new(TokioDuration::from_secs(3600)));
        let guard = SessionGuard::new(provider.clone(), store.clone(), "session-1")
            .await
            .expect("guard");

        assert!(!guard.check().await);

        let user = guard
            .attempt(Credentials {
                email: "alice@example.com".into(),
                password: "secret".into(),
                remember_me: false,
            })
            .await
            .expect("login succeeds");
        assert_eq!(user.email, "alice@example.com");
        assert!(guard.check().await);
        assert!(guard.user().await.is_some());

        guard.logout().await.expect("logout works");
        assert!(!guard.check().await);
        assert!(guard.user().await.is_none());
    }

    #[tokio::test]
    async fn session_guard_invalid_credentials() {
        let provider = InMemoryUserProvider::with_users(vec![]);
        let store = Arc::new(InMemorySessionStore::new(TokioDuration::from_secs(3600)));
        let guard = SessionGuard::new(Arc::new(provider), store, "session-2")
            .await
            .expect("guard");

        let result = guard
            .attempt(Credentials {
                email: "nobody@example.com".into(),
                password: "secret".into(),
                remember_me: false,
            })
            .await;
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }
}
