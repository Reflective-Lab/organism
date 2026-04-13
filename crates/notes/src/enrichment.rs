use std::collections::{HashMap, HashSet};
use std::fs;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::vault::{
    ObsidianVault, VaultEntryKind, VaultError, VaultPipelineStage, extract_frontmatter_value,
    path_to_relative_string, split_frontmatter,
};

const MAX_VALUE_CANDIDATES: usize = 24;

#[derive(Debug, Error)]
pub enum NoteValueError {
    #[error(transparent)]
    Vault(#[from] VaultError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type NoteValueResult<T> = Result<T, NoteValueError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteValueReport {
    pub run_id: String,
    pub enriched_root: String,
    pub report_path: String,
    pub details_path: String,
    pub summary_path: String,
    pub note_count: usize,
    pub current_note_count: usize,
    pub aging_note_count: usize,
    pub stale_note_count: usize,
    pub unknown_freshness_note_count: usize,
    pub promote_candidate_count: usize,
    pub refresh_candidate_count: usize,
    pub demote_candidate_count: usize,
    pub promote_candidates: Vec<NoteValueCandidate>,
    pub refresh_candidates: Vec<NoteValueCandidate>,
    pub demote_candidates: Vec<NoteValueCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteValueCandidate {
    pub path: String,
    pub title: String,
    pub kind: String,
    pub provenance: String,
    pub review_state: String,
    pub freshness_status: NoteFreshnessStatus,
    pub suggested_action: NoteValueAction,
    pub overall_score_bps: u32,
    pub freshness_score_bps: u32,
    pub value_score_bps: u32,
    pub age_days: Option<i64>,
    pub inbound_reference_count: usize,
    pub outgoing_reference_count: usize,
    pub external_url_count: usize,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteFreshnessStatus {
    Current,
    Aging,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteValueAction {
    Keep,
    Promote,
    Refresh,
    Demote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NoteValueDetails {
    assessments: Vec<NoteValueCandidate>,
}

#[derive(Debug, Clone)]
struct IndexedNote {
    path: String,
    title: String,
    kind: String,
    provenance: String,
    review_state: String,
    freshness_anchor: Option<DateTime<Utc>>,
    external_url_count: usize,
    outgoing_reference_targets: Vec<String>,
    resolved_references: Vec<String>,
}

#[derive(Debug, Default)]
struct AliasIndex {
    title_aliases: HashMap<String, Vec<String>>,
    path_aliases: HashMap<String, Vec<String>>,
}

pub fn analyze_note_value(vault: &ObsidianVault) -> NoteValueResult<NoteValueReport> {
    vault.ensure_root()?;

    let mut notes = load_indexed_notes(vault)?;
    let aliases = build_alias_index(&notes);
    let inbound_reference_counts = resolve_references(&mut notes, &aliases);
    let now = Utc::now();

    let mut assessments = notes
        .iter()
        .map(|note| {
            let inbound_reference_count = inbound_reference_counts
                .get(&note.path)
                .copied()
                .unwrap_or(0);
            assess_note_value(note, inbound_reference_count, now)
        })
        .collect::<Vec<_>>();
    assessments.sort_by(|left, right| left.path.cmp(&right.path));

    let run = vault.prepare_pipeline_run(VaultPipelineStage::Enriched, "note-value")?;
    let report_relative_path = run.relative_root.join("report.json");
    let details_relative_path = run.relative_root.join("assessments.json");
    let summary_relative_path = run.relative_root.join("summary.md");

    let current_note_count = assessments
        .iter()
        .filter(|assessment| assessment.freshness_status == NoteFreshnessStatus::Current)
        .count();
    let aging_note_count = assessments
        .iter()
        .filter(|assessment| assessment.freshness_status == NoteFreshnessStatus::Aging)
        .count();
    let stale_note_count = assessments
        .iter()
        .filter(|assessment| assessment.freshness_status == NoteFreshnessStatus::Stale)
        .count();
    let unknown_freshness_note_count = assessments
        .iter()
        .filter(|assessment| assessment.freshness_status == NoteFreshnessStatus::Unknown)
        .count();

    let promote_candidates = sorted_candidates(&assessments, NoteValueAction::Promote);
    let refresh_candidates = sorted_candidates(&assessments, NoteValueAction::Refresh);
    let demote_candidates = sorted_candidates(&assessments, NoteValueAction::Demote);

    let report = NoteValueReport {
        run_id: run.run_id,
        enriched_root: path_to_relative_string(&run.relative_root),
        report_path: path_to_relative_string(&report_relative_path),
        details_path: path_to_relative_string(&details_relative_path),
        summary_path: path_to_relative_string(&summary_relative_path),
        note_count: assessments.len(),
        current_note_count,
        aging_note_count,
        stale_note_count,
        unknown_freshness_note_count,
        promote_candidate_count: assessments
            .iter()
            .filter(|assessment| assessment.suggested_action == NoteValueAction::Promote)
            .count(),
        refresh_candidate_count: assessments
            .iter()
            .filter(|assessment| assessment.suggested_action == NoteValueAction::Refresh)
            .count(),
        demote_candidate_count: assessments
            .iter()
            .filter(|assessment| assessment.suggested_action == NoteValueAction::Demote)
            .count(),
        promote_candidates,
        refresh_candidates,
        demote_candidates,
    };

    write_json_file(vault, &report_relative_path, &report)?;
    write_json_file(
        vault,
        &details_relative_path,
        &NoteValueDetails {
            assessments: assessments.clone(),
        },
    )?;
    write_summary_file(vault, &summary_relative_path, &report)?;

    Ok(report)
}

fn load_indexed_notes(vault: &ObsidianVault) -> NoteValueResult<Vec<IndexedNote>> {
    let entries = vault.list_tree()?;
    let mut notes = Vec::new();
    for entry in entries
        .into_iter()
        .filter(|entry| entry.kind == VaultEntryKind::Note)
    {
        let note = vault.read_note(&entry.path)?;
        let body_without_frontmatter = strip_frontmatter(&note.body);
        notes.push(IndexedNote {
            path: note.path.clone(),
            title: note.title.clone(),
            kind: extract_frontmatter_value(&note.body, "kind")
                .unwrap_or_else(|| "note".to_string()),
            provenance: extract_frontmatter_value(&note.body, "provenance")
                .unwrap_or_else(|| "human_authored".to_string()),
            review_state: extract_frontmatter_value(&note.body, "review_state")
                .unwrap_or_else(|| "canonical".to_string()),
            freshness_anchor: select_freshness_anchor(&note.body, note.modified_at),
            external_url_count: count_external_urls(body_without_frontmatter),
            outgoing_reference_targets: extract_reference_targets(body_without_frontmatter),
            resolved_references: Vec::new(),
        });
    }
    Ok(notes)
}

fn build_alias_index(notes: &[IndexedNote]) -> AliasIndex {
    let mut aliases = AliasIndex::default();
    for note in notes {
        insert_alias(
            &mut aliases.title_aliases,
            normalize_title_key(&note.title),
            &note.path,
        );
        insert_alias(
            &mut aliases.title_aliases,
            normalize_title_key(file_stem(&note.path)),
            &note.path,
        );
        insert_alias(
            &mut aliases.path_aliases,
            normalize_path_key(&note.path),
            &note.path,
        );
    }
    aliases
}

fn resolve_references(notes: &mut [IndexedNote], aliases: &AliasIndex) -> HashMap<String, usize> {
    let mut inbound_reference_counts = HashMap::<String, usize>::new();
    for note in notes {
        let mut resolved_references = HashSet::new();
        for target in &note.outgoing_reference_targets {
            let Some(path) = resolve_reference_target(target, aliases) else {
                continue;
            };
            if path == note.path {
                continue;
            }
            resolved_references.insert(path);
        }

        let mut resolved_references = resolved_references.into_iter().collect::<Vec<_>>();
        resolved_references.sort();
        for target_path in &resolved_references {
            *inbound_reference_counts
                .entry(target_path.clone())
                .or_default() += 1;
        }
        note.resolved_references = resolved_references;
    }

    inbound_reference_counts
}

fn assess_note_value(
    note: &IndexedNote,
    inbound_reference_count: usize,
    now: DateTime<Utc>,
) -> NoteValueCandidate {
    let age_days = note
        .freshness_anchor
        .map(|anchor| now.signed_duration_since(anchor).num_days().max(0));
    let freshness_status = classify_freshness(note, age_days);
    let freshness_score_bps = freshness_score_bps(note, age_days, freshness_status);
    let outgoing_reference_count = note.resolved_references.len();
    let value_score_bps = value_score_bps(note, inbound_reference_count, outgoing_reference_count);
    let overall_score_bps =
        (((value_score_bps as u64) * 55) + ((freshness_score_bps as u64) * 45)) / 100;
    let suggested_action = classify_action(
        note,
        freshness_status,
        inbound_reference_count,
        value_score_bps,
        age_days,
    );
    let reasons = collect_reasons(
        note,
        freshness_status,
        age_days,
        inbound_reference_count,
        outgoing_reference_count,
        note.external_url_count,
    );

    NoteValueCandidate {
        path: note.path.clone(),
        title: note.title.clone(),
        kind: note.kind.clone(),
        provenance: note.provenance.clone(),
        review_state: note.review_state.clone(),
        freshness_status,
        suggested_action,
        overall_score_bps: overall_score_bps as u32,
        freshness_score_bps,
        value_score_bps,
        age_days,
        inbound_reference_count,
        outgoing_reference_count,
        external_url_count: note.external_url_count,
        reasons,
    }
}

fn classify_freshness(note: &IndexedNote, age_days: Option<i64>) -> NoteFreshnessStatus {
    let Some(age_days) = age_days else {
        return NoteFreshnessStatus::Unknown;
    };
    let (aging_after_days, stale_after_days) = freshness_thresholds(note);
    if age_days <= aging_after_days {
        NoteFreshnessStatus::Current
    } else if age_days <= stale_after_days {
        NoteFreshnessStatus::Aging
    } else {
        NoteFreshnessStatus::Stale
    }
}

fn freshness_thresholds(note: &IndexedNote) -> (i64, i64) {
    if note.path.starts_with("Archive/") {
        return (365, 1095);
    }
    if note.review_state == "draft" {
        return (45, 120);
    }
    if note.kind == "source_capture"
        || note.provenance == "imported_source"
        || note.path.starts_with("Imported/")
    {
        return (180, 365);
    }
    if note.review_state == "unreviewed" {
        return (90, 240);
    }
    (180, 540)
}

fn freshness_score_bps(
    note: &IndexedNote,
    age_days: Option<i64>,
    freshness_status: NoteFreshnessStatus,
) -> u32 {
    let Some(age_days) = age_days else {
        return 3500;
    };
    let (_, stale_after_days) = freshness_thresholds(note);
    match freshness_status {
        NoteFreshnessStatus::Current if age_days <= 30 => 10_000,
        NoteFreshnessStatus::Current if age_days <= 90 => 8_500,
        NoteFreshnessStatus::Current => 7_000,
        NoteFreshnessStatus::Aging if age_days <= 270 => 5_200,
        NoteFreshnessStatus::Aging => 4_200,
        NoteFreshnessStatus::Stale if age_days <= stale_after_days * 2 => 2_200,
        NoteFreshnessStatus::Stale => 1_000,
        NoteFreshnessStatus::Unknown => 3_500,
    }
}

fn value_score_bps(
    note: &IndexedNote,
    inbound_reference_count: usize,
    outgoing_reference_count: usize,
) -> u32 {
    let mut score = 3_000_i32;

    if note.path.starts_with("Projects/") {
        score += 700;
    } else if note.path.starts_with("Areas/") {
        score += 600;
    } else if note.path.starts_with("Resources/") {
        score += 500;
    } else if note.path.starts_with("Archive/") {
        score -= 1_200;
    } else if note.path.starts_with("Imported/") {
        score -= 600;
    }

    match note.review_state.as_str() {
        "canonical" | "approved" => score += 1_000,
        "draft" => score -= 500,
        "unreviewed" => score -= 250,
        _ => {}
    }

    match note.provenance.as_str() {
        "human_authored" => score += 900,
        "human_reviewed_agent_assisted" => score += 500,
        "machine_derived" => score -= 700,
        "imported_source" => score -= 300,
        _ => {}
    }

    if note.kind == "source_capture" {
        score -= 300;
    }

    score += match inbound_reference_count {
        0 => 0,
        1 => 1_800,
        2 => 3_200,
        3 => 4_200,
        _ => 5_000,
    };
    score += (outgoing_reference_count.min(4) as i32) * 200;
    score += (note.external_url_count.min(3) as i32) * 150;

    score.clamp(0, 10_000) as u32
}

fn classify_action(
    note: &IndexedNote,
    freshness_status: NoteFreshnessStatus,
    inbound_reference_count: usize,
    value_score_bps: u32,
    age_days: Option<i64>,
) -> NoteValueAction {
    let reviewed = matches!(note.review_state.as_str(), "canonical" | "approved");
    let imported = note.kind == "source_capture"
        || note.provenance == "imported_source"
        || note.path.starts_with("Imported/");

    if freshness_status == NoteFreshnessStatus::Stale
        && (inbound_reference_count >= 1 || note.external_url_count > 0)
    {
        return NoteValueAction::Refresh;
    }

    if !reviewed
        && imported
        && freshness_status == NoteFreshnessStatus::Current
        && (age_days.is_some_and(|age_days| age_days <= 60) || note.external_url_count > 0)
    {
        return NoteValueAction::Promote;
    }

    if !reviewed && inbound_reference_count >= 2 && freshness_status != NoteFreshnessStatus::Stale {
        return NoteValueAction::Promote;
    }

    if !note.path.starts_with("Archive/")
        && imported
        && freshness_status == NoteFreshnessStatus::Stale
        && inbound_reference_count == 0
        && value_score_bps <= 3_500
    {
        return NoteValueAction::Demote;
    }

    NoteValueAction::Keep
}

fn collect_reasons(
    note: &IndexedNote,
    freshness_status: NoteFreshnessStatus,
    age_days: Option<i64>,
    inbound_reference_count: usize,
    outgoing_reference_count: usize,
    external_url_count: usize,
) -> Vec<String> {
    let mut reasons = Vec::new();

    if let Some(age_days) = age_days {
        let (_, stale_after_days) = freshness_thresholds(note);
        match freshness_status {
            NoteFreshnessStatus::Current => {
                reasons.push(format!("freshness anchor is {age_days} days old"));
            }
            NoteFreshnessStatus::Aging => {
                reasons.push(format!("freshness anchor is {age_days} days old"));
            }
            NoteFreshnessStatus::Stale => {
                reasons.push(format!(
                    "freshness anchor is {age_days} days old and beyond the {stale_after_days}-day stale window"
                ));
            }
            NoteFreshnessStatus::Unknown => {}
        }
    } else {
        reasons
            .push("no reliable freshness anchor found in frontmatter or file metadata".to_string());
    }

    if inbound_reference_count > 0 {
        reasons.push(format!(
            "referenced by {inbound_reference_count} other note{}",
            if inbound_reference_count == 1 {
                ""
            } else {
                "s"
            }
        ));
    } else {
        reasons.push("not referenced by other visible notes".to_string());
    }

    if outgoing_reference_count > 0 {
        reasons.push(format!(
            "links to {outgoing_reference_count} other note{}",
            if outgoing_reference_count == 1 {
                ""
            } else {
                "s"
            }
        ));
    }

    if external_url_count > 0 {
        reasons.push(format!(
            "contains {external_url_count} external URL{}",
            if external_url_count == 1 { "" } else { "s" }
        ));
    }

    if note.path.starts_with("Imported/") {
        reasons.push("still lives in Imported/ rather than a curated canonical folder".to_string());
    }
    if note.review_state == "unreviewed" {
        reasons.push("has not been reviewed or promoted yet".to_string());
    }

    reasons.truncate(4);
    reasons
}

fn sorted_candidates(
    assessments: &[NoteValueCandidate],
    action: NoteValueAction,
) -> Vec<NoteValueCandidate> {
    let mut candidates = assessments
        .iter()
        .filter(|assessment| assessment.suggested_action == action)
        .cloned()
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| match action {
        NoteValueAction::Promote => right
            .inbound_reference_count
            .cmp(&left.inbound_reference_count)
            .then_with(|| right.overall_score_bps.cmp(&left.overall_score_bps))
            .then_with(|| left.path.cmp(&right.path)),
        NoteValueAction::Refresh => right
            .inbound_reference_count
            .cmp(&left.inbound_reference_count)
            .then_with(|| {
                right
                    .age_days
                    .unwrap_or_default()
                    .cmp(&left.age_days.unwrap_or_default())
            })
            .then_with(|| right.overall_score_bps.cmp(&left.overall_score_bps))
            .then_with(|| left.path.cmp(&right.path)),
        NoteValueAction::Demote => left
            .overall_score_bps
            .cmp(&right.overall_score_bps)
            .then_with(|| {
                right
                    .age_days
                    .unwrap_or_default()
                    .cmp(&left.age_days.unwrap_or_default())
            })
            .then_with(|| left.path.cmp(&right.path)),
        NoteValueAction::Keep => left.path.cmp(&right.path),
    });
    candidates.truncate(MAX_VALUE_CANDIDATES);
    candidates
}

fn write_json_file<T: Serialize>(
    vault: &ObsidianVault,
    relative_path: &std::path::Path,
    value: &T,
) -> NoteValueResult<()> {
    let resolved = vault.resolve_relative_path(relative_path)?;
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(resolved, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn write_summary_file(
    vault: &ObsidianVault,
    relative_path: &std::path::Path,
    report: &NoteValueReport,
) -> NoteValueResult<()> {
    let mut body = String::new();
    body.push_str("# Note Value Report\n\n");
    body.push_str(&format!(
        "Analyzed {} visible notes into `{}`.\n\n",
        report.note_count, report.enriched_root
    ));
    body.push_str("## Freshness\n\n");
    body.push_str(&format!(
        "- current: {}\n- aging: {}\n- stale: {}\n- unknown: {}\n\n",
        report.current_note_count,
        report.aging_note_count,
        report.stale_note_count,
        report.unknown_freshness_note_count
    ));
    body.push_str("## Suggested Actions\n\n");
    body.push_str(&format!(
        "- promote: {}\n- refresh: {}\n- demote: {}\n\n",
        report.promote_candidate_count,
        report.refresh_candidate_count,
        report.demote_candidate_count
    ));
    append_candidate_section(&mut body, "Promote Candidates", &report.promote_candidates);
    append_candidate_section(&mut body, "Refresh Candidates", &report.refresh_candidates);
    append_candidate_section(&mut body, "Demote Candidates", &report.demote_candidates);
    body.push_str("## Artifacts\n\n");
    body.push_str(&format!(
        "- report: `{}`\n- details: `{}`\n",
        report.report_path, report.details_path
    ));
    vault.write_text_file(relative_path, &body)?;
    Ok(())
}

fn append_candidate_section(body: &mut String, heading: &str, candidates: &[NoteValueCandidate]) {
    body.push_str(&format!("## {heading}\n\n"));
    if candidates.is_empty() {
        body.push_str("No candidates in this batch.\n\n");
        return;
    }
    for candidate in candidates.iter().take(10) {
        body.push_str(&format!(
            "- `{}` — {}%, {:?}, {} inbound refs, {} outbound refs{}\n",
            candidate.path,
            candidate.overall_score_bps / 100,
            candidate.freshness_status,
            candidate.inbound_reference_count,
            candidate.outgoing_reference_count,
            candidate
                .age_days
                .map(|age_days| format!(", {age_days} days old"))
                .unwrap_or_default(),
        ));
        for reason in &candidate.reasons {
            body.push_str(&format!("  - {reason}\n"));
        }
    }
    body.push('\n');
}

fn select_freshness_anchor(
    body: &str,
    modified_at: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    let kind = extract_frontmatter_value(body, "kind").unwrap_or_else(|| "note".to_string());
    let provenance = extract_frontmatter_value(body, "provenance")
        .unwrap_or_else(|| "human_authored".to_string());
    let imported = kind == "source_capture" || provenance == "imported_source";

    let preferred_fields = if imported {
        [
            "source_updated_at",
            "updated",
            "source_created_at",
            "created",
        ]
    } else {
        ["vault_touched_at", "updated", "vault_created_at", "created"]
    };

    for key in preferred_fields {
        if let Some(value) = extract_frontmatter_value(body, key) {
            if let Some(parsed) = parse_note_datetime(&value) {
                return Some(parsed);
            }
        }
    }

    modified_at
}

fn parse_note_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%A, %-d %B %Y at %H:%M:%S")
                .ok()
                .map(|datetime| Utc.from_utc_datetime(&datetime))
        })
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%A, %d %B %Y at %H:%M:%S")
                .ok()
                .map(|datetime| Utc.from_utc_datetime(&datetime))
        })
}

fn extract_reference_targets(body: &str) -> Vec<String> {
    let mut targets = extract_wikilink_targets(body);
    targets.extend(extract_markdown_note_targets(body));
    targets
}

fn count_external_urls(body: &str) -> usize {
    let mut urls = HashSet::new();
    for token in body.split_whitespace() {
        let candidate = token.trim_matches(|character: char| {
            matches!(
                character,
                '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | ',' | '.' | ';'
            )
        });
        if candidate.starts_with("http://") || candidate.starts_with("https://") {
            urls.insert(candidate.to_string());
        }
    }
    urls.len()
}

fn extract_wikilink_targets(body: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut cursor = body;
    while let Some(start) = cursor.find("[[") {
        let after = &cursor[start + 2..];
        let Some(end) = after.find("]]") else {
            break;
        };
        let target = after[..end].trim();
        if !target.is_empty() {
            targets.push(target.to_string());
        }
        cursor = &after[end + 2..];
    }
    targets
}

fn extract_markdown_note_targets(body: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut cursor = body;
    while let Some(start) = cursor.find("](") {
        let after = &cursor[start + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        let target = after[..end].trim();
        if might_be_note_link(target) {
            targets.push(target.to_string());
        }
        cursor = &after[end + 1..];
    }
    targets
}

fn might_be_note_link(target: &str) -> bool {
    let cleaned = target.trim().trim_matches('<').trim_matches('>');
    !cleaned.contains("://")
        && !cleaned.starts_with("mailto:")
        && (cleaned.contains(".md") || !cleaned.contains('/'))
}

fn resolve_reference_target(target: &str, aliases: &AliasIndex) -> Option<String> {
    let cleaned = clean_reference_target(target)?;

    if cleaned.contains('/') || cleaned.ends_with(".md") {
        let path_key = normalize_path_key(&cleaned);
        if let Some(path) = resolve_unique_alias(&aliases.path_aliases, &path_key) {
            return Some(path);
        }
        let basename = cleaned.rsplit('/').next().unwrap_or(&cleaned);
        let basename_key = normalize_title_key(strip_note_extension(basename));
        if let Some(path) = resolve_unique_alias(&aliases.title_aliases, &basename_key) {
            return Some(path);
        }
    }

    let title_key = normalize_title_key(strip_note_extension(&cleaned));
    resolve_unique_alias(&aliases.title_aliases, &title_key)
}

fn clean_reference_target(target: &str) -> Option<String> {
    let mut cleaned = target
        .trim()
        .trim_matches('<')
        .trim_matches('>')
        .to_string();
    if cleaned.is_empty() || cleaned.contains("://") || cleaned.starts_with("mailto:") {
        return None;
    }

    if let Some(index) = cleaned.find(".md") {
        cleaned.truncate(index + 3);
    }
    if let Some((value, _)) = cleaned.split_once('|') {
        cleaned = value.to_string();
    }
    if let Some((value, _)) = cleaned.split_once('#') {
        cleaned = value.to_string();
    }

    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}

fn resolve_unique_alias(aliases: &HashMap<String, Vec<String>>, key: &str) -> Option<String> {
    let matches = aliases.get(key)?;
    if matches.len() == 1 {
        matches.first().cloned()
    } else {
        None
    }
}

fn insert_alias(aliases: &mut HashMap<String, Vec<String>>, key: String, path: &str) {
    if key.is_empty() {
        return;
    }
    let paths = aliases.entry(key).or_default();
    if !paths.iter().any(|existing| existing == path) {
        paths.push(path.to_string());
    }
}

fn strip_frontmatter(body: &str) -> &str {
    split_frontmatter(body).map_or(body, |(_, remainder)| remainder)
}

fn normalize_title_key(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_path_key(value: &str) -> String {
    value
        .replace('\\', "/")
        .split('/')
        .filter_map(|segment| {
            let trimmed = strip_note_extension(segment.trim().trim_matches('.'));
            let normalized = normalize_title_key(trimmed);
            (!normalized.is_empty()).then_some(normalized)
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn strip_note_extension(value: &str) -> &str {
    value.strip_suffix(".md").unwrap_or(value)
}

fn file_stem(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".md")
}

#[cfg(test)]
mod tests {
    use super::{NoteValueAction, analyze_note_value};
    use crate::vault::ObsidianVault;
    use chrono::{Duration, Utc};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn writes_note_value_report_with_promote_refresh_and_demote_candidates() {
        let root = temp_dir("note-value");
        let vault = ObsidianVault::from_root(&root);

        let fresh_source_updated = (Utc::now() - Duration::days(7)).to_rfc3339();
        let stale_source_updated = (Utc::now() - Duration::days(900)).to_rfc3339();

        vault
            .write_text_file(
                PathBuf::from("Imported/Apple Notes/iCloud/Source Fresh.md").as_path(),
                &format!(
                    "---\nkind: \"source_capture\"\nprovenance: \"imported_source\"\nreview_state: \"unreviewed\"\nsource_updated_at: \"{}\"\n---\n\nFresh imported source.\n",
                    fresh_source_updated
                ),
            )
            .expect("fresh source note");
        vault
            .write_text_file(
                PathBuf::from("Imported/Apple Notes/iCloud/Source Stale.md").as_path(),
                &format!(
                    "---\nkind: \"source_capture\"\nprovenance: \"imported_source\"\nreview_state: \"unreviewed\"\nsource_updated_at: \"{}\"\n---\n\nStale imported source.\n",
                    stale_source_updated
                ),
            )
            .expect("stale source note");
        vault
            .write_text_file(
                PathBuf::from("Imported/Apple Notes/iCloud/Source Dormant.md").as_path(),
                &format!(
                    "---\nkind: \"source_capture\"\nprovenance: \"imported_source\"\nreview_state: \"unreviewed\"\nsource_updated_at: \"{}\"\n---\n\nDormant imported source.\n",
                    stale_source_updated
                ),
            )
            .expect("dormant source note");

        let summary = vault
            .create_note(None, "Working summary")
            .expect("summary note");
        vault
            .save_note(
                &summary.path,
                "# Working summary\n\nUse [[Source Fresh]] for the active context and [[Source Stale]] for historical context.\n",
            )
            .expect("save summary");
        let project_note = vault
            .create_note(Some("Projects"), "Project plan")
            .expect("project note");
        vault
            .save_note(
                &project_note.path,
                "# Project plan\n\nPrimary reference: [[Source Fresh]].\n",
            )
            .expect("save project note");

        let report = analyze_note_value(&vault).expect("report should succeed");

        assert_eq!(report.note_count, 5);
        assert!(
            report
                .promote_candidates
                .iter()
                .any(|candidate| candidate.path.ends_with("Source Fresh.md")
                    && candidate.suggested_action == NoteValueAction::Promote)
        );
        assert!(
            report
                .refresh_candidates
                .iter()
                .any(|candidate| candidate.path.ends_with("Source Stale.md")
                    && candidate.suggested_action == NoteValueAction::Refresh)
        );
        assert!(
            report
                .demote_candidates
                .iter()
                .any(|candidate| candidate.path.ends_with("Source Dormant.md")
                    && candidate.suggested_action == NoteValueAction::Demote)
        );
        assert!(root.join(PathBuf::from(report.report_path)).exists());
        assert!(root.join(PathBuf::from(report.details_path)).exists());
        assert!(root.join(PathBuf::from(report.summary_path)).exists());
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time after unix epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("{label}-{nonce}"));
        fs::create_dir_all(&directory).expect("temp directory should be created");
        directory
    }
}
