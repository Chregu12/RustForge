//! make:policy command

use clap::Parser;
use std::fs;
use std::path::Path;

#[derive(Debug, Parser)]
#[command(name = "make:policy", about = "Create a new authorization policy")]
pub struct MakePolicyCommand {
    /// Policy name (e.g., PostPolicy)
    pub name: String,

    /// Model name for the policy (optional)
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// Create policy without model-specific methods
    #[arg(long)]
    pub plain: bool,
}

impl MakePolicyCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        let policy_name = if self.name.ends_with("Policy") {
            self.name.clone()
        } else {
            format!("{}Policy", self.name)
        };

        println!("Creating policy: {}", policy_name);

        let content = if self.plain {
            self.generate_plain_policy(&policy_name)
        } else {
            self.generate_model_policy(&policy_name)
        };

        let filename = format!(
            "app/policies/{}.rs",
            self.to_snake_case(&policy_name)
        );

        fs::create_dir_all("app/policies")?;
        fs::write(&filename, content)?;

        println!("✓ Policy created: {}", filename);

        if !Path::new("app/policies/mod.rs").exists() {
            let mod_content = format!("pub mod {};\n", self.to_snake_case(&policy_name));
            fs::write("app/policies/mod.rs", mod_content)?;
            println!("✓ Created policies module: app/policies/mod.rs");
        } else {
            println!("⚠ Don't forget to add 'pub mod {};' to app/policies/mod.rs",
                     self.to_snake_case(&policy_name));
        }

        Ok(())
    }

    fn generate_plain_policy(&self, policy_name: &str) -> String {
        format!(
            r#"//! {} - Authorization policy
//!
//! This policy defines authorization rules for custom logic.

use foundry_domain::User;

/// {} handles authorization for custom operations
pub struct {} {{}}

impl {} {{
    /// Create a new policy instance
    pub fn new() -> Self {{
        Self {{}}
    }}

    /// Check if user can perform a custom action
    pub fn can_perform_action(&self, user: &User) -> bool {{
        // TODO: Implement your authorization logic
        true
    }}

    /// Check if user has admin privileges
    pub fn is_admin(&self, user: &User) -> bool {{
        // TODO: Implement admin check
        user.is_admin.unwrap_or(false)
    }}
}}

impl Default for {} {{
    fn default() -> Self {{
        Self::new()
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_policy_creation() {{
        let policy = {}::new();
        // Add your tests here
    }}
}}
"#,
            policy_name,
            policy_name,
            policy_name,
            policy_name,
            policy_name,
            policy_name
        )
    }

    fn generate_model_policy(&self, policy_name: &str) -> String {
        let model_name = self.model.as_ref()
            .map(|m| m.clone())
            .unwrap_or_else(|| {
                policy_name
                    .strip_suffix("Policy")
                    .unwrap_or(policy_name)
                    .to_string()
            });

        format!(
            r#"//! {} - Authorization policy for {}
//!
//! This policy defines authorization rules for {} resources.
//! Inspired by Laravel's Policy system.

use foundry_domain::{{User, {}}};

/// {} handles authorization for {} resources
pub struct {} {{}}

impl {} {{
    /// Create a new policy instance
    pub fn new() -> Self {{
        Self {{}}
    }}

    /// Determine if the user can view any {} resources
    pub fn view_any(&self, user: &User) -> bool {{
        // All authenticated users can view resources
        true
    }}

    /// Determine if the user can view the {} resource
    pub fn view(&self, user: &User, resource: &{}) -> bool {{
        // All authenticated users can view a specific resource
        true
    }}

    /// Determine if the user can create {} resources
    pub fn create(&self, user: &User) -> bool {{
        // Example: Only verified users can create
        user.email_verified_at.is_some()
    }}

    /// Determine if the user can update the {} resource
    pub fn update(&self, user: &User, resource: &{}) -> bool {{
        // Example: Only the owner can update
        match &resource.user_id {{
            Some(user_id) => *user_id == user.id,
            None => false,
        }}
    }}

    /// Determine if the user can delete the {} resource
    pub fn delete(&self, user: &User, resource: &{}) -> bool {{
        // Example: Only the owner can delete
        match &resource.user_id {{
            Some(user_id) => *user_id == user.id,
            None => false,
        }}
    }}

    /// Determine if the user can restore the {} resource
    pub fn restore(&self, user: &User, resource: &{}) -> bool {{
        // Example: Only the owner can restore
        match &resource.user_id {{
            Some(user_id) => *user_id == user.id,
            None => false,
        }}
    }}

    /// Determine if the user can permanently delete the {} resource
    pub fn force_delete(&self, user: &User, resource: &{}) -> bool {{
        // Example: Only admins can force delete
        user.is_admin.unwrap_or(false)
    }}
}}

impl Default for {} {{
    fn default() -> Self {{
        Self::new()
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;
    use uuid::Uuid;

    fn create_test_user(id: Uuid, is_admin: bool) -> User {{
        User {{
            id,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            email_verified_at: Some(chrono::Utc::now()),
            password: "hashed".to_string(),
            remember_token: None,
            is_admin: Some(is_admin),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }}
    }}

    fn create_test_resource(user_id: Uuid) -> {} {{
        {} {{
            id: Uuid::new_v4(),
            user_id: Some(user_id),
            // Add other fields as needed
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }}
    }}

    #[test]
    fn test_view_any() {{
        let policy = {}::new();
        let user = create_test_user(Uuid::new_v4(), false);
        assert!(policy.view_any(&user));
    }}

    #[test]
    fn test_view() {{
        let policy = {}::new();
        let user_id = Uuid::new_v4();
        let user = create_test_user(user_id, false);
        let resource = create_test_resource(user_id);
        assert!(policy.view(&user, &resource));
    }}

    #[test]
    fn test_create_requires_verification() {{
        let policy = {}::new();
        let mut user = create_test_user(Uuid::new_v4(), false);

        // Verified user can create
        assert!(policy.create(&user));

        // Unverified user cannot create
        user.email_verified_at = None;
        assert!(!policy.create(&user));
    }}

    #[test]
    fn test_update_owner_only() {{
        let policy = {}::new();
        let owner_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();

        let owner = create_test_user(owner_id, false);
        let other = create_test_user(other_id, false);
        let resource = create_test_resource(owner_id);

        assert!(policy.update(&owner, &resource));
        assert!(!policy.update(&other, &resource));
    }}

    #[test]
    fn test_delete_owner_only() {{
        let policy = {}::new();
        let owner_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();

        let owner = create_test_user(owner_id, false);
        let other = create_test_user(other_id, false);
        let resource = create_test_resource(owner_id);

        assert!(policy.delete(&owner, &resource));
        assert!(!policy.delete(&other, &resource));
    }}

    #[test]
    fn test_force_delete_admin_only() {{
        let policy = {}::new();
        let admin = create_test_user(Uuid::new_v4(), true);
        let user = create_test_user(Uuid::new_v4(), false);
        let resource = create_test_resource(admin.id);

        assert!(policy.force_delete(&admin, &resource));
        assert!(!policy.force_delete(&user, &resource));
    }}
}}
"#,
            policy_name,
            model_name,
            model_name,
            model_name,
            policy_name,
            model_name,
            policy_name,
            policy_name,
            model_name,
            model_name,
            model_name,
            model_name,
            model_name,
            model_name,
            model_name,
            model_name,
            model_name,
            model_name,
            model_name,
            model_name,
            policy_name,
            model_name,
            model_name,
            policy_name,
            policy_name,
            policy_name,
            policy_name,
            policy_name,
            policy_name
        )
    }

    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(ch.to_lowercase().next().unwrap());
            } else {
                result.push(ch);
            }
        }
        result
    }
}
