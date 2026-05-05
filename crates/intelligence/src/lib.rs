// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Business intelligence providers for Organism.
//!
//! These modules provide domain-specific data acquisition capabilities
//! for Organism's planning and adversarial layers. They were extracted
//! from `converge-provider` because they encode business intelligence
//! patterns, not generic kernel infrastructure.
//!
//! # Available Modules
//!
//! - [`ocr`] — Document understanding (Tesseract, Mistral, DeepSeek, LightOn)
//! - [`patent`] — IP landscape, competitive intelligence
//! - [`vision`] — Scene understanding and object detection (Claude, GPT-4o, Gemini, Pixtral)
//! - [`billing`] — Stripe ACP integration for SaaS products
//! - [`web`] — URL capture and metadata extraction for public pages
//! - [`social`] — Normalized social profile/page extraction via web capture
//!
//! LinkedIn has moved to [`embassy-linkedin`](https://github.com/Reflective-Lab/embassy)
//! — see `~/dev/extensions/embassy/crates/linkedin/`.

pub mod provenance;
pub mod secret;

#[cfg(feature = "ocr")]
pub mod ocr;

#[cfg(feature = "pdf")]
pub mod pdf;

#[cfg(feature = "vision")]
pub mod vision;

#[cfg(feature = "patent")]
pub mod patent;

#[cfg(feature = "billing")]
pub mod billing;

#[cfg(feature = "web")]
pub mod web;

#[cfg(feature = "social")]
pub mod social;
