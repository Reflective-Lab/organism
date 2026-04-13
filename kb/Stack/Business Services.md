---
tags: [stack]
---
# Business Services

Organism uses external business data services for planning intelligence. These are consumed through provider adapters — the planning layer injects them as trait objects.

## Available Services

| Service | Purpose | Legacy Location |
|---|---|---|
| OCR | Document understanding, invoice parsing | `_legacy/organism-domain/` (Tesseract, Mistral, DeepSeek, LightOn) |
| LinkedIn | Professional network research, company intel | `_legacy/organism-domain/src/packs/linkedin_research.rs` |
| Patent Search | IP landscape, competitive intelligence | `_legacy/organism-domain/src/use_cases/patent_research.rs` |
| Brave Search | Web search with citations | `_legacy/organism-application/` |

## Architecture

These services were previously in `converge-provider` as feature-gated providers. They belong in Organism — they are business intelligence capabilities, not kernel infrastructure.

The pattern: define a trait for the capability, implement it against the real service, mock it for testing. Inject into reasoners or adversarial agents at construction time.

```rust
trait PatentSearchService: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<Patent>>;
}
```

## Migration Status

These services exist in `_legacy/` as working implementations. They need to be extracted into the new crate structure as the relevant planning/adversarial crates take shape.

See also: [[Building/Getting Started]], [[Architecture/Crate Map]]
