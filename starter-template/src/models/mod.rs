pub mod post;
pub mod user;

pub use post::{ActiveModel as PostActiveModel, Entity as Post, Model as PostModel};
pub use user::{ActiveModel as UserActiveModel, Column as UserColumn, Entity as User, Model as UserModel};
