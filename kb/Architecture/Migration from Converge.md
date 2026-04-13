---
tags: [architecture]
---
# Migration from Converge

Business intelligence modules currently live in `converge-provider` but belong in Organism. This page tracks the migration.

## The Line

**Converge owns generic kernel infrastructure:** LLM routing, embedding, vector search, reranking, MCP tools, API gateway adapters. These are capabilities any system might need.

**Organism owns business intelligence:** capabilities that help an organization understand the world. Document reading, professional network research, IP landscape analysis, web research.

**The test:** If a generic Converge deployment would never need this capability, it belongs in Organism.

## What Moves

| Source (converge-provider) | Target (organism) | What it does |
|---|---|---|
| `src/ocr.rs` | `crates/intelligence/ocr.rs` | Document understanding (Tesseract, Mistral, DeepSeek, LightOn) |
| `src/linkedin.rs` | `crates/intelligence/linkedin.rs` | Professional network research |
| `src/patent.rs` | `crates/intelligence/patent.rs` | IP landscape, competitive intelligence |
| `src/graph/` | `crates/intelligence/graph/` | Knowledge graph traversal |

## What Stays in Converge

All generic LLM providers (Anthropic, OpenAI, Gemini, Ollama, etc.), embedding, vector search, reranking, MCP tools, Kong gateway, provider infrastructure, model selection, fallback routing.

**Web search (Brave, Firecrawl) stays in Converge.** It's a generic retrieval capability that complements LLMs — like embedding or reranking. Any system might need web-grounded responses. It's infrastructure, not business intelligence.

## Migration Steps

1. Create `crates/intelligence/` in Organism with `Cargo.toml`
2. Copy the source files from `converge-provider/src/` (listed above)
3. Define trait boundaries — each service gets a trait in the intelligence crate
4. Update imports to use `converge-provider-api` for backend identity (not `converge-core`)
5. Remove the moved modules from `converge-provider` and their feature flags
6. Update `converge-provider/Cargo.toml` to drop the `patent`, `linkedin`, `brave`, `ocr` features
7. Wire into Organism's planning/adversarial crates as injectable capabilities

## Trait Pattern

```rust
// In crates/intelligence/src/ocr.rs
#[async_trait]
pub trait OcrService: Send + Sync {
    async fn extract(&self, document: &[u8]) -> Result<OcrResult>;
}

// Implementations: TesseractOcr, MistralOcr, DeepSeekOcr, LightOnOcr
```

Reasoners and adversarial agents inject these as `Arc<dyn OcrService>` — they don't know which implementation they're using.

## When

After the starter task (wiring Converge integration into runtime) is done. The intelligence crate is the second milestone — it gives Organism's planning layer access to real business data.

See also: [[Stack/Business Services]], [[Architecture/Crate Map]]
