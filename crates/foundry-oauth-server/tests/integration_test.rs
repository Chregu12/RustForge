//! Integration Tests for OAuth2 Server
//!
//! Comprehensive end-to-end tests for OAuth2 flows

use foundry_oauth_server::*;
use uuid::Uuid;

/// Helper function to create test server
fn create_test_server() -> OAuth2Server<clients::InMemoryClientRepository> {
    let config = OAuth2Config::default();
    let repo = clients::InMemoryClientRepository::new();
    OAuth2Server::new(config, repo)
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_authorization_code_flow() {
        // Setup
        let server = create_test_server();

        // Create client
        let client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        let client_id = client.id;
        let secret = client.secret.clone().unwrap();

        server.client_repository().store(client.clone()).await.unwrap();

        // Request authorization code
        let user_id = Uuid::new_v4();
        let auth_code = server.create_authorization_code(
            &client,
            user_id,
            "http://localhost/callback".to_string(),
            vec!["users:read".to_string()],
            None,
            None,
        ).unwrap();

        // Verify authorization code properties
        assert_eq!(auth_code.client_id, client_id);
        assert_eq!(auth_code.user_id, user_id);
        assert_eq!(auth_code.scopes, vec!["users:read".to_string()]);
        assert!(!auth_code.revoked);
        assert!(!auth_code.code.is_empty());

        // Exchange code for tokens
        let tokens = server.exchange_authorization_code(
            &client,
            &auth_code,
            None,
        ).await.unwrap();

        // Verify token response
        assert!(!tokens.access_token.is_empty());
        assert!(tokens.refresh_token.is_some());
        assert_eq!(tokens.token_type, "Bearer");
        assert!(tokens.expires_in > 0);

        // Verify access token is valid
        let claims = server.validate_token(&tokens.access_token).unwrap();
        assert_eq!(claims.user_id, Some(user_id.to_string()));
        assert_eq!(claims.client_id, client_id.to_string());
        assert_eq!(claims.scopes, vec!["users:read".to_string()]);

        // Verify client can authenticate with credentials
        let validated = server.validate_client(client_id, Some(&secret)).await.unwrap();
        assert_eq!(validated.name, "Test App");

        // Use refresh token (Note: In a real scenario, we'd need to store and retrieve tokens)
        // For this test, we're verifying the complete flow works end-to-end
    }

    #[tokio::test]
    async fn test_pkce_flow_public_client() {
        // Setup
        let server = create_test_server();

        // Create public client (no secret)
        let client = Client::public(
            "Public App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        let client_id = client.id;

        server.client_repository().store(client.clone()).await.unwrap();

        // Generate PKCE challenge
        let code_verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let code_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        let code_challenge_method = "S256";

        let user_id = Uuid::new_v4();

        // Request authorization with PKCE
        let auth_code = server.create_authorization_code(
            &client,
            user_id,
            "http://localhost/callback".to_string(),
            vec!["users:read".to_string()],
            Some(code_challenge.to_string()),
            Some(code_challenge_method.to_string()),
        ).unwrap();

        // Verify PKCE parameters stored
        assert_eq!(auth_code.code_challenge, Some(code_challenge.to_string()));
        assert_eq!(auth_code.code_challenge_method, Some(code_challenge_method.to_string()));

        // Exchange code with verifier
        let tokens = server.exchange_authorization_code(
            &client,
            &auth_code,
            Some(code_verifier.to_string()),
        ).await.unwrap();

        // Verify token issued
        assert!(!tokens.access_token.is_empty());
        assert!(tokens.refresh_token.is_some());

        // Verify token is valid
        let claims = server.validate_token(&tokens.access_token).unwrap();
        assert_eq!(claims.user_id, Some(user_id.to_string()));

        // Verify public client cannot authenticate with secret
        let result = server.validate_client(client_id, Some("any_secret")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pkce_required_for_public_clients() {
        // Setup
        let server = create_test_server();

        // Create public client
        let client = Client::public(
            "Public App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );

        server.client_repository().store(client.clone()).await.unwrap();

        let user_id = Uuid::new_v4();

        // Attempt to create authorization code without PKCE (should fail)
        let result = server.create_authorization_code(
            &client,
            user_id,
            "http://localhost/callback".to_string(),
            vec!["users:read".to_string()],
            None, // No PKCE
            None,
        );

        assert!(result.is_err());
        if let Err(OAuth2Error::InvalidRequest(msg)) = result {
            assert!(msg.contains("PKCE required"));
        } else {
            panic!("Expected InvalidRequest error");
        }
    }

    #[tokio::test]
    async fn test_client_credentials_flow() {
        // Setup
        let server = create_test_server();

        // Create confidential client with client_credentials grant
        let mut client = Client::new(
            "Service App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        client.grants.push("client_credentials".to_string());

        server.client_repository().store(client.clone()).await.unwrap();

        // Request token with client credentials
        let tokens = server.issue_client_credentials_token(
            &client,
            vec!["api:read".to_string(), "api:write".to_string()],
        ).await.unwrap();

        // Verify token properties
        assert!(!tokens.access_token.is_empty());
        assert!(tokens.refresh_token.is_none()); // No refresh token for client credentials
        assert_eq!(tokens.token_type, "Bearer");

        // Verify token has correct scopes
        let claims = server.validate_token(&tokens.access_token).unwrap();
        assert_eq!(claims.scopes, vec!["api:read".to_string(), "api:write".to_string()]);
        assert!(claims.user_id.is_none()); // No user for client credentials
        assert_eq!(claims.client_id, client.id.to_string());

        // Verify client with wildcard scope can request any valid scope
        let result = server.issue_client_credentials_token(
            &client,
            vec!["users:read".to_string()],
        ).await;
        // Should succeed because client has wildcard scope "*"
        assert!(result.is_ok());

        // Verify client cannot request invalid/non-existent scopes
        let result = server.issue_client_credentials_token(
            &client,
            vec!["invalid:scope".to_string()],
        ).await;
        // Should fail because scope doesn't exist
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_token_introspection() {
        // Setup
        let server = create_test_server();

        // Create client and tokens
        let mut client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        client.grants.push("client_credentials".to_string());

        server.client_repository().store(client.clone()).await.unwrap();

        // Create valid token
        let tokens = server.issue_client_credentials_token(
            &client,
            vec!["users:read".to_string()],
        ).await.unwrap();

        // Introspect valid token
        let introspection = server.introspect_token(&tokens.access_token);
        assert!(introspection.active);
        assert_eq!(introspection.scope, Some("users:read".to_string()));
        assert_eq!(introspection.client_id, Some(client.id.to_string()));
        assert_eq!(introspection.token_type, Some("Bearer".to_string()));
        assert!(introspection.exp.is_some());
        assert!(introspection.iat.is_some());

        // Introspect invalid token
        let invalid_introspection = server.introspect_token("invalid_token");
        assert!(!invalid_introspection.active);
        assert!(invalid_introspection.scope.is_none());
        assert!(invalid_introspection.client_id.is_none());
    }

    #[tokio::test]
    async fn test_expired_token_introspection() {
        // Setup
        let mut config = OAuth2Config::default();
        config.access_token_lifetime = -3600; // Expired 1 hour ago

        let repo = clients::InMemoryClientRepository::new();
        let server = OAuth2Server::new(config, repo);

        // Create client and expired token
        let mut client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        client.grants.push("client_credentials".to_string());

        server.client_repository().store(client.clone()).await.unwrap();

        let tokens = server.issue_client_credentials_token(
            &client,
            vec!["users:read".to_string()],
        ).await.unwrap();

        // Introspect expired token
        let introspection = server.introspect_token(&tokens.access_token);
        assert!(!introspection.active); // Expired token should be inactive
    }

    #[tokio::test]
    async fn test_token_revocation_via_client() {
        // Setup
        let server = create_test_server();

        // Create client
        let client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        let client_id = client.id;

        server.client_repository().store(client.clone()).await.unwrap();

        // Revoke client
        server.client_repository().revoke(client_id).await.unwrap();

        // Verify client is revoked
        let revoked_client = server.client_repository().find(client_id).await.unwrap().unwrap();
        assert!(revoked_client.revoked);

        // Verify revoked client cannot be validated
        let result = server.validate_client(client_id, None).await;
        assert!(result.is_err());
        if let Err(OAuth2Error::InvalidClient(msg)) = result {
            assert!(msg.contains("revoked"));
        } else {
            panic!("Expected InvalidClient error");
        }
    }

    #[tokio::test]
    async fn test_personal_access_tokens() {
        // Setup
        let server = create_test_server();

        // Create personal access token
        let user_id = Uuid::new_v4();
        let pat = server.create_personal_access_token(
            user_id,
            "My API Token".to_string(),
            vec!["users:read".to_string(), "users:write".to_string()],
        ).unwrap();

        // Verify PAT properties
        assert_eq!(pat.user_id, user_id);
        assert_eq!(pat.name, "My API Token");
        assert_eq!(pat.scopes, vec!["users:read".to_string(), "users:write".to_string()]);
        assert!(!pat.revoked);
        assert!(pat.expires_at.is_none()); // Never expires by default
        assert!(!pat.token.is_empty());

        // Verify can create multiple PATs for same user
        let pat2 = server.create_personal_access_token(
            user_id,
            "Another Token".to_string(),
            vec!["admin".to_string()],
        ).unwrap();

        assert_ne!(pat.id, pat2.id);
        assert_ne!(pat.token, pat2.token);
        assert_eq!(pat2.name, "Another Token");
    }

    #[tokio::test]
    async fn test_scope_validation() {
        // Setup
        let server = create_test_server();

        // Create client with limited scopes
        let client = Client {
            id: Uuid::new_v4(),
            name: "Limited App".to_string(),
            secret: Some("secret".to_string()),
            redirect_uris: vec!["http://localhost/callback".to_string()],
            grants: vec!["authorization_code".to_string()],
            scopes: vec!["users:read".to_string()], // Only users:read allowed
            revoked: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        server.client_repository().store(client.clone()).await.unwrap();

        // Valid scope request
        let valid_scopes = server.validate_scopes(&client, &["users:read".to_string()]);
        assert!(valid_scopes.is_ok());

        // Invalid scope request
        let invalid_scopes = server.validate_scopes(&client, &["admin:write".to_string()]);
        assert!(invalid_scopes.is_err());
    }

    #[tokio::test]
    async fn test_redirect_uri_validation() {
        // Setup
        let server = create_test_server();

        // Create client with specific redirect URIs
        let client = Client::new(
            "Test App".to_string(),
            vec![
                "http://localhost:3000/callback".to_string(),
                "https://app.example.com/oauth/callback".to_string(),
            ],
        );

        server.client_repository().store(client.clone()).await.unwrap();

        // Valid redirect URI
        let valid = server.validate_redirect_uri(&client, "http://localhost:3000/callback");
        assert!(valid.is_ok());

        // Invalid redirect URI
        let invalid = server.validate_redirect_uri(&client, "http://evil.com/callback");
        assert!(invalid.is_err());
    }

    #[tokio::test]
    async fn test_invalid_grant_type() {
        // Setup
        let server = create_test_server();

        // Create client without client_credentials grant
        let client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        // Default grants are: authorization_code, refresh_token

        server.client_repository().store(client.clone()).await.unwrap();

        // Attempt client_credentials flow (should fail)
        let result = server.issue_client_credentials_token(
            &client,
            vec!["read".to_string()],
        ).await;

        assert!(result.is_err());
        if let Err(OAuth2Error::UnauthorizedClient(msg)) = result {
            assert!(msg.contains("client_credentials"));
        } else {
            panic!("Expected UnauthorizedClient error");
        }
    }

    #[tokio::test]
    async fn test_authorization_code_exchange_validation() {
        // Setup
        let server = create_test_server();

        // Create two clients
        let client1 = Client::new(
            "App 1".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        let client2 = Client::new(
            "App 2".to_string(),
            vec!["http://localhost/callback".to_string()],
        );

        server.client_repository().store(client1.clone()).await.unwrap();
        server.client_repository().store(client2.clone()).await.unwrap();

        // Create authorization code for client1
        let user_id = Uuid::new_v4();
        let auth_code = server.create_authorization_code(
            &client1,
            user_id,
            "http://localhost/callback".to_string(),
            vec!["users:read".to_string()],
            None,
            None,
        ).unwrap();

        // Attempt to exchange code with wrong client (should fail)
        let result = server.exchange_authorization_code(
            &client2,
            &auth_code,
            None,
        ).await;

        assert!(result.is_err());
        if let Err(OAuth2Error::InvalidGrant(msg)) = result {
            assert!(msg.contains("different client"));
        } else {
            panic!("Expected InvalidGrant error");
        }
    }

    #[tokio::test]
    async fn test_password_grant_flow() {
        // Setup
        let server = create_test_server();

        // Create client with password grant
        let mut client = Client::new(
            "Password Client".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        client.grants.push("password".to_string());

        server.client_repository().store(client.clone()).await.unwrap();

        // Issue token via password grant
        let user_id = Uuid::new_v4();
        let tokens = server.issue_password_token(
            &client,
            user_id,
            vec!["users:read".to_string()],
        ).await.unwrap();

        // Verify token
        assert!(!tokens.access_token.is_empty());
        assert!(tokens.refresh_token.is_some());

        let claims = server.validate_token(&tokens.access_token).unwrap();
        assert_eq!(claims.user_id, Some(user_id.to_string()));
    }

    #[tokio::test]
    async fn test_client_builder() {
        // Test the ClientBuilder API
        let client = clients::ClientBuilder::new()
            .name("My Application".to_string())
            .redirect_uri("http://localhost:3000/callback".to_string())
            .redirect_uri("https://app.example.com/oauth".to_string())
            .grant("client_credentials".to_string())
            .scope("users:read".to_string())
            .scope("posts:write".to_string())
            .public()
            .build()
            .unwrap();

        assert_eq!(client.name, "My Application");
        assert_eq!(client.redirect_uris.len(), 2);
        assert!(client.grants.contains(&"client_credentials".to_string()));
        assert!(client.scopes.contains(&"users:read".to_string()));
        assert!(!client.is_confidential());
    }
}
