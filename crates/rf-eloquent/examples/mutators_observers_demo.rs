//! Complete demonstration of Eloquent mutators, accessors, and observers
//!
//! This example shows:
//! - Attribute accessors (get computed values)
//! - Attribute mutators (transform data on set)
//! - Model observers (react to lifecycle events)
//! - Complete CRUD operations with events

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rf_eloquent::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User model with accessors and mutators
#[derive(Clone, Debug, Serialize, Deserialize)]
struct User {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Virtual/computed attributes (not in database)
    #[serde(skip)]
    pub _computed: HashMap<String, AttributeValue>,
}

/// Implement accessors for computed attributes
impl HasAccessors for User {
    fn get_attribute(&self, key: &str) -> Option<AttributeValue> {
        match key {
            "full_name" => Some(AttributeValue::String(self.get_full_name())),
            "initials" => Some(AttributeValue::String(self.get_initials())),
            "display_name" => Some(AttributeValue::String(self.get_display_name())),
            _ => None,
        }
    }

    fn has_accessor(&self, key: &str) -> bool {
        matches!(key, "full_name" | "initials" | "display_name")
    }
}

impl User {
    /// Accessor: Get full name
    pub fn get_full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    /// Accessor: Get initials
    pub fn get_initials(&self) -> String {
        let first = self.first_name.chars().next().unwrap_or(' ');
        let last = self.last_name.chars().next().unwrap_or(' ');
        format!("{}{}", first, last).to_uppercase()
    }

    /// Accessor: Get display name
    pub fn get_display_name(&self) -> String {
        if !self.email.is_empty() {
            format!("{} <{}>", self.get_full_name(), self.email)
        } else {
            self.get_full_name()
        }
    }
}

/// Implement mutators for data transformation
impl HasMutators for User {
    fn set_attribute(&mut self, key: &str, value: AttributeValue) -> AttributeResult<()> {
        match key {
            "password" => self.set_password(value.as_string()?),
            "email" => self.set_email(value.as_string()?),
            "full_name" => self.set_full_name(value.as_string()?),
            _ => Ok(()),
        }
    }

    fn has_mutator(&self, key: &str) -> bool {
        matches!(key, "password" | "email" | "full_name")
    }
}

impl User {
    /// Mutator: Set password (automatically hashes)
    pub fn set_password(&mut self, password: String) -> AttributeResult<()> {
        if password.len() < 8 {
            return Err(AttributeError::ValidationError(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        // In production, use bcrypt or argon2
        self.password_hash = format!("$hashed${}", password);
        Ok(())
    }

    /// Mutator: Set email (automatically lowercases and trims)
    pub fn set_email(&mut self, email: String) -> AttributeResult<()> {
        let cleaned = email.trim().to_lowercase();

        if !cleaned.contains('@') {
            return Err(AttributeError::ValidationError(
                "Invalid email format".to_string(),
            ));
        }

        self.email = cleaned;
        Ok(())
    }

    /// Mutator: Set full name (automatically splits into first/last)
    pub fn set_full_name(&mut self, full_name: String) -> AttributeResult<()> {
        let parts: Vec<&str> = full_name.split_whitespace().collect();

        match parts.len() {
            0 => Err(AttributeError::ValidationError("Name cannot be empty".to_string())),
            1 => {
                self.first_name = parts[0].to_string();
                self.last_name = String::new();
                Ok(())
            }
            _ => {
                self.first_name = parts[0].to_string();
                self.last_name = parts[1..].join(" ");
                Ok(())
            }
        }
    }
}

/// Implement model events (observers)
#[async_trait]
impl ModelEvents for User {
    async fn creating(&mut self) -> EventResult {
        println!("🔔 Event: Creating user...");

        // Auto-set timestamps
        let now = Utc::now();
        self.created_at = now;
        self.updated_at = now;

        // Validate email
        if self.email.is_empty() {
            return Err(EventError::ValidationFailed("Email is required".to_string()));
        }

        // Validate password
        if self.password_hash.is_empty() {
            return Err(EventError::ValidationFailed("Password is required".to_string()));
        }

        println!("✅ Validation passed");
        Ok(())
    }

    async fn created(&self) -> EventResult {
        println!("🔔 Event: User created!");
        println!("   Welcome, {}!", self.get_full_name());

        // In production: Send welcome email, create profile, log activity, etc.
        send_welcome_email(&self.email).await?;
        create_default_profile(self.id).await?;
        log_user_registration(self.id).await?;

        Ok(())
    }

    async fn updating(&mut self) -> EventResult {
        println!("🔔 Event: Updating user...");

        // Update timestamp
        self.updated_at = Utc::now();

        // Validate changes
        if self.email.is_empty() {
            return Err(EventError::ValidationFailed("Email cannot be empty".to_string()));
        }

        Ok(())
    }

    async fn updated(&self) -> EventResult {
        println!("🔔 Event: User updated!");

        // In production: Invalidate cache, update indexes, send notifications
        invalidate_user_cache(self.id).await?;
        update_search_index(self.id).await?;

        Ok(())
    }

    async fn deleting(&mut self) -> EventResult {
        println!("🔔 Event: Deleting user...");

        // Check for dependencies
        if has_active_orders(self.id).await? {
            return Err(EventError::ValidationFailed(
                "Cannot delete user with active orders".to_string(),
            ));
        }

        Ok(())
    }

    async fn deleted(&self) -> EventResult {
        println!("🔔 Event: User deleted!");

        // In production: Clean up related data, remove from search, log
        delete_user_data(self.id).await?;
        remove_from_search(self.id).await?;
        log_user_deletion(self.id).await?;

        Ok(())
    }

    async fn saving(&mut self) -> EventResult {
        println!("🔔 Event: Saving user (before create/update)...");

        // Normalize data
        self.email = self.email.trim().to_lowercase();
        self.first_name = capitalize(&self.first_name);
        self.last_name = capitalize(&self.last_name);

        Ok(())
    }

    async fn saved(&self) -> EventResult {
        println!("🔔 Event: User saved (after create/update)!");
        Ok(())
    }
}

// Mock async functions (would be real database/service calls in production)

async fn send_welcome_email(email: &str) -> EventResult {
    println!("   📧 Sending welcome email to {}", email);
    Ok(())
}

async fn create_default_profile(user_id: i64) -> EventResult {
    println!("   👤 Creating default profile for user {}", user_id);
    Ok(())
}

async fn log_user_registration(user_id: i64) -> EventResult {
    println!("   📝 Logging registration for user {}", user_id);
    Ok(())
}

async fn invalidate_user_cache(user_id: i64) -> EventResult {
    println!("   🗑️  Invalidating cache for user {}", user_id);
    Ok(())
}

async fn update_search_index(user_id: i64) -> EventResult {
    println!("   🔍 Updating search index for user {}", user_id);
    Ok(())
}

async fn has_active_orders(user_id: i64) -> Result<bool, EventError> {
    println!("   🔍 Checking for active orders for user {}", user_id);
    Ok(false)
}

async fn delete_user_data(user_id: i64) -> EventResult {
    println!("   🗑️  Deleting user data for user {}", user_id);
    Ok(())
}

async fn remove_from_search(user_id: i64) -> EventResult {
    println!("   🔍 Removing user {} from search index", user_id);
    Ok(())
}

async fn log_user_deletion(user_id: i64) -> EventResult {
    println!("   📝 Logging deletion for user {}", user_id);
    Ok(())
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Eloquent Mutators & Observers Demo\n");
    println!("=" .repeat(60));

    // Create a new user
    println!("\n📝 Creating new user...\n");

    let mut user = User {
        id: 1,
        first_name: "john".to_string(), // Will be capitalized by saving event
        last_name: "doe".to_string(),   // Will be capitalized by saving event
        email: "  JOHN@EXAMPLE.COM  ".to_string(), // Will be cleaned by mutator/event
        password_hash: String::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        _computed: HashMap::new(),
    };

    // Use mutator to set password (automatically hashes)
    println!("🔐 Setting password with mutator...");
    user.set_password("secret123".to_string())?;
    println!("   Password hash: {}", user.password_hash);

    // Use mutator to clean email
    println!("\n📧 Cleaning email with mutator...");
    user.set_email("  JOHN@EXAMPLE.COM  ".to_string())?;
    println!("   Cleaned email: {}", user.email);

    // Trigger events
    println!("\n🎭 Triggering model events...\n");

    // Saving event (before create/update)
    user.saving().await?;
    println!("   First name after saving: {}", user.first_name);
    println!("   Last name after saving: {}", user.last_name);

    // Creating event
    user.creating().await?;

    // Created event
    user.created().await?;

    // Use accessors to get computed values
    println!("\n📊 Using accessors for computed values:");
    println!("   Full name: {}", user.get_full_name());
    println!("   Initials: {}", user.get_initials());
    println!("   Display name: {}", user.get_display_name());

    // Get attribute via trait
    if let Some(full_name) = user.get_attribute("full_name") {
        println!("   Full name via trait: {:?}", full_name);
    }

    // Update user
    println!("\n🔄 Updating user...\n");
    user.first_name = "Jane".to_string();

    user.updating().await?;
    user.updated().await?;

    // Delete user
    println!("\n🗑️  Deleting user...\n");
    user.deleting().await?;
    user.deleted().await?;

    // Test mutator validation
    println!("\n✅ Testing mutator validation...");

    let mut test_user = user.clone();

    // Should fail: password too short
    match test_user.set_password("short".to_string()) {
        Ok(_) => println!("   ❌ Should have failed: password too short"),
        Err(e) => println!("   ✅ Correctly rejected: {}", e),
    }

    // Should fail: invalid email
    match test_user.set_email("not-an-email".to_string()) {
        Ok(_) => println!("   ❌ Should have failed: invalid email"),
        Err(e) => println!("   ✅ Correctly rejected: {}", e),
    }

    println!("\n" + &"=".repeat(60));
    println!("✅ Demo completed successfully!");

    Ok(())
}
