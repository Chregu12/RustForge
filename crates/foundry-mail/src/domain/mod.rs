pub mod address;
pub mod attachment;
pub mod content;
pub mod envelope;
pub mod message;

pub use address::{Address, AddressList};
pub use attachment::{Attachment, AttachmentBuilder};
pub use content::{Content, ContentType};
pub use envelope::Envelope;
pub use message::{Message, MessageBuilder};
