//! Database Models
//!
//! Models represent your database tables and provide an elegant API for
//! querying and manipulating data.
//!
//! Example:
//!
//! ```ignore
//! use rustforge::*;
//!
//! #[model]
//! pub struct User {
//!     pub name: String,
//!     pub email: String,
//!     #[hidden]
//!     pub password: String,
//! }
//! ```

// Add your models here.
