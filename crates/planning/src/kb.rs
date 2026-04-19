//! Generic knowledge base writer for DD results.
//!
//! Writes convergent DD output to an Obsidian-compatible vault structure.
//! Apps parameterize the writer via `KbConfig` to control directory names,
//! hub categories, and root page definitions.

use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;

use chrono::Utc;
use converge_pack::ContextKey;

use crate::dd::{DdHooks, HookPatterns, consolidate_dd_hypotheses, extract_hooks_from_facts};

// ── Configuration ────────────────────────────────────────────────

/// Controls the vault directory structure and page layout.
#[derive(Debug, Clone)]
pub struct KbConfig {
    /// Top-level directory for subjects (e.g. "Companies", "Targets", "Deals").
    pub entity_dir: String,
    /// Hub categories to generate (Business Areas, Regions, etc.).
    pub hub_categories: Vec<HubCategory>,
    /// Root MOC pages to auto-generate.
    pub root_pages: Vec<RootPageDef>,
    /// Hook extraction patterns. Uses defaults if None.
    pub hook_patterns: Option<HookPatterns>,
}

#[derive(Debug, Clone)]
pub struct HubCategory {
    pub dir_name: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct RootPageDef {
    pub filename: String,
    pub title: String,
    pub scan_dirs: Vec<String>,
    pub children_are_dirs: bool,
}

// ── Vault Writer ─────────────────────────────────────────────────

/// Write convergent DD results to an Obsidian vault.
///
/// Returns the list of written file paths (relative to `vault_root`).
#[allow(clippy::too_many_lines)]
pub fn write_dd_to_vault(
    vault_root: &Path,
    subject: &str,
    result: &converge_kernel::ConvergeResult,
    config: &KbConfig,
) -> anyhow::Result<Vec<String>> {
    let slug = slugify(subject);
    let base = vault_root.join(&config.entity_dir).join(&slug);
    let sources_dir = base.join("Sources");
    let analysis_dir = base.join("Analysis");

    std::fs::create_dir_all(&sources_dir)?;
    std::fs::create_dir_all(&analysis_dir)?;

    let now = Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
    let corr = format!("convergent-dd:{slug}");
    let mut written_files = Vec::new();

    let signals = result.context.get(ContextKey::Signals);
    let hypotheses = result.context.get(ContextKey::Hypotheses);
    let evaluations = result.context.get(ContextKey::Evaluations);
    let proposals = result.context.get(ContextKey::Proposals);

    let consolidated = consolidate_dd_hypotheses(hypotheses);

    // ── Source pages ─────────────────────────────────────────────────

    let mut seen_urls = HashSet::new();
    let mut deduped_signals = Vec::new();
    for signal in signals {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&signal.content) {
            let url = v["url"].as_str().unwrap_or("").to_string();
            if seen_urls.insert(url) {
                deduped_signals.push(signal);
            }
        }
    }

    for (i, signal) in deduped_signals.iter().enumerate() {
        let v: serde_json::Value = serde_json::from_str(&signal.content).unwrap_or_default();
        let title_raw = v["title"].as_str().unwrap_or("Untitled");
        let url = v["url"].as_str().unwrap_or("");
        let content = v["content"].as_str().unwrap_or("");
        let provider = v["provider"].as_str().unwrap_or("unknown");
        let query = v["query"].as_str().unwrap_or("");
        let title = sanitize_filename(title_raw);
        let filename = format!("Source {:03} — {}.md", i + 1, title);

        let page = format!(
            "---\ntags: [source, {provider}, {entity_tag}]\n\
             provenance: fact\ncorrelation_id: {corr}\n\
             provider: {provider}\nurl: {url}\n\
             query: \"{query}\"\ngenerated_at: {now}\n---\n\
             # {title_raw}\n\n\
             > Source: [{provider}]({url})\n> Query: `{query}`\n\n{content}\n",
            entity_tag = slug,
            query = query.replace('"', "'"),
        );

        std::fs::write(sources_dir.join(&filename), &page)?;
        written_files.push(format!(
            "{entity_dir}/{slug}/Sources/{filename}",
            entity_dir = config.entity_dir
        ));
    }

    // ── Facts page ───────────────────────────────────────────────────

    let mut facts_body = String::new();
    for (i, fact) in consolidated.iter().enumerate() {
        let conf_label = if fact.confidence >= 0.9 {
            "HIGH"
        } else if fact.confidence >= 0.7 {
            "MEDIUM"
        } else {
            "LOW"
        };

        let support = if fact.support_count > 1 {
            format!(" (corroborated by {} sources)", fact.support_count)
        } else {
            String::new()
        };

        let _ = write!(
            facts_body,
            "### Fact {num}\n\n\
             **Category:** `{cat}`\n\
             **Confidence:** {conf_label} ({conf:.0}%){support}\n\
             **Claim:** {claim}\n\n---\n\n",
            num = i + 1,
            cat = fact.category,
            conf = fact.confidence * 100.0,
            claim = fact.claim,
        );
    }

    let facts_page = format!(
        "---\ntags: [facts, {entity_tag}]\nprovenance: inferred\n\
         correlation_id: {corr}\ngenerated_at: {now}\n---\n\
         # {subject} — Tagged Facts\n\n\
         > Extracted via convergent DD ({cycles} cycles, converged: {converged}).\n\
         > {n_raw} raw facts consolidated to {n_consolidated} unique claims.\n\
         > Provenance: **inferred** by LLM from source data.\n\n\
         {facts_body}",
        entity_tag = slug,
        cycles = result.cycles,
        converged = result.converged,
        n_raw = hypotheses.len(),
        n_consolidated = consolidated.len(),
    );
    std::fs::write(base.join("Facts.md"), &facts_page)?;
    written_files.push(format!("{}/{slug}/Facts.md", config.entity_dir));

    // ── Analysis pages ───────────────────────────────────────────────

    let synthesis: Option<serde_json::Value> = proposals
        .iter()
        .find_map(|p| serde_json::from_str(&p.content).ok());

    let analysis_sections = [
        ("Market", "market_analysis", "market"),
        ("Competition", "competitive_landscape", "competition"),
        ("Technology", "technology_assessment", "technology"),
    ];

    for (name, json_key, category) in &analysis_sections {
        let content = synthesis
            .as_ref()
            .and_then(|s| s[json_key].as_str())
            .map_or_else(
                || {
                    let matching: Vec<String> = consolidated
                        .iter()
                        .filter(|f| {
                            f.category == *category
                                || (*category == "competition" && f.category == "competitors")
                        })
                        .map(|f| format!("- {}", f.claim))
                        .collect();
                    if matching.is_empty() {
                        format!("*No {name} data collected during this run.*")
                    } else {
                        format!(
                            "*Synthesis did not complete. Facts in this category:*\n\n{}",
                            matching.join("\n")
                        )
                    }
                },
                String::from,
            );

        let cat_tag = category;
        let entity_tag = &slug;
        let page = format!(
            "---\ntags: [analysis, {cat_tag}, {entity_tag}]\n\
             provenance: inferred\ncorrelation_id: {corr}\n\
             generated_at: {now}\n---\n\
             # {subject} — {name} Analysis\n\n\
             > Synthesised from convergent DD facts.\n\
             > Provenance: **inferred**. Verify claims against [[Facts]] source links.\n\n\
             {content}\n",
        );
        std::fs::write(analysis_dir.join(format!("{name}.md")), &page)?;
        written_files.push(format!("{}/{slug}/Analysis/{name}.md", config.entity_dir));
    }

    // ── Risks page ───────────────────────────────────────────────────

    let risks_body = synthesis
        .as_ref()
        .and_then(|s| s["risk_factors"].as_array())
        .map_or_else(
            || "*No risk factors synthesised.*".to_string(),
            |arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| format!("- {s}")))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
        );

    let entity_tag = &slug;
    let risks_page = format!(
        "---\ntags: [risks, {entity_tag}]\nprovenance: inferred\n\
         correlation_id: {corr}\ngenerated_at: {now}\n---\n\
         # {subject} — Risk Factors\n\n\
         > Synthesised from convergent DD.\n\
         > Provenance: **inferred**. Verify against source data.\n\n\
         {risks_body}\n",
    );
    std::fs::write(base.join("Risks.md"), &risks_page)?;
    written_files.push(format!("{}/{slug}/Risks.md", config.entity_dir));

    // ── Opportunities page ───────────────────────────────────────────

    let opps_body = synthesis
        .as_ref()
        .and_then(|s| s["growth_opportunities"].as_array())
        .map_or_else(
            || "*No growth opportunities synthesised.*".to_string(),
            |arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| format!("- {s}")))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
        );

    let opps_page = format!(
        "---\ntags: [opportunities, {entity_tag}]\nprovenance: inferred\n\
         correlation_id: {corr}\ngenerated_at: {now}\n---\n\
         # {subject} — Growth Opportunities\n\n\
         > Synthesised from convergent DD.\n\
         > Provenance: **inferred**. Verify against source data.\n\n\
         {opps_body}\n",
    );
    std::fs::write(base.join("Opportunities.md"), &opps_page)?;
    written_files.push(format!("{}/{slug}/Opportunities.md", config.entity_dir));

    // ── Contradictions page ──────────────────────────────────────────

    if !evaluations.is_empty() {
        let contra_body: String = evaluations
            .iter()
            .enumerate()
            .filter_map(|(i, eval)| {
                let v: serde_json::Value = serde_json::from_str(&eval.content).ok()?;
                if v["type"].as_str() != Some("contradiction") {
                    return None;
                }
                let cat = v["category"].as_str().unwrap_or("unknown");
                let desc = v["description"].as_str().unwrap_or(&eval.content);
                let needs_review = v["needs_human_review"].as_bool().unwrap_or(false);
                let review_tag = if needs_review {
                    " **NEEDS REVIEW**"
                } else {
                    ""
                };
                Some(format!(
                    "### Contradiction {num}\n\n\
                     **Category:** {cat}\n\
                     **Description:** {desc}{review_tag}\n\n---\n",
                    num = i + 1,
                ))
            })
            .collect();

        if !contra_body.is_empty() {
            let entity_tag = &slug;
            let contra_page = format!(
                "---\ntags: [contradictions, {entity_tag}]\n\
                 correlation_id: {corr}\ngenerated_at: {now}\n---\n\
                 # {subject} — Contradictions\n\n\
                 > Sources that disagree on the same claim. Resolve before decisions.\n\n\
                 {contra_body}",
            );
            std::fs::write(base.join("Contradictions.md"), &contra_page)?;
            written_files.push(format!("{}/{slug}/Contradictions.md", config.entity_dir));
        }
    }

    // ── Hub pages ────────────────────────────────────────────────────

    let patterns = config.hook_patterns.clone().unwrap_or_default();
    let hooks = extract_hooks_from_facts(subject, &consolidated, &patterns);

    let hub_mappings: Vec<(&str, &[String])> = vec![
        ("Investors", &hooks.investors),
        ("Business Areas", &hooks.business_areas),
        ("Regions", &hooks.regions),
        ("Competitors", &hooks.competitors),
    ];

    for (hub_dir, items) in &hub_mappings {
        for item in *items {
            let files = write_or_append_hub(vault_root, hub_dir, item, subject, &corr, &now)?;
            written_files.extend(files);
        }
    }

    let hook_section = format_hook_links(&hooks);

    // ── Subject overview ─────────────────────────────────────────────

    let summary = synthesis
        .as_ref()
        .and_then(|s| s["summary"].as_str())
        .unwrap_or("*Synthesis did not complete — see Facts for raw findings.*");

    let recommendation = synthesis
        .as_ref()
        .and_then(|s| s["recommendation"].as_str())
        .unwrap_or("*Pending synthesis completion.*");

    let graph_section = if hook_section.is_empty() {
        String::new()
    } else {
        format!("## Graph Connections\n\n{hook_section}\n")
    };

    let overview = format!(
        "---\ntags: [{entity_dir_lower}, {entity_tag}]\n\
         correlation_id: {corr}\ngenerated_at: {now}\n---\n\
         # {subject}\n\n\
         > Convergent due diligence overview. Correlation ID: `{corr}`\n\n\
         ## Summary\n\n{summary}\n\n\
         ## Recommendation\n\n{recommendation}\n\n\
         ## Navigation\n\n\
         ### Analysis\n\
         - [[Analysis/Market|Market Analysis]]\n\
         - [[Analysis/Competition|Competitive Landscape]]\n\
         - [[Analysis/Technology|Technology Assessment]]\n\n\
         ### Evidence\n\
         - [[Facts|Tagged Facts]] — {n_facts} facts\n\
         - [[Risks|Risk Factors]]\n\
         - [[Opportunities|Growth Opportunities]]\n\
         - [[Contradictions]] — {n_contradictions} flagged\n\n\
         ### Raw Sources\n\
         {n_sources} source documents in [[Sources/]]\n\n\
         {graph_section}\
         ## Metadata\n\n\
         | Field | Value |\n|-------|-------|\n\
         | Correlation ID | `{corr}` |\n\
         | Cycles | {cycles} |\n\
         | Converged | {converged} |\n\
         | Strategies | {n_strategies} |\n\
         | Generated | {now} |\n",
        entity_dir_lower = config.entity_dir.to_lowercase(),
        entity_tag = slug,
        n_facts = consolidated.len(),
        n_contradictions = evaluations.len(),
        n_sources = deduped_signals.len(),
        n_strategies = result.context.get(ContextKey::Strategies).len(),
        cycles = result.cycles,
        converged = result.converged,
    );
    std::fs::write(base.join(format!("{subject}.md")), &overview)?;
    written_files.push(format!("{}/{slug}/{subject}.md", config.entity_dir));

    Ok(written_files)
}

// ── Hub Pages ────────────────────────────────────────────────────

/// Create a hub page if it doesn't exist, or append this subject to an existing one.
pub fn write_or_append_hub(
    vault_root: &Path,
    hub_dir: &str,
    entity: &str,
    subject: &str,
    corr: &str,
    now: &str,
) -> anyhow::Result<Vec<String>> {
    let dir = vault_root.join(hub_dir);
    std::fs::create_dir_all(&dir)?;

    let filename = format!("{}.md", sanitize_filename(entity));
    let filepath = dir.join(&filename);
    let rel_path = format!("{hub_dir}/{filename}");

    let tag = slugify(hub_dir);
    let company_link = format!("[[{subject}]]");

    if filepath.exists() {
        let existing = std::fs::read_to_string(&filepath)?;
        if !existing.contains(&company_link) {
            let updated = format!("{existing}\n- {company_link} _(corr: `{corr}`, {now})_");
            std::fs::write(&filepath, updated)?;
        }
        Ok(vec![])
    } else {
        let page = format!(
            "---\ntags: [hub, {tag}]\nentity: \"{entity}\"\n---\n\
             # {entity}\n\n\
             > Hub page — auto-created by due diligence. Links grow as more subjects are analyzed.\n\n\
             ## Subjects\n\n\
             - {company_link} _(corr: `{corr}`, {now})_\n",
        );
        std::fs::write(&filepath, page)?;
        Ok(vec![rel_path])
    }
}

// ── Root Pages ───────────────────────────────────────────────────

/// Rebuild all root MOC pages by scanning the vault filesystem.
pub fn update_root_pages(vault_root: &Path, config: &KbConfig) -> anyhow::Result<Vec<String>> {
    let mut written = Vec::new();

    for root_def in &config.root_pages {
        let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();

        for scan_dir in &root_def.scan_dirs {
            let dir = vault_root.join(scan_dir);
            if !dir.exists() {
                continue;
            }

            let mut children: Vec<(String, String)> = Vec::new();

            if root_def.children_are_dirs {
                for entry in std::fs::read_dir(&dir)? {
                    let entry = entry?;
                    if !entry.file_type()?.is_dir() {
                        continue;
                    }
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    let display =
                        find_overview_page(&entry.path()).unwrap_or_else(|| titlecase(&dir_name));
                    let parent = scan_dir.as_str();
                    let link = format!("{parent}/{dir_name}/{display}");
                    children.push((link, display));
                }
            } else {
                for entry in std::fs::read_dir(&dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().is_none_or(|e| e != "md") {
                        continue;
                    }
                    let stem = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let link = format!("{scan_dir}/{stem}");
                    children.push((link, stem));
                }
            }

            children.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

            let section_title = if root_def.scan_dirs.len() > 1 {
                scan_dir.clone()
            } else {
                String::new()
            };
            sections.push((section_title, children));
        }

        let total_children: usize = sections.iter().map(|(_, c)| c.len()).sum();
        if total_children == 0 {
            continue;
        }

        let mut body = String::new();
        for (section_title, children) in &sections {
            if !section_title.is_empty() {
                let _ = write!(body, "### {section_title}\n\n");
            }
            for (link, display) in children {
                let _ = writeln!(body, "- [[{link}|{display}]]");
            }
            body.push('\n');
        }

        let page = format!(
            "---\ntags: [moc, root]\n---\n# {title}\n\n\
             > Root index — auto-updated by due diligence runs.\n\n\
             {body}",
            title = root_def.title,
        );

        std::fs::write(vault_root.join(&root_def.filename), &page)?;
        written.push(root_def.filename.clone());
    }

    Ok(written)
}

// ── Helpers ──────────────────────────────────────────────────────

fn find_overview_page(subject_dir: &Path) -> Option<String> {
    let skip = [
        "Facts",
        "Risks",
        "Opportunities",
        "Contradictions",
        "Loose Ends",
        "_raw_report",
    ];
    for entry in std::fs::read_dir(subject_dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") || path.is_dir() {
            continue;
        }
        let stem = path.file_stem()?.to_string_lossy().to_string();
        if skip.iter().any(|s| *s == stem) {
            continue;
        }
        return Some(stem);
    }
    None
}

fn format_hook_links(hooks: &DdHooks) -> String {
    let mut sections = Vec::new();

    let pairs: &[(&str, &str, &[String])] = &[
        ("Investors", "Investors", &hooks.investors),
        ("Business Areas", "Business Areas", &hooks.business_areas),
        ("Regions", "Regions", &hooks.regions),
        ("Competitors", "Competitors", &hooks.competitors),
    ];

    for (label, dir, items) in pairs {
        if !items.is_empty() {
            let links: Vec<String> = items
                .iter()
                .map(|item| {
                    let filename = sanitize_filename(item);
                    format!("[[{dir}/{filename}|{item}]]")
                })
                .collect();
            sections.push(format!("**{label}:** {}", links.join(" · ")));
        }
    }

    sections.join("\n\n")
}

fn titlecase(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in s.chars() {
        if c == '-' || c == '_' {
            result.push(' ');
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

pub fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .to_lowercase()
}

pub fn sanitize_filename(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect();
    if cleaned.len() > 80 {
        cleaned[..80].to_string()
    } else {
        cleaned
    }
}
