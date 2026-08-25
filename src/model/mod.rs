//! Core Discord data models.

mod message;
mod snowflake;
mod user;

pub use message::{CreateMessage, Message};
pub use snowflake::{DISCORD_EPOCH_MILLIS, Snowflake};
pub use user::User;
