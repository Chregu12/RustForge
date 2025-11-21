//! Integration tests for the Validate derive macro

use rf_validation_derive::Validate;
use validator::Validate as ValidatorValidate;

#[test]
fn test_simple_string_validation() {
    #[derive(Validate)]
    struct CreateUser {
        #[validate(required, email)]
        email: String,

        #[validate(required, min = 8)]
        password: String,
    }

    // Valid case
    let user = CreateUser {
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
    };
    assert!(user.validate().is_ok());

    // Invalid email
    let user = CreateUser {
        email: "invalid".to_string(),
        password: "password123".to_string(),
    };
    assert!(user.validate().is_err());

    // Password too short
    let user = CreateUser {
        email: "test@example.com".to_string(),
        password: "short".to_string(),
    };
    assert!(user.validate().is_err());
}

#[test]
fn test_optional_fields() {
    #[derive(Validate)]
    struct UpdateUser {
        #[validate(email)]
        email: Option<String>,

        #[validate(url)]
        website: Option<String>,
    }

    // All None - should pass
    let user = UpdateUser {
        email: None,
        website: None,
    };
    assert!(user.validate().is_ok());

    // Valid values
    let user = UpdateUser {
        email: Some("test@example.com".to_string()),
        website: Some("https://example.com".to_string()),
    };
    assert!(user.validate().is_ok());

    // Invalid email
    let user = UpdateUser {
        email: Some("invalid".to_string()),
        website: None,
    };
    assert!(user.validate().is_err());

    // Invalid URL
    let user = UpdateUser {
        email: None,
        website: Some("not-a-url".to_string()),
    };
    assert!(user.validate().is_err());
}

#[test]
fn test_required_optional_field() {
    #[derive(Validate)]
    struct CreatePost {
        #[validate(required)]
        title: Option<String>,
    }

    // None - should fail
    let post = CreatePost { title: None };
    assert!(post.validate().is_err());

    // Some - should pass
    let post = CreatePost {
        title: Some("Hello".to_string()),
    };
    assert!(post.validate().is_ok());
}

#[test]
fn test_length_constraints() {
    #[derive(Validate)]
    struct Comment {
        #[validate(min = 10, max = 500)]
        content: String,
    }

    // Too short
    let comment = Comment {
        content: "Short".to_string(),
    };
    assert!(comment.validate().is_err());

    // Just right
    let comment = Comment {
        content: "This is a valid comment with enough characters".to_string(),
    };
    assert!(comment.validate().is_ok());

    // Too long
    let comment = Comment {
        content: "a".repeat(501),
    };
    assert!(comment.validate().is_err());
}

#[test]
fn test_string_patterns() {
    #[derive(Validate)]
    struct Slug {
        #[validate(starts_with = "post-")]
        slug: String,
    }

    let slug = Slug {
        slug: "post-hello-world".to_string(),
    };
    assert!(slug.validate().is_ok());

    let slug = Slug {
        slug: "hello-world".to_string(),
    };
    assert!(slug.validate().is_err());
}

#[test]
fn test_alpha_validation() {
    #[derive(Validate)]
    struct Name {
        #[validate(alpha)]
        first_name: String,

        #[validate(alpha_numeric)]
        username: String,
    }

    let name = Name {
        first_name: "John".to_string(),
        username: "john123".to_string(),
    };
    assert!(name.validate().is_ok());

    let name = Name {
        first_name: "John123".to_string(), // Invalid - contains numbers
        username: "john123".to_string(),
    };
    assert!(name.validate().is_err());

    let name = Name {
        first_name: "John".to_string(),
        username: "john-123".to_string(), // Invalid - contains hyphen
    };
    assert!(name.validate().is_err());
}

#[test]
fn test_case_validation() {
    #[derive(Validate)]
    struct CaseSensitive {
        #[validate(lowercase)]
        lowercase_field: String,

        #[validate(uppercase)]
        uppercase_field: String,
    }

    let data = CaseSensitive {
        lowercase_field: "hello".to_string(),
        uppercase_field: "WORLD".to_string(),
    };
    assert!(data.validate().is_ok());

    let data = CaseSensitive {
        lowercase_field: "Hello".to_string(), // Invalid - has uppercase
        uppercase_field: "WORLD".to_string(),
    };
    assert!(data.validate().is_err());

    let data = CaseSensitive {
        lowercase_field: "hello".to_string(),
        uppercase_field: "World".to_string(), // Invalid - has lowercase
    };
    assert!(data.validate().is_err());
}

#[test]
fn test_url_validation() {
    #[derive(Validate)]
    struct Link {
        #[validate(url)]
        url: String,
    }

    let link = Link {
        url: "https://example.com".to_string(),
    };
    assert!(link.validate().is_ok());

    let link = Link {
        url: "not-a-url".to_string(),
    };
    assert!(link.validate().is_err());
}

#[test]
fn test_ip_validation() {
    #[derive(Validate)]
    struct Server {
        #[validate(ip)]
        address: String,
    }

    // IPv4
    let server = Server {
        address: "192.168.1.1".to_string(),
    };
    assert!(server.validate().is_ok());

    // IPv6
    let server = Server {
        address: "::1".to_string(),
    };
    assert!(server.validate().is_ok());

    // Invalid
    let server = Server {
        address: "not-an-ip".to_string(),
    };
    assert!(server.validate().is_err());
}

#[test]
fn test_uuid_validation() {
    #[derive(Validate)]
    struct Entity {
        #[validate(uuid)]
        id: String,
    }

    let entity = Entity {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    };
    assert!(entity.validate().is_ok());

    let entity = Entity {
        id: "not-a-uuid".to_string(),
    };
    assert!(entity.validate().is_err());
}

#[test]
fn test_regex_validation() {
    #[derive(Validate)]
    struct PhoneNumber {
        #[validate(regex = r"^\+?[1-9]\d{1,14}$")]
        phone: String,
    }

    let phone = PhoneNumber {
        phone: "+1234567890".to_string(),
    };
    assert!(phone.validate().is_ok());

    let phone = PhoneNumber {
        phone: "invalid".to_string(),
    };
    assert!(phone.validate().is_err());
}

#[test]
fn test_multiple_rules() {
    #[derive(Validate)]
    struct CreatePost {
        #[validate(required, min = 3, max = 255)]
        title: String,

        #[validate(required, email, max = 255)]
        author_email: String,
    }

    // Valid
    let post = CreatePost {
        title: "Hello World".to_string(),
        author_email: "author@example.com".to_string(),
    };
    assert!(post.validate().is_ok());

    // Title too short
    let post = CreatePost {
        title: "Hi".to_string(),
        author_email: "author@example.com".to_string(),
    };
    assert!(post.validate().is_err());

    // Invalid email
    let post = CreatePost {
        title: "Hello World".to_string(),
        author_email: "invalid".to_string(),
    };
    assert!(post.validate().is_err());
}

#[test]
fn test_nested_validation() {
    #[derive(Validate)]
    struct Tag {
        #[validate(required, min = 2)]
        name: String,
    }

    #[derive(Validate)]
    struct Post {
        #[validate(required)]
        title: String,

        #[validate]
        tags: Vec<Tag>,
    }

    // Valid
    let post = Post {
        title: "Hello".to_string(),
        tags: vec![
            Tag {
                name: "rust".to_string(),
            },
            Tag {
                name: "web".to_string(),
            },
        ],
    };
    assert!(post.validate().is_ok());

    // Invalid nested tag
    let post = Post {
        title: "Hello".to_string(),
        tags: vec![Tag {
            name: "x".to_string(), // Too short
        }],
    };
    assert!(post.validate().is_err());
}

#[test]
fn test_nullable_marker() {
    #[derive(Validate)]
    struct UpdateData {
        #[validate(nullable, email)]
        email: Option<String>,
    }

    let data = UpdateData { email: None };
    assert!(data.validate().is_ok());

    let data = UpdateData {
        email: Some("test@example.com".to_string()),
    };
    assert!(data.validate().is_ok());

    let data = UpdateData {
        email: Some("invalid".to_string()),
    };
    assert!(data.validate().is_err());
}

#[test]
fn test_empty_string_as_required() {
    #[derive(Validate)]
    struct Form {
        #[validate(required)]
        field: String,
    }

    let form = Form {
        field: String::new(),
    };
    assert!(form.validate().is_err());

    let form = Form {
        field: "not empty".to_string(),
    };
    assert!(form.validate().is_ok());
}
