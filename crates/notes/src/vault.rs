//! Obsidian-compatible vault management.
//!
//! Note tree, note CRUD, markdown import, pipeline stages (raw/enriched),
//! YAML frontmatter with vault freshness tracking.
//!
//! Extracted from `prio-vault` in the Outcome Workbench. This is the
//! canonical implementation — apps depend on this crate.

use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const STANDARD_VAULT_DIRECTORIES: &[&str] = &["Inbox", "Projects", "Areas", "Resources", "Archive"];
const DEFAULT_NOTE_DIRECTORY: &str = "Inbox";

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("home directory is not available")]
    HomeDirectoryUnavailable,
    #[error("invalid relative path: {0}")]
    InvalidRelativePath(String),
    #[error("invalid note title")]
    InvalidTitle,
    #[error("source directory does not exist: {0}")]
    SourceDirectoryMissing(String),
    #[error("path is not a directory: {0}")]
    NotADirectory(String),
    #[error("path is not a markdown note: {0}")]
    NotMarkdownNote(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type VaultResult<T> = Result<T, VaultError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultEntryKind {
    Directory,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultTreeEntry {
    pub path: String,
    pub name: String,
    pub kind: VaultEntryKind,
    pub depth: usize,
    pub modified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultNote {
    pub path: String,
    pub title: String,
    pub body: String,
    pub modified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultImportReport {
    pub imported_root: String,
    pub note_count: usize,
    pub attachment_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedImportRoot {
    pub relative_root: PathBuf,
    pub absolute_root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultPipelineStage {
    Raw,
    Enriched,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPipelineRun {
    pub stage: VaultPipelineStage,
    pub source: String,
    pub run_id: String,
    pub relative_root: PathBuf,
    pub absolute_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ObsidianVault {
    root: PathBuf,
}

impl ObsidianVault {
    #[must_use]
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_in_home() -> VaultResult<Self> {
        let home = env::var_os("HOME").ok_or(VaultError::HomeDirectoryUnavailable)?;
        Ok(Self::from_root(PathBuf::from(home).join("Notes")))
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure_root(&self) -> VaultResult<()> {
        fs::create_dir_all(&self.root)?;
        for directory in STANDARD_VAULT_DIRECTORIES {
            fs::create_dir_all(self.root.join(directory))?;
        }
        fs::create_dir_all(
            self.root
                .join(stage_directory_name(VaultPipelineStage::Raw)),
        )?;
        fs::create_dir_all(
            self.root
                .join(stage_directory_name(VaultPipelineStage::Enriched)),
        )?;
        Ok(())
    }

    pub fn list_tree(&self) -> VaultResult<Vec<VaultTreeEntry>> {
        self.ensure_root()?;
        let mut entries = Vec::new();
        self.collect_tree(&self.root, Path::new(""), 0, &mut entries)?;
        Ok(entries)
    }

    pub fn read_note(&self, relative_path: &str) -> VaultResult<VaultNote> {
        let resolved = self.resolve_note_path(relative_path)?;
        let body = fs::read_to_string(&resolved)?;
        let relative = resolved
            .strip_prefix(&self.root)
            .map_err(|_| VaultError::InvalidRelativePath(relative_path.to_string()))?;
        Ok(VaultNote {
            path: path_to_relative_string(relative),
            title: title_from_path(&resolved),
            body,
            modified_at: metadata_modified_at(&fs::metadata(&resolved)?),
        })
    }

    pub fn save_note(&self, relative_path: &str, body: &str) -> VaultResult<VaultNote> {
        self.ensure_root()?;
        let resolved = self.resolve_note_path(relative_path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }
        let existing_body = fs::read_to_string(&resolved).ok();
        let now = Utc::now();
        let body = with_vault_freshness(
            body,
            existing_body.as_deref(),
            NoteFreshnessStamp {
                created_at: now,
                touched_at: now,
            },
        );
        fs::write(&resolved, body)?;
        self.read_note(relative_path)
    }

    pub fn create_note(&self, parent_dir: Option<&str>, title: &str) -> VaultResult<VaultNote> {
        self.ensure_root()?;
        let default_parent = parent_dir
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_NOTE_DIRECTORY);
        let parent = self.resolve_relative_path(Path::new(default_parent))?;
        fs::create_dir_all(&parent)?;

        let relative_dir = normalize_relative_path(Path::new(default_parent))?;
        let relative_path = self.allocate_note_path(&relative_dir, title)?;
        let title = title.trim();
        let body = format!("# {title}\n\n");
        self.save_note(&path_to_relative_string(&relative_path), &body)
    }

    pub fn move_note(
        &self,
        from_relative_path: &str,
        to_relative_path: &str,
    ) -> VaultResult<VaultNote> {
        self.ensure_root()?;
        let from = self.resolve_note_path(from_relative_path)?;
        let to_relative =
            ensure_markdown_extension(normalize_relative_path(Path::new(to_relative_path))?);
        let to = self.root.join(&to_relative);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(from, &to)?;
        self.read_note(&path_to_relative_string(&to_relative))
    }

    pub fn import_markdown_tree(&self, source_dir: &str) -> VaultResult<VaultImportReport> {
        self.ensure_root()?;
        let source = PathBuf::from(source_dir);
        if !source.exists() {
            return Err(VaultError::SourceDirectoryMissing(source_dir.to_string()));
        }
        if !source.is_dir() {
            return Err(VaultError::NotADirectory(source_dir.to_string()));
        }

        let base_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .map_or_else(|| "import".to_string(), str::to_string);
        let target = self.prepare_import_root(&base_name)?;

        let mut note_count = 0;
        let mut attachment_count = 0;
        copy_tree(
            &source,
            &target.absolute_root,
            &mut note_count,
            &mut attachment_count,
        )?;

        Ok(VaultImportReport {
            imported_root: path_to_relative_string(&target.relative_root),
            note_count,
            attachment_count,
        })
    }

    pub fn prepare_import_root(&self, source_name: &str) -> VaultResult<PreparedImportRoot> {
        self.ensure_root()?;
        let import_dir_name = unique_import_directory_name(&self.root, source_name);
        let relative_root = Path::new("Imported").join(import_dir_name);
        let absolute_root = self.root.join(&relative_root);
        fs::create_dir_all(&absolute_root)?;
        Ok(PreparedImportRoot {
            relative_root,
            absolute_root,
        })
    }

    pub fn prepare_pipeline_run(
        &self,
        stage: VaultPipelineStage,
        source_name: &str,
    ) -> VaultResult<PreparedPipelineRun> {
        self.ensure_root()?;
        let source = sanitize_directory_name(source_name)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-");
        let source = if source.is_empty() {
            "source".to_string()
        } else {
            source.to_ascii_lowercase()
        };
        let base_run_id = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let stage_root = self.root.join(stage_directory_name(stage)).join(&source);
        let run_id = unique_run_id(&stage_root, &base_run_id);
        let relative_root = Path::new(stage_directory_name(stage))
            .join(&source)
            .join(&run_id);
        let absolute_root = self.root.join(&relative_root);
        fs::create_dir_all(&absolute_root)?;
        Ok(PreparedPipelineRun {
            stage,
            source,
            run_id,
            relative_root,
            absolute_root,
        })
    }

    pub fn allocate_note_path(&self, relative_dir: &Path, title: &str) -> VaultResult<PathBuf> {
        let filename = sanitized_note_filename(title).ok_or(VaultError::InvalidTitle)?;
        Ok(unique_note_path(&self.root, relative_dir, &filename))
    }

    pub fn resolve_relative_path(&self, relative_path: &Path) -> VaultResult<PathBuf> {
        Ok(self.root.join(normalize_relative_path(relative_path)?))
    }

    pub fn write_text_file(&self, relative_path: &Path, body: &str) -> VaultResult<()> {
        self.ensure_root()?;
        let resolved = self.resolve_relative_path(relative_path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(resolved, body)?;
        Ok(())
    }

    fn resolve_note_path(&self, relative_path: &str) -> VaultResult<PathBuf> {
        let normalized = normalize_relative_path(Path::new(relative_path))?;
        if normalized.as_os_str().is_empty() {
            return Err(VaultError::NotMarkdownNote(relative_path.to_string()));
        }
        Ok(self.root.join(ensure_markdown_extension(normalized)))
    }

    #[allow(clippy::self_only_used_in_recursion)]
    fn collect_tree(
        &self,
        directory: &Path,
        relative_directory: &Path,
        depth: usize,
        entries: &mut Vec<VaultTreeEntry>,
    ) -> VaultResult<()> {
        let mut children = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
        children.retain(|entry| !is_hidden_name(&entry.file_name()));
        children.sort_by(|left, right| compare_dir_entries(&left.path(), &right.path()));

        for child in children {
            let path = child.path();
            let child_name = child.file_name().to_string_lossy().to_string();
            let relative_path = relative_directory.join(&child_name);

            if path.is_dir() {
                entries.push(VaultTreeEntry {
                    path: path_to_relative_string(&relative_path),
                    name: child_name.clone(),
                    kind: VaultEntryKind::Directory,
                    depth,
                    modified_at: metadata_modified_at(&child.metadata()?),
                });
                self.collect_tree(&path, &relative_path, depth + 1, entries)?;
            } else if is_markdown_path(&path) {
                entries.push(VaultTreeEntry {
                    path: path_to_relative_string(&relative_path),
                    name: child_name,
                    kind: VaultEntryKind::Note,
                    depth,
                    modified_at: metadata_modified_at(&child.metadata()?),
                });
            }
        }

        Ok(())
    }
}

pub fn path_to_relative_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

const MAX_PATH_COMPONENT_BYTES: usize = 255;
const MAX_NOTE_ASSET_SUFFIX_BYTES: usize = ".assets".len();

pub fn sanitize_directory_name(value: &str) -> String {
    truncate_path_component(
        &value
        .trim()
        .chars()
        .filter_map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => Some('-'),
            value if value.is_control() => None,
            value => Some(value),
        })
        .collect::<String>()
        .trim()
        .trim_matches('.'),
        MAX_PATH_COMPONENT_BYTES,
    )
}

pub fn is_markdown_path(path: &Path) -> bool {
    path.extension().and_then(|value| value.to_str()) == Some("md")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NoteFreshnessStamp {
    created_at: DateTime<Utc>,
    touched_at: DateTime<Utc>,
}

fn normalize_relative_path(path: &Path) -> VaultResult<PathBuf> {
    if path.is_absolute() {
        return Err(VaultError::InvalidRelativePath(
            path.to_string_lossy().to_string(),
        ));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(VaultError::InvalidRelativePath(
                    path.to_string_lossy().to_string(),
                ));
            }
        }
    }

    Ok(normalized)
}

fn ensure_markdown_extension(path: PathBuf) -> PathBuf {
    if path.extension().and_then(|value| value.to_str()) == Some("md") {
        return path;
    }

    let mut with_extension = path;
    with_extension.set_extension("md");
    with_extension
}

fn is_hidden_name(name: &std::ffi::OsStr) -> bool {
    name.to_str().is_some_and(|value| value.starts_with('.'))
}

fn with_vault_freshness(
    body: &str,
    existing_body: Option<&str>,
    stamp: NoteFreshnessStamp,
) -> String {
    let existing_created = existing_body
        .and_then(|body| extract_frontmatter_value(body, "vault_created_at"))
        .or_else(|| extract_frontmatter_value(body, "vault_created_at"));
    let created_at = existing_created.unwrap_or_else(|| stamp.created_at.to_rfc3339());
    let touched_at = stamp.touched_at.to_rfc3339();

    if let Some((frontmatter, remainder)) = split_frontmatter(body) {
        let frontmatter = upsert_frontmatter_pair(frontmatter, "vault_created_at", &created_at);
        let frontmatter = upsert_frontmatter_pair(&frontmatter, "vault_touched_at", &touched_at);
        return format!("---\n{frontmatter}\n---\n{remainder}");
    }

    format!(
        "---\nvault_created_at: {}\nvault_touched_at: {}\n---\n\n{}",
        yaml_quote(&created_at),
        yaml_quote(&touched_at),
        body.trim_start_matches('\n')
    )
}

pub fn split_frontmatter(body: &str) -> Option<(&str, &str)> {
    let trimmed = body.strip_prefix("---\n")?;
    let end = trimmed.find("\n---\n")?;
    let frontmatter = &trimmed[..end];
    let remainder = &trimmed[end + 5..];
    Some((frontmatter, remainder))
}

pub fn upsert_frontmatter_pair(frontmatter: &str, key: &str, value: &str) -> String {
    let mut lines = frontmatter
        .lines()
        .map(str::to_string)
        .collect::<Vec<String>>();
    let new_line = format!("{key}: {}", yaml_quote(value));

    if let Some(index) = lines
        .iter()
        .position(|line| line.starts_with(&format!("{key}: ")))
    {
        lines[index] = new_line;
    } else {
        lines.push(new_line);
    }

    lines.join("\n")
}

pub fn extract_frontmatter_value(body: &str, key: &str) -> Option<String> {
    let (frontmatter, _) = split_frontmatter(body)?;
    frontmatter
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}: ")))
        .map(|value| yaml_unquote(value.trim()))
}

fn metadata_modified_at(metadata: &fs::Metadata) -> Option<DateTime<Utc>> {
    metadata.modified().ok().map(DateTime::<Utc>::from)
}

fn title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

fn sanitized_note_filename(title: &str) -> Option<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut output = String::with_capacity(trimmed.len() + 3);
    let mut last_was_space = false;
    for character in trimmed.chars() {
        match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => {
                if !last_was_space && !output.is_empty() {
                    output.push('-');
                    last_was_space = true;
                }
            }
            value if value.is_control() => {}
            value if value.is_whitespace() => {
                if !last_was_space && !output.is_empty() {
                    output.push(' ');
                    last_was_space = true;
                }
            }
            value => {
                output.push(value);
                last_was_space = false;
            }
        }
    }

    let cleaned = truncate_path_component(
        output.trim().trim_matches('.'),
        max_note_stem_bytes("md", ""),
    );
    if cleaned.is_empty() {
        return None;
    }

    Some(format!("{cleaned}.md"))
}

fn unique_note_path(root: &Path, relative_dir: &Path, filename: &str) -> PathBuf {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Note");
    let extension = Path::new(filename)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("md");
    let filename = build_note_filename(stem, extension, "");
    let candidate = relative_dir.join(&filename);
    if !root.join(&candidate).exists() {
        return candidate;
    }

    let mut index = 2;
    loop {
        let suffix = format!(" {index}");
        let candidate = relative_dir.join(build_note_filename(stem, extension, &suffix));
        if !root.join(&candidate).exists() {
            return candidate;
        }
        index += 1;
    }
}

fn build_note_filename(stem: &str, extension: &str, suffix: &str) -> String {
    let bounded_stem = truncate_path_component(stem, max_note_stem_bytes(extension, suffix));
    if extension.is_empty() {
        format!("{bounded_stem}{suffix}")
    } else {
        format!("{bounded_stem}{suffix}.{extension}")
    }
}

fn max_note_stem_bytes(extension: &str, suffix: &str) -> usize {
    let asset_budget = MAX_PATH_COMPONENT_BYTES
        .saturating_sub(MAX_NOTE_ASSET_SUFFIX_BYTES)
        .saturating_sub(suffix.len());
    let file_budget = MAX_PATH_COMPONENT_BYTES
        .saturating_sub(suffix.len())
        .saturating_sub(if extension.is_empty() {
            0
        } else {
            extension.len() + 1
        });
    asset_budget.min(file_budget)
}

fn truncate_path_component(value: &str, max_bytes: usize) -> String {
    let trimmed = value.trim().trim_matches('.');
    if trimmed.len() <= max_bytes {
        return trimmed.to_string();
    }

    let mut end = 0;
    for (index, character) in trimmed.char_indices() {
        let next = index + character.len_utf8();
        if next > max_bytes {
            break;
        }
        end = next;
    }

    trimmed[..end].trim().trim_matches('.').to_string()
}

fn unique_import_directory_name(root: &Path, source_name: &str) -> String {
    let imported_root = root.join("Imported");
    let base_name = sanitize_directory_name(source_name);
    let base = if base_name.is_empty() {
        "import".to_string()
    } else {
        base_name
    };
    if !imported_root.join(&base).exists() {
        return base;
    }

    let mut index = 2;
    loop {
        let candidate = format!("{base} {index}");
        if !imported_root.join(&candidate).exists() {
            return candidate;
        }
        index += 1;
    }
}

fn compare_dir_entries(left: &Path, right: &Path) -> std::cmp::Ordering {
    let left_is_dir = left.is_dir();
    let right_is_dir = right.is_dir();
    right_is_dir.cmp(&left_is_dir).then_with(|| {
        left.to_string_lossy()
            .to_lowercase()
            .cmp(&right.to_string_lossy().to_lowercase())
    })
}

fn stage_directory_name(stage: VaultPipelineStage) -> &'static str {
    match stage {
        VaultPipelineStage::Raw => ".raw",
        VaultPipelineStage::Enriched => ".enriched",
    }
}

fn unique_run_id(stage_root: &Path, base_run_id: &str) -> String {
    if !stage_root.join(base_run_id).exists() {
        return base_run_id.to_string();
    }

    let mut index = 2;
    loop {
        let candidate = format!("{base_run_id}-{index}");
        if !stage_root.join(&candidate).exists() {
            return candidate;
        }
        index += 1;
    }
}

pub fn yaml_quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    )
}

pub fn yaml_unquote(value: &str) -> String {
    let trimmed = value.trim();
    let Some(unquoted) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return trimmed.to_string();
    };

    let mut output = String::with_capacity(unquoted.len());
    let mut chars = unquoted.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }

        match chars.next() {
            Some('\\') => output.push('\\'),
            Some('"') => output.push('"'),
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some(other) => output.push(other),
            None => break,
        }
    }
    output
}

fn copy_tree(
    source: &Path,
    destination: &Path,
    note_count: &mut usize,
    attachment_count: &mut usize,
) -> VaultResult<()> {
    let mut children = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
    children.retain(|entry| !is_hidden_name(&entry.file_name()));

    for child in children {
        let child_path = child.path();
        let target_path = destination.join(child.file_name());

        if child_path.is_dir() {
            fs::create_dir_all(&target_path)?;
            copy_tree(&child_path, &target_path, note_count, attachment_count)?;
        } else if child_path.is_file() {
            fs::copy(&child_path, &target_path)?;
            if is_markdown_path(&target_path) {
                *note_count += 1;
            } else {
                *attachment_count += 1;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ObsidianVault, VaultEntryKind, VaultPipelineStage, path_to_relative_string,
        sanitized_note_filename,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn creates_reads_and_lists_notes() {
        let root = temp_dir("vault-create-read");
        let vault = ObsidianVault::from_root(&root);

        let created = vault
            .create_note(None, "April plan")
            .expect("note should be created");
        assert_eq!(created.path, "Inbox/April plan.md");

        let loaded = vault
            .read_note(&created.path)
            .expect("note should be readable");
        assert!(loaded.body.contains("vault_created_at:"));
        assert!(loaded.body.contains("vault_touched_at:"));
        assert!(loaded.body.contains("# April plan"));

        let tree = vault.list_tree().expect("tree should load");
        assert_eq!(tree.len(), 6);
        assert!(
            tree.iter()
                .any(|entry| entry.kind == VaultEntryKind::Directory && entry.path == "Inbox")
        );
        assert!(
            tree.iter()
                .any(|entry| entry.kind == VaultEntryKind::Note
                    && entry.path == "Inbox/April plan.md")
        );
    }

    #[test]
    fn rejects_parent_escape() {
        let root = temp_dir("vault-parent-escape");
        let vault = ObsidianVault::from_root(root);
        let error = vault
            .save_note("../outside.md", "nope")
            .expect_err("parent escapes should fail");
        assert!(error.to_string().contains("invalid relative path"));
    }

    #[test]
    fn imports_markdown_tree_into_imported_folder() {
        let root = temp_dir("vault-import-target");
        let source = temp_dir("vault-import-source");
        fs::create_dir_all(source.join("Project/Assets")).expect("source directory");
        fs::write(source.join("Project/Index.md"), "# Imported\n").expect("write note");
        fs::write(source.join("Project/Assets/diagram.png"), "png").expect("write attachment");
        fs::write(source.join(".DS_Store"), "ignore").expect("write hidden file");

        let vault = ObsidianVault::from_root(&root);
        let report = vault
            .import_markdown_tree(source.to_str().expect("utf8 path"))
            .expect("import should succeed");

        assert_eq!(report.note_count, 1);
        assert_eq!(report.attachment_count, 1);
        assert!(report.imported_root.starts_with("Imported/"));

        let imported = root.join(PathBuf::from(report.imported_root));
        assert!(imported.join("Project/Index.md").exists());
        assert!(imported.join("Project/Assets/diagram.png").exists());
    }

    #[test]
    fn relative_paths_use_forward_slashes() {
        let path = PathBuf::from("Folder").join("Note.md");
        assert_eq!(path_to_relative_string(&path), "Folder/Note.md");
    }

    #[test]
    fn prepares_hidden_raw_pipeline_run() {
        let root = temp_dir("vault-pipeline-run");
        let vault = ObsidianVault::from_root(&root);

        let run = vault
            .prepare_pipeline_run(VaultPipelineStage::Raw, "Apple Notes")
            .expect("pipeline run should be created");

        assert!(run.relative_root.starts_with(".raw/apple-notes"));
        assert!(run.absolute_root.exists());
        assert!(!run.run_id.is_empty());
    }

    #[test]
    fn save_note_preserves_created_and_updates_touched() {
        let root = temp_dir("vault-freshness");
        let vault = ObsidianVault::from_root(&root);

        let created = vault.create_note(None, "Fresh note").expect("note");
        let first = vault.read_note(&created.path).expect("first read");
        let created_at = first
            .body
            .lines()
            .find(|line| line.starts_with("vault_created_at: "))
            .expect("created line")
            .to_string();

        let saved = vault
            .save_note(&created.path, "# Fresh note\n\nUpdated body\n")
            .expect("save should succeed");

        assert!(saved.body.contains(&created_at));
        assert!(saved.body.contains("Updated body"));
    }

    #[test]
    fn truncates_long_note_titles_and_duplicate_suffixes() {
        let root = temp_dir("vault-long-title");
        let vault = ObsidianVault::from_root(&root);
        let title = "A".repeat(400);

        let first = vault
            .allocate_note_path(PathBuf::from("Inbox").as_path(), &title)
            .expect("first path");
        let first_name = first
            .file_name()
            .and_then(|value| value.to_str())
            .expect("first filename");
        assert!(first_name.len() <= 255);
        assert!(
            first
                .file_stem()
                .and_then(|value| value.to_str())
                .expect("first stem")
                .len()
                <= 248
        );

        let first_absolute = root.join(&first);
        fs::create_dir_all(first_absolute.parent().expect("first parent")).expect("parent dir");
        fs::write(&first_absolute, "# first\n").expect("first note");

        let second = vault
            .allocate_note_path(PathBuf::from("Inbox").as_path(), &title)
            .expect("second path");
        let second_name = second
            .file_name()
            .and_then(|value| value.to_str())
            .expect("second filename");
        assert!(second_name.ends_with(" 2.md"));
        assert!(second_name.len() <= 255);
        assert!(
            second
                .file_stem()
                .and_then(|value| value.to_str())
                .expect("second stem")
                .len()
                <= 248
        );
    }

    #[test]
    fn sanitized_note_filename_respects_asset_directory_budget() {
        let filename =
            sanitized_note_filename(&"B".repeat(400)).expect("sanitized filename should exist");
        let path = PathBuf::from(&filename);
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("stem");
        assert!(filename.len() <= 255);
        assert!(stem.len() <= 248);
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("organism-notes-{label}-{nonce}"));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }
}
