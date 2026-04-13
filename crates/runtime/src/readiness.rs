//! Readiness check — validates that a resolved intent binding can actually execute.
//!
//! Resolution says what you NEED. Readiness says what you HAVE.
//! The gap between them is the readiness report.
//!
//! Checks:
//! - Are required capabilities compiled in? (feature flags)
//! - Are credentials available? (API keys, tokens)
//! - Is there budget? (token limits, spend caps)
//! - Are external services reachable? (optional, expensive)
//!
//! ```rust,ignore
//! let binding = resolver.resolve(&intent, &baseline);
//! let report = readiness::check(&binding, &registry);
//!
//! if !report.ready {
//!     for gap in &report.gaps {
//!         eprintln!("{}: {}", gap.resource, gap.reason);
//!     }
//!     // → "linkedin: LINKEDIN_API_KEY not set"
//!     // → "ocr: feature 'ocr' not compiled"
//!     return Err(report);
//! }
//! ```

use organism_intent::resolution::IntentBinding;
use serde::{Deserialize, Serialize};

// ── Readiness Report ───────────────────────────────────────────────

/// Result of checking whether a binding can execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReport {
    /// Can the binding execute right now?
    pub ready: bool,
    /// What's available and confirmed.
    pub confirmed: Vec<ReadinessConfirmation>,
    /// What's missing or degraded.
    pub gaps: Vec<ReadinessGap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessConfirmation {
    pub resource: String,
    pub kind: ResourceKind,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessGap {
    pub resource: String,
    pub kind: ResourceKind,
    pub severity: GapSeverity,
    pub reason: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    /// A compiled feature flag (e.g., "ocr", "vision").
    Feature,
    /// An API key or credential (e.g., ANTHROPIC_API_KEY).
    Credential,
    /// A spending or token budget.
    Budget,
    /// An external service endpoint.
    Service,
    /// A domain pack being registered.
    Pack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapSeverity {
    /// Cannot proceed without this. Hard stop.
    Blocking,
    /// Can proceed but with degraded quality.
    Degraded,
    /// Informational — might affect results.
    Advisory,
}

// ── Readiness Checker ──────────────────────────────────────────────

/// A single readiness probe. Implementations check one kind of resource.
pub trait ReadinessProbe: Send + Sync {
    fn kind(&self) -> ResourceKind;
    fn check(&self, binding: &IntentBinding) -> Vec<ReadinessItem>;
}

/// Single item from a probe — either confirmed or a gap.
pub enum ReadinessItem {
    Confirmed(ReadinessConfirmation),
    Gap(ReadinessGap),
}

/// Run all probes against a binding and produce a report.
pub fn check(binding: &IntentBinding, probes: &[&dyn ReadinessProbe]) -> ReadinessReport {
    let mut confirmed = Vec::new();
    let mut gaps = Vec::new();

    for probe in probes {
        for item in probe.check(binding) {
            match item {
                ReadinessItem::Confirmed(c) => confirmed.push(c),
                ReadinessItem::Gap(g) => gaps.push(g),
            }
        }
    }

    let ready = !gaps.iter().any(|g| g.severity == GapSeverity::Blocking);

    ReadinessReport {
        ready,
        confirmed,
        gaps,
    }
}

// ── Built-in Probes ────────────────────────────────────────────────

/// Checks that required capabilities have their credentials available
/// by reading environment variables.
pub struct CredentialProbe {
    /// Maps capability name → environment variable name.
    checks: Vec<(String, String)>,
}

impl CredentialProbe {
    #[must_use]
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Register a credential requirement: if capability X is needed,
    /// environment variable Y must be set.
    #[must_use]
    pub fn require(mut self, capability: impl Into<String>, env_var: impl Into<String>) -> Self {
        self.checks.push((capability.into(), env_var.into()));
        self
    }

    /// Standard credential checks for organism-intelligence providers.
    #[must_use]
    pub fn with_standard_checks(self) -> Self {
        self.require("vision", "ANTHROPIC_API_KEY")
            .require("ocr", "MISTRAL_API_KEY")
            .require("linkedin", "LINKEDIN_API_KEY")
            .require("patent", "USPTO_API_KEY")
            .require("social", "ANTHROPIC_API_KEY")
    }
}

impl Default for CredentialProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadinessProbe for CredentialProbe {
    fn kind(&self) -> ResourceKind {
        ResourceKind::Credential
    }

    fn check(&self, binding: &IntentBinding) -> Vec<ReadinessItem> {
        let needed_capabilities: Vec<&str> = binding
            .capabilities
            .iter()
            .map(|c| c.capability.as_str())
            .collect();

        let mut items = Vec::new();
        for (capability, env_var) in &self.checks {
            if !needed_capabilities.contains(&capability.as_str()) {
                continue;
            }
            if std::env::var(env_var).is_ok() {
                items.push(ReadinessItem::Confirmed(ReadinessConfirmation {
                    resource: capability.clone(),
                    kind: ResourceKind::Credential,
                    detail: format!("{env_var} is set"),
                }));
            } else {
                items.push(ReadinessItem::Gap(ReadinessGap {
                    resource: capability.clone(),
                    kind: ResourceKind::Credential,
                    severity: GapSeverity::Blocking,
                    reason: format!("{env_var} is not set"),
                    suggestion: Some(format!("export {env_var}=<your-key>")),
                }));
            }
        }
        items
    }
}

/// Checks that all packs in the binding are registered in the registry.
pub struct PackProbe<'a> {
    registry: &'a super::registry::Registry,
}

impl<'a> PackProbe<'a> {
    #[must_use]
    pub fn new(registry: &'a super::registry::Registry) -> Self {
        Self { registry }
    }
}

impl ReadinessProbe for PackProbe<'_> {
    fn kind(&self) -> ResourceKind {
        ResourceKind::Pack
    }

    fn check(&self, binding: &IntentBinding) -> Vec<ReadinessItem> {
        let mut items = Vec::new();
        for pack_req in &binding.packs {
            let registered = self
                .registry
                .packs()
                .iter()
                .any(|p| p.name == pack_req.pack_name);
            if registered {
                items.push(ReadinessItem::Confirmed(ReadinessConfirmation {
                    resource: pack_req.pack_name.clone(),
                    kind: ResourceKind::Pack,
                    detail: "registered in runtime".into(),
                }));
            } else {
                let severity = if pack_req.confidence >= 0.8 {
                    GapSeverity::Blocking
                } else {
                    GapSeverity::Degraded
                };
                items.push(ReadinessItem::Gap(ReadinessGap {
                    resource: pack_req.pack_name.clone(),
                    kind: ResourceKind::Pack,
                    severity,
                    reason: format!(
                        "pack '{}' needed ({:?}, confidence {:.0}%) but not registered",
                        pack_req.pack_name,
                        pack_req.source,
                        pack_req.confidence * 100.0
                    ),
                    suggestion: Some(format!(
                        "registry.register_pack(\"{}\", ...)",
                        pack_req.pack_name
                    )),
                }));
            }
        }
        items
    }
}

/// Checks token/spend budget.
pub struct BudgetProbe {
    /// Maximum token spend allowed for this intent.
    pub token_budget: Option<u64>,
    /// Maximum dollar spend allowed.
    pub spend_budget: Option<f64>,
}

impl BudgetProbe {
    #[must_use]
    pub fn new() -> Self {
        Self {
            token_budget: None,
            spend_budget: None,
        }
    }

    #[must_use]
    pub fn with_token_budget(mut self, tokens: u64) -> Self {
        self.token_budget = Some(tokens);
        self
    }

    #[must_use]
    pub fn with_spend_budget(mut self, dollars: f64) -> Self {
        self.spend_budget = Some(dollars);
        self
    }
}

impl Default for BudgetProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadinessProbe for BudgetProbe {
    fn kind(&self) -> ResourceKind {
        ResourceKind::Budget
    }

    fn check(&self, binding: &IntentBinding) -> Vec<ReadinessItem> {
        let mut items = Vec::new();
        let needs_llm = binding
            .capabilities
            .iter()
            .any(|c| ["vision", "ocr", "social"].contains(&c.capability.as_str()));

        if needs_llm {
            if let Some(tokens) = self.token_budget {
                if tokens > 0 {
                    items.push(ReadinessItem::Confirmed(ReadinessConfirmation {
                        resource: "token_budget".into(),
                        kind: ResourceKind::Budget,
                        detail: format!("{tokens} tokens available"),
                    }));
                } else {
                    items.push(ReadinessItem::Gap(ReadinessGap {
                        resource: "token_budget".into(),
                        kind: ResourceKind::Budget,
                        severity: GapSeverity::Blocking,
                        reason: "token budget exhausted".into(),
                        suggestion: Some("increase token budget or remove LLM capabilities".into()),
                    }));
                }
            }

            if let Some(spend) = self.spend_budget {
                if spend > 0.0 {
                    items.push(ReadinessItem::Confirmed(ReadinessConfirmation {
                        resource: "spend_budget".into(),
                        kind: ResourceKind::Budget,
                        detail: format!("${spend:.2} remaining"),
                    }));
                } else {
                    items.push(ReadinessItem::Gap(ReadinessGap {
                        resource: "spend_budget".into(),
                        kind: ResourceKind::Budget,
                        severity: GapSeverity::Blocking,
                        reason: "spend budget exhausted".into(),
                        suggestion: Some("increase spend budget".into()),
                    }));
                }
            }
        }

        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use organism_intent::resolution::DeclarativeBinding;

    #[test]
    fn reports_ready_when_no_gaps() {
        let binding = DeclarativeBinding::new()
            .pack("customers", "lead qualification")
            .build();

        let report = check(&binding, &[]);
        assert!(report.ready);
        assert!(report.gaps.is_empty());
    }

    #[test]
    fn credential_probe_detects_missing_key() {
        let binding = DeclarativeBinding::new()
            .capability("vision", "scene understanding")
            .build();
        // Use a key name that will not exist in the test environment

        // Use a key name that is very unlikely to be set
        let binding = DeclarativeBinding::new()
            .capability("vision", "scene understanding")
            .build();

        // Use a fake env var that won't exist

        let probe =
            CredentialProbe::new().require("vision", "ORGANISM_TEST_KEY_THAT_DOES_NOT_EXIST");
        let report = check(&binding, &[&probe]);

        assert!(!report.ready);
        assert_eq!(report.gaps.len(), 1);
        assert_eq!(report.gaps[0].resource, "vision");
        assert_eq!(report.gaps[0].severity, GapSeverity::Blocking);
        assert!(report.gaps[0].reason.contains("not set"));
    }

    #[test]
    fn pack_probe_detects_unregistered_pack() {
        let binding = DeclarativeBinding::new()
            .pack("customers", "lead qualification")
            .pack("legal", "contract review")
            .build();

        let mut registry = super::super::registry::Registry::new();
        registry.register_pack_raw(super::super::registry::RegisteredPack {
            name: "customers".into(),
            description: "revenue ops".into(),
            fact_prefixes: vec!["lead:".into()],
            agent_names: vec![],
            invariant_names: vec![],
            agent_count: 8,
            invariant_count: 2,
            context_keys_read: vec![],
            context_keys_written: vec![],
            has_acceptance_invariants: false,
            profile: organism_domain::pack::PackProfile::default(),
        });
        // legal is NOT registered

        let probe = PackProbe::new(&registry);
        let report = check(&binding, &[&probe]);

        assert!(!report.ready);
        assert_eq!(report.confirmed.len(), 1);
        assert_eq!(report.confirmed[0].resource, "customers");
        assert_eq!(report.gaps.len(), 1);
        assert_eq!(report.gaps[0].resource, "legal");
    }

    #[test]
    fn budget_probe_blocks_on_zero_budget() {
        let binding = DeclarativeBinding::new()
            .capability("vision", "scene analysis")
            .build();

        let probe = BudgetProbe::new().with_token_budget(0);
        let report = check(&binding, &[&probe]);

        assert!(!report.ready);
        assert!(report.gaps.iter().any(|g| g.resource == "token_budget"));
    }

    #[test]
    fn budget_probe_confirms_available_budget() {
        let binding = DeclarativeBinding::new()
            .capability("ocr", "document reading")
            .build();

        let probe = BudgetProbe::new()
            .with_token_budget(100_000)
            .with_spend_budget(5.0);
        let report = check(&binding, &[&probe]);

        assert!(report.ready);
        assert_eq!(report.confirmed.len(), 2);
    }
}
