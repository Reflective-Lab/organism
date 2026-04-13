//! Source adapters for note ingestion.
//!
//! Each source knows how to import notes from a specific origin
//! into the vault's pipeline stages.

pub mod markdown;

#[cfg(feature = "sources-apple-notes")]
pub mod apple_notes;

#[cfg(feature = "sources-web")]
pub mod web;
