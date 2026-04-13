// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Vision — scene understanding and object detection for Organism.
//!
//! OCR extracts text. Vision describes content. Both produce observations
//! that feed into Organism's planning layer.
//!
//! # Available Backends
//!
//! | Backend | Model | Strengths |
//! |---|---|---|
//! | Anthropic | Claude (Sonnet/Opus) | Detailed reasoning, spatial understanding |
//! | Gemini | Gemini Pro Vision | Native multimodal, fast |
//! | OpenAI | GPT-4o | Strong general vision |
//! | Mistral | Pixtral | EU-hosted, GDPR-compliant |
//!
//! # Usage
//!
//! ```ignore
//! use organism_intelligence::vision::{VisionDescriber, AnthropicVision, VisionRequest};
//!
//! let vision = AnthropicVision::from_env()?;
//! let request = VisionRequest::from_bytes(image_bytes)
//!     .with_prompt("What products are visible on this shelf?");
//! let description = vision.describe(&request)?;
//!
//! for object in &description.objects {
//!     println!("{}: {:.0}% confidence", object.label, object.confidence * 100.0);
//! }
//! println!("Scene: {}", description.scene);
//! ```

mod backends;
mod types;

pub use backends::*;
pub use types::*;
