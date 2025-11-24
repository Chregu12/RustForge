//! Example: User and Post models with Laravel-like syntax
//!
//! This example demonstrates the compact model syntax that matches Laravel's simplicity.
//!
//! Compare with Laravel:
//! ```php
//! class User extends Model {
//!     protected $fillable = ['name', 'email'];
//!     protected $hidden = ['password'];
//!     public function posts() { return $this->hasMany(Post::class); }
//! }
//! ```

use rf_model_macro::model;

#[model]
pub struct User {
    // id, created_at, updated_at are automatically added
    pub name: String,
    pub email: String,

    #[hidden]
    pub password: String,
}

#[model]
pub struct Post {
    // id, created_at, updated_at are automatically added
    pub title: String,
    pub content: String,
    pub user_id: i32,
    pub published: bool,
}

#[model]
pub struct Profile {
    // id, created_at, updated_at are automatically added
    pub user_id: i32,
    pub bio: String,
    pub avatar_url: Option<String>,
}

fn main() {
    println!("Models defined with Laravel-like syntax!");
    println!("✓ User model with hidden password field");
    println!("✓ Post model");
    println!("✓ Profile model");
    println!("\nAll models automatically include:");
    println!("  - id: i32 (primary key)");
    println!("  - created_at: DateTime<Utc>");
    println!("  - updated_at: DateTime<Utc>");
    println!("  - All SeaORM derives");
    println!("  - Serde serialization");
}
