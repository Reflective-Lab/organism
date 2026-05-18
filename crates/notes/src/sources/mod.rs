//! Source adapters for note ingestion.
//!
//! Each source knows how to import notes from a specific origin
//! into the vault's pipeline stages.
//!
//! The Apple Notes and web adapters sit behind cargo features that
//! gate **optional deps** (HTML parsing, HTTP/TLS) — see
//! `organism-notes/Cargo.toml` for the carve-out rationale. The
//! markdown adapter has no extra deps and is always compiled.

pub mod markdown;

#[cfg(feature = "sources-apple-notes")]
pub mod apple_notes;

#[cfg(feature = "sources-web")]
pub mod web;
