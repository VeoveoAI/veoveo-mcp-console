//! The server's public types. Consumers select `default-features = false,
//! features = ["contract"]`, which builds without a database or runtime dependency.

mod calls;
pub use calls::{Items, ReadItem, ReadItems, TemplateProvider};
mod ids;
mod items;
mod permissions;
mod resource;

pub use ids::{ItemId, ItemName, MAX_ITEM_NAME_CHARS, TemplateIdError};
pub use items::{CreateItemRequest, CreatedItem, ITEM_FORMAT, ItemView};
pub use permissions::{TemplatePermission, TemplateScope};
pub use resource::{TemplateDocument, TemplateResource, TemplateResourceError};
