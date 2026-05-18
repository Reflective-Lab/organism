//! Note and vault capability for Organism.
//!
//! Reusable note lifecycle below the application layer. Apps consume
//! these types and traits; Organism owns the durable capability.
//!
//! # Modules
//!
//! - [`vault`] — Obsidian-compatible vault: note tree, CRUD, import, pipeline stages
//! - [`sources`] — Ingestion adapters: Markdown tree, Apple Notes, web capture
//! - [`cleanup`] — Duplicate detection, similarity candidates, merge suggestions
//! - [`enrichment`] — Freshness and value analysis, with room for richer derived passes
//! - `indexing` — Backlinks, chunks, embeddings, provenance (future)

pub mod cleanup;
pub mod enrichment;
pub mod sources;
pub mod vault;
