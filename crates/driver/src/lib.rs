//! Driver implementations for Minecraft versions and mod loaders.
//!
//! A driver inspects and prepares local game instances, resolves launch
//! metadata, and provides catalogs for available game and loader versions.
//! Drivers never execute the game directly; they produce data consumed by the
//! launcher layer.

pub mod catalog;
pub mod descriptors;
pub mod driver;
pub mod drivers;
pub mod error;
pub mod families;
mod http;
pub mod inspect;
pub mod launch;
pub mod loader_version;
mod maven;
pub mod url;
