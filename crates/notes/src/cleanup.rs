use std::collections::{HashMap, HashSet};
use std::fs;

use crate::vault::{ObsidianVault, VaultError, VaultPipelineStage, path_to_relative_string};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_SIMILARITY_CANDIDATES: usize = 64;
const MAX_MERGE_SUGGESTIONS: usize = 24;

#[derive(Debug, Error)]
pub enum NoteCleanupError {
    #[error(transparent)]
    Vault(#[from] VaultError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type NoteCleanupResult<T> = Result<T, NoteCleanupError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteCleanupReport {
    pub run_id: String,
    pub enriched_root: String,
    pub report_path: String,
    pub note_count: usize,
    pub exact_duplicate_group_count: usize,
    pub similarity_candidate_count: usize,
    pub merge_suggestion_count: usize,
    pub exact_duplicates: Vec<ExactDuplicateGroup>,
    pub similarity_candidates: Vec<SimilarityCandidate>,
    pub merge_suggestions: Vec<MergeSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactDuplicateGroup {
    pub canonical_path: String,
    pub duplicate_paths: Vec<String>,
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimilarityCandidate {
    pub left_path: String,
    pub right_path: String,
    pub score_bps: u32,
    pub shared_tokens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeSuggestion {
    pub primary_path: String,
    pub secondary_path: String,
    pub score_bps: u32,
    pub rationale: String,
}

#[derive(Debug, Clone)]
struct IndexedNote {
    path: String,
    title: String,
    normalized_title: String,
    normalized_body: String,
    similarity_tokens: HashSet<String>,
}

pub fn analyze_note_cleanup(vault: &ObsidianVault) -> NoteCleanupResult<NoteCleanupReport> {
    vault.ensure_root()?;
    let indexed_notes = load_indexed_notes(vault)?;
    let exact_duplicates = collect_exact_duplicates(&indexed_notes);
    let similarity_candidates = collect_similarity_candidates(&indexed_notes, &exact_duplicates);
    let merge_suggestions = collect_merge_suggestions(&similarity_candidates, &exact_duplicates);

    let run = vault.prepare_pipeline_run(VaultPipelineStage::Enriched, "note-cleanup")?;
    let report_relative_path = run.relative_root.join("report.json");
    let report = NoteCleanupReport {
        run_id: run.run_id,
        enriched_root: path_to_relative_string(&run.relative_root),
        report_path: path_to_relative_string(&report_relative_path),
        note_count: indexed_notes.len(),
        exact_duplicate_group_count: exact_duplicates.len(),
        similarity_candidate_count: similarity_candidates.len(),
        merge_suggestion_count: merge_suggestions.len(),
        exact_duplicates,
        similarity_candidates,
        merge_suggestions,
    };

    let resolved = vault.resolve_relative_path(&report_relative_path)?;
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(resolved, serde_json::to_string_pretty(&report)?)?;
    Ok(report)
}

fn load_indexed_notes(vault: &ObsidianVault) -> NoteCleanupResult<Vec<IndexedNote>> {
    let entries = vault.list_tree()?;
    let mut notes = Vec::new();
    for entry in entries
        .into_iter()
        .filter(|entry| entry.kind == crate::vault::VaultEntryKind::Note)
    {
        let note = vault.read_note(&entry.path)?;
        let body_without_frontmatter = strip_frontmatter(&note.body);
        let normalized_title = normalize_text(&note.title);
        let normalized_body = normalize_text(body_without_frontmatter);
        let similarity_tokens = tokenize_similarity(&note.title, body_without_frontmatter);
        notes.push(IndexedNote {
            path: note.path,
            title: note.title,
            normalized_title,
            normalized_body,
            similarity_tokens,
        });
    }
    Ok(notes)
}

fn collect_exact_duplicates(notes: &[IndexedNote]) -> Vec<ExactDuplicateGroup> {
    let mut groups = HashMap::<String, Vec<&IndexedNote>>::new();
    for note in notes {
        if note.normalized_body.len() < 40 {
            continue;
        }
        groups
            .entry(note.normalized_body.clone())
            .or_default()
            .push(note);
    }

    let mut duplicates = groups
        .into_values()
        .filter(|group| group.len() > 1)
        .map(|group| {
            let mut paths = group
                .iter()
                .map(|note| note.path.clone())
                .collect::<Vec<String>>();
            paths.sort();
            let canonical_path = pick_primary_path(&paths, &paths[0]);
            ExactDuplicateGroup {
                duplicate_paths: paths
                    .iter()
                    .filter(|path| *path != &canonical_path)
                    .cloned()
                    .collect(),
                canonical_path,
                title: group[0].title.clone(),
                reason: "identical normalized body".to_string(),
            }
        })
        .collect::<Vec<_>>();
    duplicates.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));
    duplicates
}

fn collect_similarity_candidates(
    notes: &[IndexedNote],
    exact_duplicates: &[ExactDuplicateGroup],
) -> Vec<SimilarityCandidate> {
    let exact_duplicate_pairs = exact_duplicate_pair_set(exact_duplicates);
    let mut shared_counts = HashMap::<(usize, usize), usize>::new();
    let mut token_index = HashMap::<String, Vec<usize>>::new();

    for (index, note) in notes.iter().enumerate() {
        for token in &note.similarity_tokens {
            token_index.entry(token.clone()).or_default().push(index);
        }
    }

    for indices in token_index.values() {
        for left in 0..indices.len() {
            for right in (left + 1)..indices.len() {
                let pair = ordered_pair(indices[left], indices[right]);
                *shared_counts.entry(pair).or_default() += 1;
            }
        }
    }

    for left in 0..notes.len() {
        for right in (left + 1)..notes.len() {
            if notes[left].normalized_title == notes[right].normalized_title
                && !notes[left].normalized_title.is_empty()
            {
                shared_counts.entry((left, right)).or_insert(2);
            }
        }
    }

    let mut candidates = shared_counts
        .into_iter()
        .filter_map(|((left, right), overlap_count)| {
            if overlap_count < 2 {
                return None;
            }
            let left_note = &notes[left];
            let right_note = &notes[right];
            if exact_duplicate_pairs.contains(&(left_note.path.clone(), right_note.path.clone())) {
                return None;
            }

            let score =
                jaccard_similarity(&left_note.similarity_tokens, &right_note.similarity_tokens);
            if score < 0.45 {
                return None;
            }

            let shared_tokens =
                shared_token_list(&left_note.similarity_tokens, &right_note.similarity_tokens);
            Some(SimilarityCandidate {
                left_path: left_note.path.clone(),
                right_path: right_note.path.clone(),
                score_bps: (score * 10_000.0).round() as u32,
                shared_tokens,
            })
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .score_bps
            .cmp(&left.score_bps)
            .then_with(|| left.left_path.cmp(&right.left_path))
    });
    candidates.truncate(MAX_SIMILARITY_CANDIDATES);
    candidates
}

fn collect_merge_suggestions(
    candidates: &[SimilarityCandidate],
    exact_duplicates: &[ExactDuplicateGroup],
) -> Vec<MergeSuggestion> {
    let exact_pairs = exact_duplicate_pair_set(exact_duplicates);
    let mut suggestions = Vec::new();

    for group in exact_duplicates {
        for duplicate in &group.duplicate_paths {
            let (primary_path, secondary_path) =
                ordered_merge_pair(&group.canonical_path, duplicate);
            suggestions.push(MergeSuggestion {
                primary_path,
                secondary_path,
                score_bps: 10_000,
                rationale: "exact duplicate body".to_string(),
            });
        }
    }

    for candidate in candidates {
        let same_title_bias = file_stem(&candidate.left_path) == file_stem(&candidate.right_path);
        if candidate.score_bps < 7000 && !(same_title_bias && candidate.score_bps >= 5500) {
            continue;
        }

        let (primary_path, secondary_path) =
            ordered_merge_pair(&candidate.left_path, &candidate.right_path);
        let rationale = if exact_pairs.contains(&(primary_path.clone(), secondary_path.clone())) {
            "exact duplicate body".to_string()
        } else if same_title_bias {
            "same title with high content overlap".to_string()
        } else {
            "high token overlap".to_string()
        };
        suggestions.push(MergeSuggestion {
            primary_path,
            secondary_path,
            score_bps: candidate.score_bps,
            rationale,
        });
    }

    suggestions.sort_by(|left, right| {
        right
            .score_bps
            .cmp(&left.score_bps)
            .then_with(|| left.primary_path.cmp(&right.primary_path))
            .then_with(|| left.secondary_path.cmp(&right.secondary_path))
    });
    suggestions.dedup_by(|left, right| {
        left.primary_path == right.primary_path && left.secondary_path == right.secondary_path
    });
    suggestions.truncate(MAX_MERGE_SUGGESTIONS);
    suggestions
}

fn exact_duplicate_pair_set(groups: &[ExactDuplicateGroup]) -> HashSet<(String, String)> {
    let mut pairs = HashSet::new();
    for group in groups {
        for duplicate in &group.duplicate_paths {
            pairs.insert((group.canonical_path.clone(), duplicate.clone()));
            pairs.insert((duplicate.clone(), group.canonical_path.clone()));
        }
    }
    pairs
}

fn strip_frontmatter(body: &str) -> &str {
    let Some(trimmed) = body.strip_prefix("---\n") else {
        return body;
    };
    let Some(end) = trimmed.find("\n---\n") else {
        return body;
    };
    &trimmed[end + 5..]
}

fn normalize_text(value: &str) -> String {
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

fn tokenize_similarity(title: &str, body: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();
    for token in normalize_text(&format!(
        "{title} {}",
        body.chars().take(1200).collect::<String>()
    ))
    .split_whitespace()
    {
        if token.len() >= 4 && !STOP_WORDS.contains(&token) {
            tokens.insert(token.to_string());
        }
    }
    tokens
}

fn jaccard_similarity(left: &HashSet<String>, right: &HashSet<String>) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let intersection = left.intersection(right).count() as f32;
    let union = left.union(right).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn shared_token_list(left: &HashSet<String>, right: &HashSet<String>) -> Vec<String> {
    let mut tokens = left
        .intersection(right)
        .map(|token| token.to_string())
        .collect::<Vec<_>>();
    tokens.sort();
    tokens.truncate(8);
    tokens
}

fn ordered_pair(left: usize, right: usize) -> (usize, usize) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn ordered_merge_pair(left: &str, right: &str) -> (String, String) {
    let preferred = pick_primary_path(&[left.to_string(), right.to_string()], left);
    if preferred == left {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

fn pick_primary_path(paths: &[String], fallback: &str) -> String {
    paths
        .iter()
        .min_by(|left, right| compare_note_priority(left, right))
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

fn compare_note_priority(left: &str, right: &str) -> std::cmp::Ordering {
    inbox_bias(left)
        .cmp(&inbox_bias(right))
        .then_with(|| left.matches('/').count().cmp(&right.matches('/').count()))
        .then_with(|| left.len().cmp(&right.len()))
        .then_with(|| left.cmp(right))
}

fn inbox_bias(path: &str) -> usize {
    if path.starts_with("Inbox/") { 1 } else { 0 }
}

fn file_stem(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".md")
}

const STOP_WORDS: &[&str] = &[
    "this", "that", "with", "from", "have", "your", "into", "about", "would", "there", "their",
    "they", "them", "when", "what", "where", "which", "shall", "could", "should", "notes",
];

#[cfg(test)]
mod tests {
    use super::analyze_note_cleanup;
    use crate::vault::ObsidianVault;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn writes_cleanup_report_with_duplicate_and_merge_candidates() {
        let root = temp_dir("note-cleanup");
        let vault = ObsidianVault::from_root(&root);
        let first = vault
            .create_note(None, "Founder market map")
            .expect("first");
        let second = vault
            .create_note(Some("Projects"), "Founder market map copy")
            .expect("second");
        let third = vault
            .create_note(Some("Resources"), "Go to market ideas")
            .expect("third");

        vault
            .save_note(
                &first.path,
                "# Founder market map\n\nNorthwind and Orbit share the same ICP and outreach motion.\n",
            )
            .expect("save first");
        vault
            .save_note(
                &second.path,
                "# Founder market map\n\nNorthwind and Orbit share the same ICP and outreach motion.\n",
            )
            .expect("save second");
        vault
            .save_note(
                &third.path,
                "# Go to market ideas\n\nOrbit and Northwind share the same ICP with overlapping outreach experiments.\n",
            )
            .expect("save third");

        let report = analyze_note_cleanup(&vault).expect("report should succeed");

        assert_eq!(report.note_count, 3);
        assert_eq!(report.exact_duplicate_group_count, 1);
        assert!(!report.merge_suggestions.is_empty());
        assert!(root.join(PathBuf::from(report.report_path)).exists());
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
