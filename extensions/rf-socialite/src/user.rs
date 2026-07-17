//! User data structures

use serde::{Deserialize, Serialize};

/// User information from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID from provider
    pub id: String,

    /// User's display name
    pub name: String,

    /// User's email address
    pub email: Option<String>,

    /// URL to user's avatar
    pub avatar: Option<String>,

    /// OAuth provider name
    pub provider: String,

    /// Access token
    pub token: String,

    /// Raw user data from provider
    #[serde(flatten)]
    pub raw: UserData,
}

/// Raw user data from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_email: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: "123".to_string(),
            name: "John Doe".to_string(),
            email: Some("john@example.com".to_string()),
            avatar: Some("https://example.com/avatar.jpg".to_string()),
            provider: "github".to_string(),
            token: "token123".to_string(),
            raw: UserData {
                id: Some("123".to_string()),
                name: Some("John Doe".to_string()),
                email: Some("john@example.com".to_string()),
                login: None,
                avatar_url: Some("https://example.com/avatar.jpg".to_string()),
                picture: None,
                given_name: None,
                family_name: None,
                locale: None,
                verified_email: None,
            },
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("John Doe"));
        assert!(json.contains("john@example.com"));
    }

    #[test]
    fn test_user_data_deserialization() {
        let json = r#"{
            "id": "123",
            "name": "John Doe",
            "email": "john@example.com"
        }"#;

        let data: UserData = serde_json::from_str(json).unwrap();
        assert_eq!(data.id, Some("123".to_string()));
        assert_eq!(data.name, Some("John Doe".to_string()));
        assert_eq!(data.email, Some("john@example.com".to_string()));
    }
}
