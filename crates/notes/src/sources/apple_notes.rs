use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::vault::{
    ObsidianVault, VaultError, VaultPipelineStage, path_to_relative_string, sanitize_directory_name,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppleNotesImportError {
    #[error("apple notes script failed: {0}")]
    AppleScriptExecutionFailed(String),
    #[error("apple notes import run not found: {0}")]
    ImportRunNotFound(String),
    #[error("no completed apple notes import run found")]
    NoCompletedImportRun,
    #[error("invalid apple notes import artifact: {0}")]
    InvalidImportedNote(String),
    #[error(transparent)]
    Vault(#[from] VaultError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
}

pub type AppleNotesImportResult<T> = Result<T, AppleNotesImportError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppleNotesImportReport {
    pub run_id: String,
    pub imported_root: String,
    pub raw_root: String,
    pub note_root: String,
    pub manifest_path: String,
    pub note_count: usize,
    pub attachment_count: usize,
    pub reused_note_count: usize,
    pub reused_attachment_count: usize,
    pub locked_note_count: usize,
    pub timed_out_note_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppleNotesPublishReport {
    pub run_id: String,
    pub published_at: DateTime<Utc>,
    pub source_run_id: String,
    pub source_raw_root: String,
    pub source_manifest_path: String,
    pub published_root: String,
    pub report_path: String,
    pub note_count: usize,
    pub attachment_count: usize,
    pub created_note_count: usize,
    pub updated_note_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppleNotesScanReport {
    pub account_count: usize,
    pub folder_count: usize,
    pub note_count: usize,
    pub folders: Vec<AppleNotesFolderScan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppleNotesFolderScan {
    pub account: String,
    pub folder: String,
    pub note_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppleNotesImportManifest {
    pub manifest_version: u32,
    pub source_system: String,
    pub status: AppleNotesImportStatus,
    pub run_id: String,
    pub imported_at: DateTime<Utc>,
    pub raw_root: String,
    pub note_root: String,
    pub scan: AppleNotesScanReport,
    pub note_count: usize,
    pub attachment_count: usize,
    pub reused_note_count: usize,
    pub reused_attachment_count: usize,
    pub locked_note_count: usize,
    pub timed_out_note_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppleNotesImportStatus {
    Running,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppleNotesImportProgress {
    IndexingExistingImports,
    ExistingImportsIndexed {
        reusable_note_count: usize,
    },
    ScanningLibrary,
    LibraryScanned(AppleNotesScanReport),
    ExportingFolder {
        completed_folders: usize,
        total_folders: usize,
        account: String,
        folder: String,
        note_count: usize,
    },
    ExportingBatch {
        completed_folders: usize,
        total_folders: usize,
        account: String,
        folder: String,
        batch_start: usize,
        batch_end: usize,
        folder_note_count: usize,
    },
    WritingNotes {
        total: usize,
    },
    NoteWritten {
        completed: usize,
        total: usize,
        relative_path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AppleScriptNote {
    title: String,
    content: String,
    folder: String,
    account: String,
    id: String,
    created: String,
    modified: String,
    #[serde(default)]
    locked: bool,
    #[serde(default)]
    content_unavailable: bool,
    #[serde(default)]
    content_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AppleScriptNoteMetadata {
    title: String,
    folder: String,
    account: String,
    id: String,
    created: String,
    modified: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExistingImportedNote {
    note_id: String,
    account: String,
    folder: String,
    updated: String,
    content_state: String,
    source_note_path: PathBuf,
    source_asset_dir: Option<PathBuf>,
    source_attachment_count: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct AppleNotesPublishCounts {
    note_count: usize,
    attachment_count: usize,
    created_note_count: usize,
    updated_note_count: usize,
}

const APPLE_NOTES_METADATA_BATCH_SIZE: usize = 100;
const APPLE_NOTES_SELECTED_EXPORT_CHUNK_SIZE: usize = 10;
const APPLE_NOTES_SCAN_TIMEOUT: Duration = Duration::from_secs(30);
const APPLE_NOTES_METADATA_TIMEOUT: Duration = Duration::from_secs(30);
const APPLE_NOTES_SELECTED_NOTES_TIMEOUT: Duration = Duration::from_secs(45);
const APPLE_NOTES_SINGLE_NOTE_TIMEOUT: Duration = Duration::from_secs(25);
const APPLE_NOTES_PUBLISHED_ROOT: &str = "Imported/Apple Notes";

pub fn import_apple_notes(vault: &ObsidianVault) -> AppleNotesImportResult<AppleNotesImportReport> {
    import_apple_notes_with_progress(vault, |_| {})
}

pub fn publish_apple_notes(
    vault: &ObsidianVault,
    source_run_id: Option<&str>,
) -> AppleNotesImportResult<AppleNotesPublishReport> {
    vault.ensure_root()?;

    let (source_manifest, source_manifest_relative_path) =
        load_completed_apple_notes_manifest(vault, source_run_id)?;
    let source_note_root_relative = PathBuf::from(&source_manifest.note_root);
    let source_note_root = vault.resolve_relative_path(&source_note_root_relative)?;
    if !source_note_root.exists() {
        return Err(AppleNotesImportError::InvalidImportedNote(format!(
            "missing source note root: {}",
            source_manifest.note_root
        )));
    }

    let published_root_relative = PathBuf::from(APPLE_NOTES_PUBLISHED_ROOT);
    let published_root = vault.resolve_relative_path(&published_root_relative)?;
    fs::create_dir_all(&published_root)?;
    let existing_published_notes = index_existing_imported_notes_under(&published_root)?;

    let publish_run =
        vault.prepare_pipeline_run(VaultPipelineStage::Enriched, "apple-notes-publish")?;
    let report_relative_path = publish_run.relative_root.join("report.json");
    let published_at = Utc::now();
    let mut counts = AppleNotesPublishCounts::default();

    publish_apple_notes_directory(
        vault,
        &source_note_root,
        &source_note_root,
        &published_root_relative,
        &existing_published_notes,
        &source_manifest.run_id,
        &path_to_relative_string(&source_manifest_relative_path),
        published_at,
        &mut counts,
    )?;

    let report = AppleNotesPublishReport {
        run_id: publish_run.run_id,
        published_at,
        source_run_id: source_manifest.run_id,
        source_raw_root: source_manifest.raw_root,
        source_manifest_path: path_to_relative_string(&source_manifest_relative_path),
        published_root: path_to_relative_string(&published_root_relative),
        report_path: path_to_relative_string(&report_relative_path),
        note_count: counts.note_count,
        attachment_count: counts.attachment_count,
        created_note_count: counts.created_note_count,
        updated_note_count: counts.updated_note_count,
    };

    write_publish_report(vault, &report_relative_path, &report)?;
    Ok(report)
}

pub fn import_apple_notes_with_progress<F>(
    vault: &ObsidianVault,
    mut on_progress: F,
) -> AppleNotesImportResult<AppleNotesImportReport>
where
    F: FnMut(AppleNotesImportProgress),
{
    vault.ensure_root()?;
    on_progress(AppleNotesImportProgress::IndexingExistingImports);
    let existing_notes = index_existing_imported_notes(vault)?;
    on_progress(AppleNotesImportProgress::ExistingImportsIndexed {
        reusable_note_count: existing_notes.len(),
    });
    on_progress(AppleNotesImportProgress::ScanningLibrary);
    let scan = scan_apple_notes()?;
    on_progress(AppleNotesImportProgress::LibraryScanned(scan.clone()));
    let raw_run = vault.prepare_pipeline_run(VaultPipelineStage::Raw, "apple-notes")?;
    let note_root_relative = raw_run.relative_root.join("notes");
    let manifest_relative_path = raw_run.relative_root.join("manifest.json");
    let total_notes = scan.note_count;
    on_progress(AppleNotesImportProgress::WritingNotes { total: total_notes });

    let mut note_count = 0;
    let mut attachment_count = 0;
    let mut reused_note_count = 0;
    let mut reused_attachment_count = 0;
    let mut locked_note_count = 0;
    let mut timed_out_note_count = 0;
    let imported_at = Utc::now();

    write_manifest(
        vault,
        &manifest_relative_path,
        &AppleNotesImportManifest {
            manifest_version: 1,
            source_system: "apple_notes".to_string(),
            status: AppleNotesImportStatus::Running,
            run_id: raw_run.run_id.clone(),
            imported_at,
            raw_root: path_to_relative_string(&raw_run.relative_root),
            note_root: path_to_relative_string(&note_root_relative),
            scan: scan.clone(),
            note_count,
            attachment_count,
            reused_note_count,
            reused_attachment_count,
            locked_note_count,
            timed_out_note_count,
        },
    )?;

    let total_folders = scan.folders.len();
    for (folder_index, folder_scan) in scan.folders.iter().enumerate() {
        on_progress(AppleNotesImportProgress::ExportingFolder {
            completed_folders: folder_index,
            total_folders,
            account: folder_scan.account.clone(),
            folder: folder_scan.folder.clone(),
            note_count: folder_scan.note_count,
        });

        if folder_scan.note_count == 0 {
            continue;
        }

        let mut batch_start = 1;
        while batch_start <= folder_scan.note_count {
            let batch_end =
                (batch_start + APPLE_NOTES_METADATA_BATCH_SIZE - 1).min(folder_scan.note_count);
            let metadata_batch = read_apple_notes_folder_batch_metadata_from_system(
                &folder_scan.account,
                &folder_scan.folder,
                batch_start,
                batch_end,
            )?;
            let metadata_by_index = metadata_batch
                .iter()
                .enumerate()
                .map(|(offset, metadata)| (batch_start + offset, metadata.clone()))
                .collect::<HashMap<_, _>>();

            let mut export_indices = Vec::new();
            for (offset, metadata) in metadata_batch.iter().enumerate() {
                let batch_note_index = batch_start + offset;
                if let Some(existing) = existing_notes.get(metadata.id.trim()) {
                    if can_reuse_existing_note(existing, metadata) {
                        let relative_path = reuse_existing_note(
                            vault,
                            &note_root_relative,
                            existing,
                            &mut attachment_count,
                            &mut reused_attachment_count,
                            imported_at,
                        )?;
                        note_count += 1;
                        reused_note_count += 1;
                        if note_count == 1 || note_count == total_notes || note_count % 25 == 0 {
                            on_progress(AppleNotesImportProgress::NoteWritten {
                                completed: note_count,
                                total: total_notes,
                                relative_path,
                            });
                        }
                        continue;
                    }
                }
                export_indices.push(batch_note_index);
            }

            for export_chunk in export_indices.chunks(APPLE_NOTES_SELECTED_EXPORT_CHUNK_SIZE) {
                let export_start = *export_chunk
                    .first()
                    .expect("export chunk should not be empty");
                let export_end = *export_chunk
                    .last()
                    .expect("export chunk should not be empty");
                on_progress(AppleNotesImportProgress::ExportingBatch {
                    completed_folders: folder_index,
                    total_folders,
                    account: folder_scan.account.clone(),
                    folder: folder_scan.folder.clone(),
                    batch_start: export_start,
                    batch_end: export_end,
                    folder_note_count: folder_scan.note_count,
                });

                let notes = read_apple_notes_folder_selected_notes_from_system(
                    &folder_scan.account,
                    &folder_scan.folder,
                    export_chunk,
                    &metadata_by_index,
                    |note_index| {
                        on_progress(AppleNotesImportProgress::ExportingBatch {
                            completed_folders: folder_index,
                            total_folders,
                            account: folder_scan.account.clone(),
                            folder: folder_scan.folder.clone(),
                            batch_start: note_index,
                            batch_end: note_index,
                            folder_note_count: folder_scan.note_count,
                        });
                    },
                )?;

                for note in notes {
                    let account_name = non_empty_name(&note.account, "Account");
                    let folder_name = non_empty_name(&note.folder, "Notes");
                    let relative_dir = note_root_relative
                        .join(sanitize_directory_name(&account_name))
                        .join(sanitize_directory_name(&folder_name));
                    let note_title = non_empty_name(&note.title, "Untitled");
                    let relative_path = vault.allocate_note_path(&relative_dir, &note_title)?;
                    let absolute_path = vault.resolve_relative_path(&relative_path)?;

                    let markdown = render_apple_note_markdown(
                        &note,
                        &absolute_path,
                        &mut attachment_count,
                        imported_at,
                    )?;
                    vault.write_text_file(&relative_path, &markdown)?;

                    note_count += 1;
                    if note_count == 1 || note_count == total_notes || note_count % 25 == 0 {
                        on_progress(AppleNotesImportProgress::NoteWritten {
                            completed: note_count,
                            total: total_notes,
                            relative_path: path_to_relative_string(&relative_path),
                        });
                    }
                    if note.locked {
                        locked_note_count += 1;
                    }
                    if is_timed_out_content_state(&note.content_state) {
                        timed_out_note_count += 1;
                    }
                }
            }

            write_manifest(
                vault,
                &manifest_relative_path,
                &AppleNotesImportManifest {
                    manifest_version: 1,
                    source_system: "apple_notes".to_string(),
                    status: AppleNotesImportStatus::Running,
                    run_id: raw_run.run_id.clone(),
                    imported_at,
                    raw_root: path_to_relative_string(&raw_run.relative_root),
                    note_root: path_to_relative_string(&note_root_relative),
                    scan: scan.clone(),
                    note_count,
                    attachment_count,
                    reused_note_count,
                    reused_attachment_count,
                    locked_note_count,
                    timed_out_note_count,
                },
            )?;

            batch_start = batch_end + 1;
        }
    }

    let raw_root = path_to_relative_string(&raw_run.relative_root);
    let note_root = path_to_relative_string(&note_root_relative);
    let manifest_path = path_to_relative_string(&manifest_relative_path);

    write_manifest(
        vault,
        &manifest_relative_path,
        &AppleNotesImportManifest {
            manifest_version: 1,
            source_system: "apple_notes".to_string(),
            status: AppleNotesImportStatus::Completed,
            run_id: raw_run.run_id.clone(),
            imported_at,
            raw_root: raw_root.clone(),
            note_root: note_root.clone(),
            scan,
            note_count,
            attachment_count,
            reused_note_count,
            reused_attachment_count,
            locked_note_count,
            timed_out_note_count,
        },
    )?;

    Ok(AppleNotesImportReport {
        run_id: raw_run.run_id,
        imported_root: raw_root.clone(),
        raw_root,
        note_root,
        manifest_path,
        note_count,
        attachment_count,
        reused_note_count,
        reused_attachment_count,
        locked_note_count,
        timed_out_note_count,
    })
}

fn load_completed_apple_notes_manifest(
    vault: &ObsidianVault,
    requested_run_id: Option<&str>,
) -> AppleNotesImportResult<(AppleNotesImportManifest, PathBuf)> {
    let raw_root = vault.root().join(".raw").join("apple-notes");
    if !raw_root.exists() {
        return Err(AppleNotesImportError::NoCompletedImportRun);
    }

    if let Some(run_id) = requested_run_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let manifest_relative_path = Path::new(".raw")
            .join("apple-notes")
            .join(run_id)
            .join("manifest.json");
        let manifest_path = vault.resolve_relative_path(&manifest_relative_path)?;
        if !manifest_path.exists() {
            return Err(AppleNotesImportError::ImportRunNotFound(run_id.to_string()));
        }
        let manifest =
            serde_json::from_str::<AppleNotesImportManifest>(&fs::read_to_string(&manifest_path)?)?;
        if manifest.status != AppleNotesImportStatus::Completed {
            return Err(AppleNotesImportError::InvalidImportedNote(format!(
                "run is not completed: {run_id}"
            )));
        }
        return Ok((manifest, manifest_relative_path));
    }

    let mut completed_manifests = Vec::new();
    let children = fs::read_dir(&raw_root)?.collect::<Result<Vec<_>, _>>()?;
    for child in children {
        let manifest_path = child.path().join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }

        let manifest =
            serde_json::from_str::<AppleNotesImportManifest>(&fs::read_to_string(&manifest_path)?)?;
        if manifest.status != AppleNotesImportStatus::Completed {
            continue;
        }

        let manifest_relative_path = manifest_path
            .strip_prefix(vault.root())
            .map(PathBuf::from)
            .map_err(|_| {
                AppleNotesImportError::InvalidImportedNote(manifest_path.display().to_string())
            })?;
        completed_manifests.push((manifest.imported_at, manifest, manifest_relative_path));
    }

    completed_manifests
        .into_iter()
        .max_by_key(|(imported_at, _, _)| *imported_at)
        .map(|(_, manifest, path)| (manifest, path))
        .ok_or(AppleNotesImportError::NoCompletedImportRun)
}

fn publish_apple_notes_directory(
    vault: &ObsidianVault,
    source_note_root: &Path,
    directory: &Path,
    published_root_relative: &Path,
    existing_published_notes: &HashMap<String, ExistingImportedNote>,
    source_run_id: &str,
    source_manifest_path: &str,
    published_at: DateTime<Utc>,
    counts: &mut AppleNotesPublishCounts,
) -> AppleNotesImportResult<()> {
    let children = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    for child in children {
        let source_path = child.path();
        if source_path.is_dir() {
            publish_apple_notes_directory(
                vault,
                source_note_root,
                &source_path,
                published_root_relative,
                existing_published_notes,
                source_run_id,
                source_manifest_path,
                published_at,
                counts,
            )?;
            continue;
        }

        if !is_markdown_path(&source_path) {
            continue;
        }

        let imported_note = parse_existing_imported_note(&source_path)?.ok_or_else(|| {
            AppleNotesImportError::InvalidImportedNote(source_path.display().to_string())
        })?;
        let source_relative_path = source_path
            .strip_prefix(vault.root())
            .map(PathBuf::from)
            .map_err(|_| {
                AppleNotesImportError::InvalidImportedNote(source_path.display().to_string())
            })?;
        let source_note_relative_path = source_path
            .strip_prefix(source_note_root)
            .map(PathBuf::from)
            .map_err(|_| {
                AppleNotesImportError::InvalidImportedNote(source_path.display().to_string())
            })?;
        let target_relative_path = existing_published_notes
            .get(&imported_note.note_id)
            .and_then(|existing| {
                existing
                    .source_note_path
                    .strip_prefix(vault.root())
                    .ok()
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| published_root_relative.join(&source_note_relative_path));

        let source_body = fs::read_to_string(&source_path)?;
        let target_body = render_published_apple_note_markdown(
            &source_body,
            source_run_id,
            source_manifest_path,
            &path_to_relative_string(&source_relative_path),
            published_at,
        );
        vault.save_note(
            &path_to_relative_string(&target_relative_path),
            &target_body,
        )?;

        let target_path = vault.resolve_relative_path(&target_relative_path)?;
        let target_asset_dir = note_asset_directory(&target_path);
        if target_asset_dir.exists() {
            fs::remove_dir_all(&target_asset_dir)?;
        }
        if let Some(source_asset_dir) = &imported_note.source_asset_dir {
            copy_directory(source_asset_dir, &target_asset_dir)?;
            counts.attachment_count += imported_note.source_attachment_count;
        }

        counts.note_count += 1;
        if existing_published_notes.contains_key(&imported_note.note_id) {
            counts.updated_note_count += 1;
        } else {
            counts.created_note_count += 1;
        }
    }

    Ok(())
}

fn render_published_apple_note_markdown(
    raw_body: &str,
    source_run_id: &str,
    source_manifest_path: &str,
    raw_note_path: &str,
    published_at: DateTime<Utc>,
) -> String {
    let published_at = published_at.to_rfc3339();
    let (frontmatter, remainder, had_frontmatter) = match split_frontmatter(raw_body) {
        Some((frontmatter, remainder)) => (frontmatter.to_string(), remainder.to_string(), true),
        None => (String::new(), raw_body.to_string(), false),
    };

    let frontmatter = upsert_frontmatter_pair(&frontmatter, "kind", "source_capture");
    let frontmatter = upsert_frontmatter_pair(&frontmatter, "provenance", "imported_source");
    let frontmatter = upsert_frontmatter_pair(&frontmatter, "review_state", "unreviewed");
    let frontmatter = upsert_frontmatter_pair(&frontmatter, "managed_by", "apple_notes_publish");
    let frontmatter = upsert_frontmatter_pair(&frontmatter, "source_system", "apple_notes");
    let frontmatter = upsert_frontmatter_pair(&frontmatter, "raw_run_id", source_run_id);
    let frontmatter =
        upsert_frontmatter_pair(&frontmatter, "raw_manifest_path", source_manifest_path);
    let frontmatter = upsert_frontmatter_pair(&frontmatter, "raw_note_path", raw_note_path);
    let frontmatter = upsert_frontmatter_pair(&frontmatter, "published_at", &published_at);
    let frontmatter = upsert_frontmatter_pair(&frontmatter, "indexed_at", &published_at);

    if had_frontmatter {
        format!("---\n{frontmatter}\n---\n{remainder}")
    } else {
        format!("---\n{frontmatter}\n---\n\n{remainder}")
    }
}

pub fn scan_apple_notes() -> AppleNotesImportResult<AppleNotesScanReport> {
    read_apple_notes_scan_from_system()
}

const APPLE_NOTES_METADATA_FOLDER_BATCH_SCRIPT: &str = r#"
on run argv
    if (count of argv) is not 4 then
        error "expected account, folder, start index, and end index"
    end if

    set requestedAccountName to item 1 of argv
    set requestedFolderName to item 2 of argv
    set startIndex to (item 3 of argv) as integer
    set endIndex to (item 4 of argv) as integer
    set notesList to {}

    tell application "Notes"
        set matchedFolder to missing value
        set theAccounts to every account
        repeat with anAccount in theAccounts
            if (name of anAccount) is requestedAccountName then
                set theFolders to every folder of anAccount
                repeat with aFolder in theFolders
                    if (name of aFolder) is requestedFolderName then
                        set matchedFolder to aFolder
                        exit repeat
                    end if
                end repeat
                exit repeat
            end if
        end repeat

        if matchedFolder is missing value then
            error "folder not found: " & requestedAccountName & "/" & requestedFolderName
        end if

        set theNotes to notes of matchedFolder
        set totalNotes to count of theNotes
        if startIndex < 1 then set startIndex to 1
        if endIndex > totalNotes then set endIndex to totalNotes
        if startIndex > totalNotes or startIndex > endIndex then return "[]"

        repeat with noteIndex from startIndex to endIndex
            set theNote to item noteIndex of theNotes
            set noteTitle to name of theNote
            set noteId to id of theNote
            set noteCreationDate to creation date of theNote
            set noteModificationDate to modification date of theNote

            set cleanTitle to my cleanForJson(noteTitle)
            set cleanFolder to my cleanForJson(requestedFolderName)
            set cleanAccount to my cleanForJson(requestedAccountName)
            set cleanId to my cleanForJson(noteId)

            set noteData to "{"
            set noteData to noteData & "\"title\":\"" & cleanTitle & "\","
            set noteData to noteData & "\"folder\":\"" & cleanFolder & "\","
            set noteData to noteData & "\"account\":\"" & cleanAccount & "\","
            set noteData to noteData & "\"id\":\"" & cleanId & "\","
            set noteData to noteData & "\"created\":\"" & ((noteCreationDate as text)) & "\","
            set noteData to noteData & "\"modified\":\"" & ((noteModificationDate as text)) & "\""
            set noteData to noteData & "}"

            set end of notesList to noteData
        end repeat
    end tell

    return "[" & (my joinList(notesList, ",")) & "]"
end run

on joinList(theList, theDelimiter)
    set oldDelimiters to AppleScript's text item delimiters
    set AppleScript's text item delimiters to theDelimiter
    set theString to theList as string
    set AppleScript's text item delimiters to oldDelimiters
    return theString
end joinList

on cleanForJson(str)
    set str to my replaceText(str, "\\", "\\\\")
    set str to my replaceText(str, "\"", "\\\"")
    set str to my replaceText(str, "\n", "\\n")
    set str to my replaceText(str, "\r", "\\r")
    set str to my replaceText(str, "\t", "\\t")
    set str to my replaceText(str, "/", "\\/")
    return str
end cleanForJson

on replaceText(sourceText, searchString, replacementString)
    set AppleScript's text item delimiters to searchString
    set the textItems to every text item of sourceText
    set AppleScript's text item delimiters to replacementString
    set sourceText to textItems as string
    set AppleScript's text item delimiters to ""
    return sourceText
end replaceText
"#;

const APPLE_NOTES_EXPORT_SELECTED_NOTES_SCRIPT: &str = r#"
on run argv
    if (count of argv) is less than 3 then
        error "expected account, folder, and at least one note index"
    end if

    set requestedAccountName to item 1 of argv
    set requestedFolderName to item 2 of argv
    set notesList to {}

    tell application "Notes"
        set matchedFolder to missing value
        set theAccounts to every account
        repeat with anAccount in theAccounts
            if (name of anAccount) is requestedAccountName then
                set theFolders to every folder of anAccount
                repeat with aFolder in theFolders
                    if (name of aFolder) is requestedFolderName then
                        set matchedFolder to aFolder
                        exit repeat
                    end if
                end repeat
                exit repeat
            end if
        end repeat

        if matchedFolder is missing value then
            error "folder not found: " & requestedAccountName & "/" & requestedFolderName
        end if

        set theNotes to notes of matchedFolder
        set totalNotes to count of theNotes

        repeat with argumentIndex from 3 to (count of argv)
            set noteIndex to (item argumentIndex of argv) as integer
            if noteIndex < 1 or noteIndex > totalNotes then
                error "note index out of range: " & noteIndex
            end if

            set theNote to item noteIndex of theNotes
            set noteTitle to name of theNote
            set noteId to id of theNote
            set noteCreationDate to creation date of theNote
            set noteModificationDate to modification date of theNote
            set noteLocked to false
            set noteContentUnavailable to false
            set noteContentState to "available"
            set noteContent to ""
            try
                with timeout of 20 seconds
                    set noteContent to body of theNote
                end timeout
            on error errorMessage number errorNumber
                set noteContent to ""
                set noteContentUnavailable to true
                if errorNumber is -1712 then
                    set noteContentState to "timeout"
                else
                    set noteLocked to true
                    set noteContentState to "locked"
                end if
            end try

            set cleanTitle to my cleanForJson(noteTitle)
            set cleanContent to my cleanForJson(noteContent)
            set cleanFolder to my cleanForJson(requestedFolderName)
            set cleanAccount to my cleanForJson(requestedAccountName)
            set cleanId to my cleanForJson(noteId)
            set cleanContentState to my cleanForJson(noteContentState)

            set noteData to "{"
            set noteData to noteData & "\"title\":\"" & cleanTitle & "\","
            set noteData to noteData & "\"content\":\"" & cleanContent & "\","
            set noteData to noteData & "\"folder\":\"" & cleanFolder & "\","
            set noteData to noteData & "\"account\":\"" & cleanAccount & "\","
            set noteData to noteData & "\"id\":\"" & cleanId & "\","
            set noteData to noteData & "\"created\":\"" & ((noteCreationDate as text)) & "\","
            set noteData to noteData & "\"modified\":\"" & ((noteModificationDate as text)) & "\","
            set noteData to noteData & "\"locked\":" & (my booleanLiteral(noteLocked)) & ","
            set noteData to noteData & "\"content_unavailable\":" & (my booleanLiteral(noteContentUnavailable)) & ","
            set noteData to noteData & "\"content_state\":\"" & cleanContentState & "\""
            set noteData to noteData & "}"

            set end of notesList to noteData
        end repeat
    end tell

    return "[" & (my joinList(notesList, ",")) & "]"
end run

on joinList(theList, theDelimiter)
    set oldDelimiters to AppleScript's text item delimiters
    set AppleScript's text item delimiters to theDelimiter
    set theString to theList as string
    set AppleScript's text item delimiters to oldDelimiters
    return theString
end joinList

on booleanLiteral(value)
    if value is true then
        return "true"
    end if
    return "false"
end booleanLiteral

on cleanForJson(str)
    set str to my replaceText(str, "\\", "\\\\")
    set str to my replaceText(str, "\"", "\\\"")
    set str to my replaceText(str, "\n", "\\n")
    set str to my replaceText(str, "\r", "\\r")
    set str to my replaceText(str, "\t", "\\t")
    set str to my replaceText(str, "/", "\\/")
    return str
end cleanForJson

on replaceText(sourceText, searchString, replacementString)
    set AppleScript's text item delimiters to searchString
    set the textItems to every text item of sourceText
    set AppleScript's text item delimiters to replacementString
    set sourceText to textItems as string
    set AppleScript's text item delimiters to ""
    return sourceText
end replaceText
"#;

const APPLE_NOTES_SCAN_SCRIPT: &str = r#"
on run
    set accountCount to 0
    set folderCount to 0
    set noteCount to 0
    set folderList to {}

    tell application "Notes"
        set theAccounts to every account
        repeat with anAccount in theAccounts
            set accountCount to accountCount + 1
            set accountName to name of anAccount
            set theFolders to every folder of anAccount
            repeat with aFolder in theFolders
                set folderName to name of aFolder
                if folderName is not "Recently Deleted" then
                    set folderNoteCount to count of (notes of aFolder)
                    set folderCount to folderCount + 1
                    set noteCount to noteCount + folderNoteCount

                    set cleanFolder to my cleanForJson(folderName)
                    set cleanAccount to my cleanForJson(accountName)

                    set folderData to "{"
                    set folderData to folderData & "\"account\":\"" & cleanAccount & "\","
                    set folderData to folderData & "\"folder\":\"" & cleanFolder & "\","
                    set folderData to folderData & "\"note_count\":" & folderNoteCount
                    set folderData to folderData & "}"
                    set end of folderList to folderData
                end if
            end repeat
        end repeat
    end tell

    return "{\"account_count\":" & accountCount & ",\"folder_count\":" & folderCount & ",\"note_count\":" & noteCount & ",\"folders\":[" & (my joinList(folderList, ",")) & "]}"
end run

on joinList(theList, theDelimiter)
    set oldDelimiters to AppleScript's text item delimiters
    set AppleScript's text item delimiters to theDelimiter
    set theString to theList as string
    set AppleScript's text item delimiters to oldDelimiters
    return theString
end joinList

on cleanForJson(str)
    set str to my replaceText(str, "\\", "\\\\")
    set str to my replaceText(str, "\"", "\\\"")
    set str to my replaceText(str, "\n", "\\n")
    set str to my replaceText(str, "\r", "\\r")
    set str to my replaceText(str, "\t", "\\t")
    set str to my replaceText(str, "/", "\\/")
    return str
end cleanForJson

on replaceText(sourceText, searchString, replacementString)
    set AppleScript's text item delimiters to searchString
    set the textItems to every text item of sourceText
    set AppleScript's text item delimiters to replacementString
    set sourceText to textItems as string
    set AppleScript's text item delimiters to ""
    return sourceText
end replaceText
"#;

fn read_apple_notes_folder_batch_metadata_from_system(
    account: &str,
    folder: &str,
    start_index: usize,
    end_index: usize,
) -> AppleNotesImportResult<Vec<AppleScriptNoteMetadata>> {
    match read_apple_notes_folder_batch_metadata_once(account, folder, start_index, end_index)? {
        MetadataBatchFetchOutcome::Metadata(metadata) => Ok(metadata),
        MetadataBatchFetchOutcome::TimedOut => {
            if start_index >= end_index {
                return Err(AppleNotesImportError::AppleScriptExecutionFailed(
                    "apple notes metadata export timed out".to_string(),
                ));
            }

            let midpoint = start_index + ((end_index - start_index) / 2);
            let mut left = read_apple_notes_folder_batch_metadata_from_system(
                account,
                folder,
                start_index,
                midpoint,
            )?;
            let right = read_apple_notes_folder_batch_metadata_from_system(
                account,
                folder,
                midpoint + 1,
                end_index,
            )?;
            left.extend(right);
            Ok(left)
        }
        MetadataBatchFetchOutcome::ScriptError(message) => {
            Err(AppleNotesImportError::AppleScriptExecutionFailed(message))
        }
    }
}

fn read_apple_notes_folder_batch_metadata_once(
    account: &str,
    folder: &str,
    start_index: usize,
    end_index: usize,
) -> AppleNotesImportResult<MetadataBatchFetchOutcome> {
    let mut command = Command::new("osascript");
    command
        .arg("-e")
        .arg(APPLE_NOTES_METADATA_FOLDER_BATCH_SCRIPT)
        .arg(account)
        .arg(folder)
        .arg(start_index.to_string())
        .arg(end_index.to_string());

    match run_command_with_timeout(command, APPLE_NOTES_METADATA_TIMEOUT)? {
        CommandRunResult::TimedOut => Ok(MetadataBatchFetchOutcome::TimedOut),
        CommandRunResult::Completed(output) => {
            if !output.status.success() {
                let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Ok(MetadataBatchFetchOutcome::ScriptError(message));
            }

            let json = String::from_utf8(output.stdout)?;
            Ok(MetadataBatchFetchOutcome::Metadata(serde_json::from_str(
                &json,
            )?))
        }
    }
}

fn read_apple_notes_folder_selected_notes_from_system<F>(
    account: &str,
    folder: &str,
    note_indices: &[usize],
    metadata_by_index: &HashMap<usize, AppleScriptNoteMetadata>,
    mut on_retry_note: F,
) -> AppleNotesImportResult<Vec<AppleScriptNote>>
where
    F: FnMut(usize),
{
    if note_indices.is_empty() {
        return Ok(Vec::new());
    }

    let mut notes = Vec::new();
    for chunk in note_indices.chunks(APPLE_NOTES_SELECTED_EXPORT_CHUNK_SIZE) {
        match run_selected_notes_fetch(account, folder, chunk, APPLE_NOTES_SELECTED_NOTES_TIMEOUT)?
        {
            SelectedNotesFetchOutcome::Notes(selected_notes) => notes.extend(selected_notes),
            SelectedNotesFetchOutcome::TimedOut | SelectedNotesFetchOutcome::ScriptError => {
                for index in chunk {
                    on_retry_note(*index);
                    match run_selected_notes_fetch(
                        account,
                        folder,
                        std::slice::from_ref(index),
                        APPLE_NOTES_SINGLE_NOTE_TIMEOUT,
                    )? {
                        SelectedNotesFetchOutcome::Notes(selected_notes) => {
                            notes.extend(selected_notes)
                        }
                        SelectedNotesFetchOutcome::TimedOut => {
                            notes.push(unavailable_note_from_metadata(
                                metadata_by_index.get(index),
                                account,
                                folder,
                                *index,
                                "process_timeout",
                            ));
                        }
                        SelectedNotesFetchOutcome::ScriptError => {
                            notes.push(unavailable_note_from_metadata(
                                metadata_by_index.get(index),
                                account,
                                folder,
                                *index,
                                "script_error",
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(notes)
}

fn read_apple_notes_scan_from_system() -> AppleNotesImportResult<AppleNotesScanReport> {
    let mut command = Command::new("osascript");
    command.arg("-e").arg(APPLE_NOTES_SCAN_SCRIPT);
    let output = run_command_with_timeout(command, APPLE_NOTES_SCAN_TIMEOUT)?;
    let json = expect_command_success(output, "apple notes scan timed out")?;
    Ok(serde_json::from_str(&json)?)
}

fn write_manifest(
    vault: &ObsidianVault,
    manifest_path: &Path,
    manifest: &AppleNotesImportManifest,
) -> AppleNotesImportResult<()> {
    let body = serde_json::to_string_pretty(manifest)?;
    vault.write_text_file(manifest_path, &body)?;
    Ok(())
}

fn write_publish_report(
    vault: &ObsidianVault,
    report_path: &Path,
    report: &AppleNotesPublishReport,
) -> AppleNotesImportResult<()> {
    let body = serde_json::to_string_pretty(report)?;
    vault.write_text_file(report_path, &body)?;
    Ok(())
}

enum CommandRunResult {
    Completed(Output),
    TimedOut,
}

enum SelectedNotesFetchOutcome {
    Notes(Vec<AppleScriptNote>),
    TimedOut,
    ScriptError,
}

enum MetadataBatchFetchOutcome {
    Metadata(Vec<AppleScriptNoteMetadata>),
    TimedOut,
    ScriptError(String),
}

fn run_command_with_timeout(
    mut command: Command,
    timeout: Duration,
) -> AppleNotesImportResult<CommandRunResult> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let started_at = Instant::now();

    loop {
        if child.try_wait()?.is_some() {
            return Ok(CommandRunResult::Completed(child.wait_with_output()?));
        }

        if started_at.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait_with_output()?;
            return Ok(CommandRunResult::TimedOut);
        }

        sleep(Duration::from_millis(100));
    }
}

fn expect_command_success(
    result: CommandRunResult,
    timeout_message: &str,
) -> AppleNotesImportResult<String> {
    match result {
        CommandRunResult::TimedOut => Err(AppleNotesImportError::AppleScriptExecutionFailed(
            timeout_message.to_string(),
        )),
        CommandRunResult::Completed(output) => {
            if !output.status.success() {
                let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err(AppleNotesImportError::AppleScriptExecutionFailed(message));
            }

            Ok(String::from_utf8(output.stdout)?)
        }
    }
}

fn run_selected_notes_fetch(
    account: &str,
    folder: &str,
    note_indices: &[usize],
    timeout: Duration,
) -> AppleNotesImportResult<SelectedNotesFetchOutcome> {
    let mut command = Command::new("osascript");
    command
        .arg("-e")
        .arg(APPLE_NOTES_EXPORT_SELECTED_NOTES_SCRIPT)
        .arg(account)
        .arg(folder);
    for index in note_indices {
        command.arg(index.to_string());
    }

    match run_command_with_timeout(command, timeout)? {
        CommandRunResult::TimedOut => Ok(SelectedNotesFetchOutcome::TimedOut),
        CommandRunResult::Completed(output) => {
            if !output.status.success() {
                return Ok(SelectedNotesFetchOutcome::ScriptError);
            }

            let json = String::from_utf8(output.stdout)?;
            let selected_notes = serde_json::from_str::<Vec<AppleScriptNote>>(&json)?;
            Ok(SelectedNotesFetchOutcome::Notes(selected_notes))
        }
    }
}

fn unavailable_note_from_metadata(
    metadata: Option<&AppleScriptNoteMetadata>,
    account: &str,
    folder: &str,
    note_index: usize,
    content_state: &str,
) -> AppleScriptNote {
    let title = metadata
        .map(|value| non_empty_name(&value.title, "Untitled"))
        .unwrap_or_else(|| format!("Untitled {note_index}"));
    let note_id = metadata
        .map(|value| value.id.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("unknown-note-{note_index}"));
    let created = metadata
        .map(|value| value.created.trim().to_string())
        .unwrap_or_default();
    let modified = metadata
        .map(|value| value.modified.trim().to_string())
        .unwrap_or_default();

    AppleScriptNote {
        title,
        content: String::new(),
        folder: metadata
            .map(|value| non_empty_name(&value.folder, folder))
            .unwrap_or_else(|| folder.to_string()),
        account: metadata
            .map(|value| non_empty_name(&value.account, account))
            .unwrap_or_else(|| account.to_string()),
        id: note_id,
        created,
        modified,
        locked: false,
        content_unavailable: true,
        content_state: content_state.to_string(),
    }
}

fn non_empty_name(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn is_timed_out_content_state(value: &str) -> bool {
    matches!(value.trim(), "timeout" | "process_timeout")
}

fn render_apple_note_markdown(
    note: &AppleScriptNote,
    note_path: &Path,
    attachment_count: &mut usize,
    imported_at: DateTime<Utc>,
) -> AppleNotesImportResult<String> {
    let body = if note.content_unavailable {
        match note.content_state.as_str() {
            "timeout" | "process_timeout" => {
                "> Apple Note body export timed out. Run the importer again or export this note manually from Notes.\n".to_string()
            }
            "script_error" => {
                "> Apple Note body export failed for this note. Run the importer again or export this note manually from Notes.\n".to_string()
            }
            _ => "> Locked Apple Note. Unlock it in Notes and run the importer again.\n"
                .to_string(),
        }
    } else {
        let prepared_html =
            replace_inline_image_data_urls(&note.content, note_path, attachment_count)?;
        let markdown = html2md::parse_html(&prepared_html).trim().to_string();
        if markdown.is_empty() {
            String::new()
        } else {
            markdown
        }
    };

    let mut output = String::new();
    output.push_str("---\n");
    output.push_str(&format!(
        "title: {}\n",
        yaml_quote(&non_empty_name(&note.title, "Untitled"))
    ));
    output.push_str(&format!(
        "source_created_at: {}\n",
        yaml_quote(note.created.trim())
    ));
    output.push_str(&format!(
        "source_updated_at: {}\n",
        yaml_quote(note.modified.trim())
    ));
    output.push_str(&format!(
        "vault_created_at: {}\n",
        yaml_quote(&imported_at.to_rfc3339())
    ));
    output.push_str(&format!(
        "vault_touched_at: {}\n",
        yaml_quote(&imported_at.to_rfc3339())
    ));
    output.push_str(&format!(
        "indexed_at: {}\n",
        yaml_quote(&imported_at.to_rfc3339())
    ));
    output.push_str(&format!("created: {}\n", yaml_quote(note.created.trim())));
    output.push_str(&format!("updated: {}\n", yaml_quote(note.modified.trim())));
    output.push_str("tags:\n");
    output.push_str("  - imported/apple-notes\n");
    output.push_str("source:\n");
    output.push_str("  system: apple_notes\n");
    output.push_str(&format!(
        "  account: {}\n",
        yaml_quote(&non_empty_name(&note.account, "Account"))
    ));
    output.push_str(&format!(
        "  folder: {}\n",
        yaml_quote(&non_empty_name(&note.folder, "Notes"))
    ));
    output.push_str(&format!("  note_id: {}\n", yaml_quote(note.id.trim())));
    output.push_str(&format!(
        "  locked: {}\n",
        if note.locked { "true" } else { "false" }
    ));
    output.push_str(&format!(
        "  content_state: {}\n",
        yaml_quote(if note.content_unavailable {
            if note.content_state.trim().is_empty() {
                "unavailable"
            } else {
                note.content_state.trim()
            }
        } else {
            "available"
        })
    ));
    output.push_str("---\n\n");
    output.push_str(&body);
    if !output.ends_with('\n') {
        output.push('\n');
    }
    Ok(output)
}

fn index_existing_imported_notes(
    vault: &ObsidianVault,
) -> AppleNotesImportResult<HashMap<String, ExistingImportedNote>> {
    let mut index = HashMap::new();
    for root in [
        vault.root().join("Imported"),
        vault.root().join(".raw").join("apple-notes"),
    ] {
        collect_existing_imported_notes(&root, &mut index)?;
    }
    Ok(index)
}

fn index_existing_imported_notes_under(
    root: &Path,
) -> AppleNotesImportResult<HashMap<String, ExistingImportedNote>> {
    let mut index = HashMap::new();
    collect_existing_imported_notes(root, &mut index)?;
    Ok(index)
}

fn collect_existing_imported_notes(
    root: &Path,
    index: &mut HashMap<String, ExistingImportedNote>,
) -> AppleNotesImportResult<()> {
    if !root.exists() {
        return Ok(());
    }

    let children = fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?;
    for child in children {
        let path = child.path();
        if path.is_dir() {
            collect_existing_imported_notes(&path, index)?;
            continue;
        }
        if !is_markdown_path(&path) {
            continue;
        }
        let Some(imported_note) = parse_existing_imported_note(&path)? else {
            continue;
        };
        let replace = index
            .get(&imported_note.note_id)
            .map(|current| should_replace_existing_import(current, &imported_note))
            .unwrap_or(true);
        if replace {
            index.insert(imported_note.note_id.clone(), imported_note);
        }
    }

    Ok(())
}

fn parse_existing_imported_note(
    path: &Path,
) -> AppleNotesImportResult<Option<ExistingImportedNote>> {
    let body = fs::read_to_string(path)?;
    let Some(frontmatter) = parse_apple_note_frontmatter(&body) else {
        return Ok(None);
    };
    if frontmatter.note_id.trim().is_empty() {
        return Ok(None);
    }
    let asset_dir = note_asset_directory(path);
    let source_asset_dir = asset_dir.exists().then_some(asset_dir.clone());
    let source_attachment_count = if source_asset_dir.is_some() {
        count_files_in_directory(&asset_dir)?
    } else {
        0
    };

    Ok(Some(ExistingImportedNote {
        note_id: frontmatter.note_id,
        account: frontmatter.account,
        folder: frontmatter.folder,
        updated: frontmatter.updated,
        content_state: frontmatter.content_state,
        source_note_path: path.to_path_buf(),
        source_asset_dir,
        source_attachment_count,
    }))
}

fn can_reuse_existing_note(
    existing: &ExistingImportedNote,
    metadata: &AppleScriptNoteMetadata,
) -> bool {
    existing.content_state == "available"
        && existing.account == metadata.account.trim()
        && existing.folder == metadata.folder.trim()
        && existing.updated == metadata.modified.trim()
}

fn reuse_existing_note(
    vault: &ObsidianVault,
    note_root_relative: &Path,
    existing: &ExistingImportedNote,
    attachment_count: &mut usize,
    reused_attachment_count: &mut usize,
    imported_at: DateTime<Utc>,
) -> AppleNotesImportResult<String> {
    let filename = existing
        .source_note_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Imported.md");
    let relative_path = note_root_relative
        .join(sanitize_directory_name(&existing.account))
        .join(sanitize_directory_name(&existing.folder))
        .join(filename);
    let resolved = vault.resolve_relative_path(&relative_path)?;
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&existing.source_note_path, &resolved)?;
    normalize_reused_note_freshness(&resolved, imported_at)?;

    if let Some(asset_dir) = &existing.source_asset_dir {
        let target_asset_dir = note_asset_directory(&resolved);
        copy_directory(asset_dir, &target_asset_dir)?;
        *attachment_count += existing.source_attachment_count;
        *reused_attachment_count += existing.source_attachment_count;
    }

    Ok(path_to_relative_string(&relative_path))
}

fn normalize_reused_note_freshness(
    note_path: &Path,
    imported_at: DateTime<Utc>,
) -> AppleNotesImportResult<()> {
    let body = fs::read_to_string(note_path)?;
    let Some((frontmatter, remainder)) = split_frontmatter(&body) else {
        return Ok(());
    };

    let source_created_at = parse_frontmatter_value(frontmatter, "source_created_at")
        .or_else(|| parse_frontmatter_value(frontmatter, "created"))
        .unwrap_or_default();
    let source_updated_at = parse_frontmatter_value(frontmatter, "source_updated_at")
        .or_else(|| parse_frontmatter_value(frontmatter, "updated"))
        .unwrap_or_default();

    let frontmatter = upsert_frontmatter_pair(
        frontmatter,
        "vault_created_at",
        &parse_frontmatter_value(frontmatter, "vault_created_at")
            .unwrap_or_else(|| imported_at.to_rfc3339()),
    );
    let frontmatter =
        upsert_frontmatter_pair(&frontmatter, "vault_touched_at", &imported_at.to_rfc3339());
    let frontmatter =
        upsert_frontmatter_pair(&frontmatter, "indexed_at", &imported_at.to_rfc3339());
    let frontmatter = if source_created_at.is_empty() {
        frontmatter
    } else {
        upsert_frontmatter_pair(&frontmatter, "source_created_at", &source_created_at)
    };
    let frontmatter = if source_updated_at.is_empty() {
        frontmatter
    } else {
        upsert_frontmatter_pair(&frontmatter, "source_updated_at", &source_updated_at)
    };

    fs::write(note_path, format!("---\n{frontmatter}\n---\n{remainder}"))?;
    Ok(())
}

fn split_frontmatter(body: &str) -> Option<(&str, &str)> {
    let trimmed = body.strip_prefix("---\n")?;
    let end = trimmed.find("\n---\n")?;
    let frontmatter = &trimmed[..end];
    let remainder = &trimmed[end + 5..];
    Some((frontmatter, remainder))
}

fn upsert_frontmatter_pair(frontmatter: &str, key: &str, value: &str) -> String {
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

fn parse_frontmatter_value(frontmatter: &str, key: &str) -> Option<String> {
    frontmatter
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}: ")))
        .map(|value| yaml_unquote(value.trim()))
}

fn copy_directory(source: &Path, destination: &Path) -> AppleNotesImportResult<()> {
    if !source.exists() {
        return Ok(());
    }

    fs::create_dir_all(destination)?;
    let children = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
    for child in children {
        let source_path = child.path();
        let target_path = destination.join(child.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn count_files_in_directory(root: &Path) -> AppleNotesImportResult<usize> {
    if !root.exists() {
        return Ok(0);
    }

    let mut count = 0;
    let children = fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?;
    for child in children {
        let path = child.path();
        if path.is_dir() {
            count += count_files_in_directory(&path)?;
        } else {
            count += 1;
        }
    }
    Ok(count)
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension().and_then(|value| value.to_str()) == Some("md")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedAppleNoteFrontmatter {
    account: String,
    folder: String,
    note_id: String,
    updated: String,
    content_state: String,
}

fn parse_apple_note_frontmatter(body: &str) -> Option<ParsedAppleNoteFrontmatter> {
    let mut lines = body.lines();
    if lines.next()? != "---" {
        return None;
    }

    let mut updated = String::new();
    let mut account = String::new();
    let mut folder = String::new();
    let mut note_id = String::new();
    let mut content_state = String::new();
    let mut in_source = false;

    for line in lines {
        if line == "---" {
            break;
        }
        if line == "source:" {
            in_source = true;
            continue;
        }

        if !line.starts_with(' ') {
            in_source = false;
        }

        if let Some(value) = line.strip_prefix("source_updated_at: ") {
            updated = yaml_unquote(value.trim());
            continue;
        }

        if let Some(value) = line.strip_prefix("updated: ") {
            updated = yaml_unquote(value.trim());
            continue;
        }

        if !in_source {
            continue;
        }

        if let Some(value) = line.strip_prefix("  account: ") {
            account = yaml_unquote(value.trim());
        } else if let Some(value) = line.strip_prefix("  folder: ") {
            folder = yaml_unquote(value.trim());
        } else if let Some(value) = line.strip_prefix("  note_id: ") {
            note_id = yaml_unquote(value.trim());
        } else if let Some(value) = line.strip_prefix("  content_state: ") {
            content_state = yaml_unquote(value.trim());
        }
    }

    if note_id.is_empty() {
        return None;
    }

    Some(ParsedAppleNoteFrontmatter {
        account,
        folder,
        note_id,
        updated,
        content_state,
    })
}

fn should_replace_existing_import(
    current: &ExistingImportedNote,
    candidate: &ExistingImportedNote,
) -> bool {
    if current.content_state != "available" && candidate.content_state == "available" {
        return true;
    }

    let current_is_raw = current
        .source_note_path
        .to_string_lossy()
        .contains("/.raw/apple-notes/");
    let candidate_is_raw = candidate
        .source_note_path
        .to_string_lossy()
        .contains("/.raw/apple-notes/");
    candidate_is_raw && !current_is_raw
}

fn yaml_unquote(value: &str) -> String {
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

fn yaml_quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    )
}

fn replace_inline_image_data_urls(
    html: &str,
    note_path: &Path,
    attachment_count: &mut usize,
) -> AppleNotesImportResult<String> {
    let mut remaining = html;
    let mut output = String::with_capacity(html.len());

    while let Some(position) = remaining.find("src=\"data:image/") {
        output.push_str(&remaining[..position]);
        let after_prefix = &remaining[position + 5..];
        let Some(end_quote) = after_prefix.find('"') else {
            output.push_str(&remaining[position..]);
            return Ok(output);
        };

        let data_url = &after_prefix[..end_quote];
        let replacement = match write_data_url_image(data_url, note_path, attachment_count)? {
            Some(path) => path,
            None => data_url.to_string(),
        };
        output.push_str("src=\"");
        output.push_str(&replacement);
        output.push('"');
        remaining = &after_prefix[end_quote + 1..];
    }

    output.push_str(remaining);
    Ok(output)
}

fn write_data_url_image(
    data_url: &str,
    note_path: &Path,
    attachment_count: &mut usize,
) -> AppleNotesImportResult<Option<String>> {
    let Some((header, encoded)) = data_url.split_once(',') else {
        return Ok(None);
    };
    if !header.starts_with("data:image/") || !header.contains(";base64") {
        return Ok(None);
    }

    let extension = header
        .trim_start_matches("data:image/")
        .split(';')
        .next()
        .map(sanitize_image_extension)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "png".to_string());

    let asset_dir = note_asset_directory(note_path);
    fs::create_dir_all(&asset_dir)?;
    *attachment_count += 1;
    let filename = format!("attachment-{:03}.{}", *attachment_count, extension);
    let asset_path = asset_dir.join(&filename);
    fs::write(&asset_path, BASE64.decode(encoded)?)?;

    let asset_directory_name = asset_dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("attachments");
    Ok(Some(format!("{asset_directory_name}/{filename}")))
}

fn note_asset_directory(note_path: &Path) -> PathBuf {
    let stem = note_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Note");
    note_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}.assets"))
}

fn sanitize_image_extension(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{
        AppleNotesFolderScan, AppleNotesImportManifest, AppleNotesImportStatus,
        AppleNotesScanReport, AppleScriptNote, note_asset_directory, parse_apple_note_frontmatter,
        publish_apple_notes, render_apple_note_markdown,
    };
    use crate::vault::ObsidianVault;
    use chrono::Utc;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn renders_apple_note_frontmatter_and_inline_images() {
        let root = temp_dir("apple-notes-markdown");
        let note_path = root.join("Imported/Apple Notes/iCloud/Test/Test.md");
        fs::create_dir_all(note_path.parent().expect("note parent")).expect("parent directory");
        let note = AppleScriptNote {
            title: "Test".to_string(),
            content: "<p>Hello</p><img src=\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=\"/>".to_string(),
            folder: "Test".to_string(),
            account: "iCloud".to_string(),
            id: "note-id".to_string(),
            created: "2026-04-12".to_string(),
            modified: "2026-04-12".to_string(),
            locked: false,
            content_unavailable: false,
            content_state: "available".to_string(),
        };

        let mut attachment_count = 0;
        let markdown =
            render_apple_note_markdown(&note, &note_path, &mut attachment_count, Utc::now())
                .expect("markdown");

        assert!(markdown.contains("imported/apple-notes"));
        assert!(markdown.contains("source:\n  system: apple_notes"));
        assert!(markdown.contains("source_created_at:"));
        assert!(markdown.contains("vault_created_at:"));
        assert!(markdown.contains("![](Test.assets/attachment-001.png)"));
        assert_eq!(attachment_count, 1);
        assert!(
            note_asset_directory(&note_path)
                .join("attachment-001.png")
                .exists()
        );
    }

    #[test]
    fn parses_existing_import_frontmatter_for_reuse() {
        let body = r#"---
title: "Test"
updated: "Sunday, 12 September 2021 at 22:44:44"
tags:
  - imported/apple-notes
source:
  system: apple_notes
  account: "Google"
  folder: "Notes"
  note_id: "x-coredata://example"
  locked: false
  content_state: "available"
---

Body
"#;

        let frontmatter = parse_apple_note_frontmatter(body).expect("frontmatter should parse");
        assert_eq!(frontmatter.account, "Google");
        assert_eq!(frontmatter.folder, "Notes");
        assert_eq!(frontmatter.note_id, "x-coredata://example");
        assert_eq!(frontmatter.content_state, "available");
        assert_eq!(frontmatter.updated, "Sunday, 12 September 2021 at 22:44:44");
    }

    #[test]
    fn long_note_paths_still_allow_asset_directories() {
        let root = temp_dir("apple-notes-long-title");
        let vault = ObsidianVault::from_root(&root);
        let title = "L".repeat(400);
        let relative_path = vault
            .allocate_note_path(Path::new("Imported/Apple Notes/iCloud/Test"), &title)
            .expect("allocated note path");
        let note_path = root.join(&relative_path);
        fs::create_dir_all(note_path.parent().expect("note parent")).expect("parent directory");

        let note = AppleScriptNote {
            title,
            content: "<p>Hello</p><img src=\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=\"/>".to_string(),
            folder: "Test".to_string(),
            account: "iCloud".to_string(),
            id: "note-id".to_string(),
            created: "2026-04-12".to_string(),
            modified: "2026-04-12".to_string(),
            locked: false,
            content_unavailable: false,
            content_state: "available".to_string(),
        };

        let mut attachment_count = 0;
        render_apple_note_markdown(&note, &note_path, &mut attachment_count, Utc::now())
            .expect("markdown");

        let asset_dir = note_asset_directory(&note_path);
        let asset_dir_name = asset_dir
            .file_name()
            .and_then(|value| value.to_str())
            .expect("asset directory name");
        assert!(asset_dir_name.len() <= 255);
        assert!(asset_dir.join("attachment-001.png").exists());
    }

    #[test]
    fn publishes_completed_raw_run_into_visible_import_root() {
        let root = temp_dir("apple-notes-publish");
        let vault = ObsidianVault::from_root(&root);
        vault.ensure_root().expect("vault root");

        let run_id = "20260413T090841Z";
        let raw_note_relative = PathBuf::from(format!(
            ".raw/apple-notes/{run_id}/notes/iCloud/Notes/Test.md"
        ));
        let raw_note_path = root.join(&raw_note_relative);
        fs::create_dir_all(raw_note_path.parent().expect("raw note parent"))
            .expect("raw note parent");
        fs::write(
            &raw_note_path,
            r#"---
title: "Test"
source_created_at: "2026-04-10T08:12:14Z"
source_updated_at: "2026-04-11T19:42:03Z"
tags:
  - imported/apple-notes
source:
  system: apple_notes
  account: "iCloud"
  folder: "Notes"
  note_id: "x-coredata://example"
  locked: false
  content_state: "available"
---

Body
"#,
        )
        .expect("raw note");
        let raw_asset_dir = note_asset_directory(&raw_note_path);
        fs::create_dir_all(&raw_asset_dir).expect("raw asset dir");
        fs::write(raw_asset_dir.join("attachment-001.png"), "png").expect("raw attachment");

        let manifest_relative = PathBuf::from(format!(".raw/apple-notes/{run_id}/manifest.json"));
        let manifest = AppleNotesImportManifest {
            manifest_version: 1,
            source_system: "apple_notes".to_string(),
            status: AppleNotesImportStatus::Completed,
            run_id: run_id.to_string(),
            imported_at: Utc::now(),
            raw_root: format!(".raw/apple-notes/{run_id}"),
            note_root: format!(".raw/apple-notes/{run_id}/notes"),
            scan: AppleNotesScanReport {
                account_count: 1,
                folder_count: 1,
                note_count: 1,
                folders: vec![AppleNotesFolderScan {
                    account: "iCloud".to_string(),
                    folder: "Notes".to_string(),
                    note_count: 1,
                }],
            },
            note_count: 1,
            attachment_count: 1,
            reused_note_count: 0,
            reused_attachment_count: 0,
            locked_note_count: 0,
            timed_out_note_count: 0,
        };
        fs::write(
            root.join(&manifest_relative),
            serde_json::to_string_pretty(&manifest).expect("manifest json"),
        )
        .expect("manifest");

        let report = publish_apple_notes(&vault, Some(run_id)).expect("publish");
        assert_eq!(report.source_run_id, run_id);
        assert_eq!(report.note_count, 1);
        assert_eq!(report.attachment_count, 1);
        assert_eq!(report.created_note_count, 1);
        assert_eq!(report.updated_note_count, 0);
        assert!(root.join(PathBuf::from(&report.report_path)).exists());

        let published_note = root.join("Imported/Apple Notes/iCloud/Notes/Test.md");
        assert!(published_note.exists());
        let published_body = fs::read_to_string(&published_note).expect("published note body");
        assert!(published_body.contains("kind: \"source_capture\""));
        assert!(published_body.contains("provenance: \"imported_source\""));
        assert!(published_body.contains("review_state: \"unreviewed\""));
        assert!(published_body.contains("managed_by: \"apple_notes_publish\""));
        assert!(published_body.contains(&format!(
            "raw_note_path: \"{}\"",
            raw_note_relative.display()
        )));
        assert!(
            note_asset_directory(&published_note)
                .join("attachment-001.png")
                .exists()
        );
    }

    #[test]
    fn republishes_existing_note_id_in_place() {
        let root = temp_dir("apple-notes-republish");
        let vault = ObsidianVault::from_root(&root);
        vault.ensure_root().expect("vault root");

        let existing_note = root.join("Imported/Apple Notes/iCloud/Notes/Old Name.md");
        fs::create_dir_all(existing_note.parent().expect("existing note parent"))
            .expect("existing note parent");
        fs::write(
            &existing_note,
            r#"---
title: "Old Name"
source:
  system: apple_notes
  account: "iCloud"
  folder: "Notes"
  note_id: "x-coredata://same-note"
  locked: false
  content_state: "available"
updated: "2026-04-11T19:42:03Z"
---

Old body
"#,
        )
        .expect("existing note");

        let run_id = "20260413T090842Z";
        let raw_note_path = root.join(format!(
            ".raw/apple-notes/{run_id}/notes/iCloud/Notes/New Name.md"
        ));
        fs::create_dir_all(raw_note_path.parent().expect("raw note parent"))
            .expect("raw note parent");
        fs::write(
            &raw_note_path,
            r#"---
title: "New Name"
source:
  system: apple_notes
  account: "iCloud"
  folder: "Notes"
  note_id: "x-coredata://same-note"
  locked: false
  content_state: "available"
updated: "2026-04-12T19:42:03Z"
---

Updated body
"#,
        )
        .expect("raw note");

        let manifest = AppleNotesImportManifest {
            manifest_version: 1,
            source_system: "apple_notes".to_string(),
            status: AppleNotesImportStatus::Completed,
            run_id: run_id.to_string(),
            imported_at: Utc::now(),
            raw_root: format!(".raw/apple-notes/{run_id}"),
            note_root: format!(".raw/apple-notes/{run_id}/notes"),
            scan: AppleNotesScanReport {
                account_count: 1,
                folder_count: 1,
                note_count: 1,
                folders: vec![AppleNotesFolderScan {
                    account: "iCloud".to_string(),
                    folder: "Notes".to_string(),
                    note_count: 1,
                }],
            },
            note_count: 1,
            attachment_count: 0,
            reused_note_count: 0,
            reused_attachment_count: 0,
            locked_note_count: 0,
            timed_out_note_count: 0,
        };
        let manifest_path = root.join(format!(".raw/apple-notes/{run_id}/manifest.json"));
        fs::write(
            manifest_path,
            serde_json::to_string_pretty(&manifest).expect("manifest json"),
        )
        .expect("manifest");

        let report = publish_apple_notes(&vault, Some(run_id)).expect("publish");
        assert_eq!(report.created_note_count, 0);
        assert_eq!(report.updated_note_count, 1);
        assert!(existing_note.exists());
        assert!(
            !root
                .join("Imported/Apple Notes/iCloud/Notes/New Name.md")
                .exists()
        );
        let published_body = fs::read_to_string(existing_note).expect("existing note body");
        assert!(published_body.contains("Updated body"));
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("prio-apple-notes-{label}-{nonce}"));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }
}
