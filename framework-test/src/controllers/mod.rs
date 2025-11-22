// Controllers module - REAL implementations demonstrating framework features
pub mod user_controller;
pub mod post_controller;
pub mod auth_controller;
// pub mod product_controller;
// pub mod order_controller;
// pub mod search_controller;

// Re-exports for convenience
pub use user_controller::*;
pub use post_controller::*;
pub use auth_controller::*;
