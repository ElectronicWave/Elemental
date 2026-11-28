//! Driver implementations for Minecraft versions and mod loaders.
//!
//! A driver inspects and prepares local game instances, resolves launch
//! metadata, and provides catalogs for available game and loader versions.
//! Drivers never execute the game directly; they produce data consumed by the
//! launcher layer.

elemental_core::require_tls_backend!();

pub mod catalog;
pub mod descriptors;
pub mod driver;
pub mod drivers;
pub mod error;
pub mod families;
pub mod inspect;
pub mod launch;
pub mod loader_version;
pub mod maven;
pub use elemental_core::url;
