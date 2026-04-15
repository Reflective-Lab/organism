---
tags: [stack]
---
# Business Services

Organism uses external business data services for planning intelligence. These are consumed through provider adapters — the planning layer injects them as trait objects.

## Available Services

| Service | Purpose | Current Home |
|---|---|---|
| OCR | Document understanding, invoice parsing | `crates/intelligence/src/ocr/` |
| LinkedIn | Professional network research, company intel | `crates/intelligence/src/linkedin.rs` and `crates/domain/src/packs/linkedin_research.rs` |
| Patent Search | IP landscape, competitive intelligence | `crates/intelligence/src/patent.rs` and `crates/domain/src/blueprints/patent_research.rs` |
| Brave Search | Web search with citations | Converge provider surfaces; injected into Organism planning as a dependency |

## Architecture

These services were previously in `converge-provider` as feature-gated providers. They belong in Organism — they are business intelligence capabilities, not kernel infrastructure.

The pattern: define a trait for the capability, implement it against the real service, mock it for testing. Inject into reasoners or adversarial agents at construction time.

```rust
trait PatentSearchService: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<Patent>>;
}
```

## Migration Status

These services now live under the current Organism crate structure. Organism owns the business-facing capability contracts and domain wiring; generic web search stays in Converge and is injected where needed.

See also: [[Building/Getting Started]], [[Architecture/Crate Map]]
