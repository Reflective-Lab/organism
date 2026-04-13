use std::fs;
use std::path::{Path, PathBuf};

use crate::vault::{ObsidianVault, VaultError, VaultPipelineStage, path_to_relative_string};
use chrono::{DateTime, Utc};
use organism_intelligence::provenance::CallContext;
use organism_intelligence::web::{
    HttpWebCaptureProvider, WebCaptureProvider, WebCaptureRequest, WebDocument, WebLink,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SNAPSHOT_NOTE_DIRECTORY: &str = "Inbox/Web Snapshots";
const SNAPSHOT_NOTE_TITLE_LIMIT: usize = 96;

#[derive(Debug, Error)]
pub enum WebSnapshotError {
    #[error("url is required")]
    MissingUrl,
    #[error("web capture failed: {0}")]
    Capture(String),
    #[error(transparent)]
    Vault(#[from] VaultError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type WebSnapshotResult<T> = Result<T, WebSnapshotError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSnapshotCaptureReport {
    pub run_id: String,
    pub raw_root: String,
    pub manifest_path: String,
    pub metadata_path: String,
    pub body_path: String,
    pub note_path: String,
    pub requested_url: String,
    pub canonical_url: String,
    pub title: String,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSnapshotManifest {
    pub manifest_version: u32,
    pub source_system: String,
    pub run_id: String,
    pub captured_at: DateTime<Utc>,
    pub raw_root: String,
    pub requested_url: String,
    pub canonical_url: String,
    pub title: String,
    pub description: Option<String>,
    pub site_name: Option<String>,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub metadata_path: String,
    pub body_path: String,
    pub note_path: String,
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSnapshotMetadata {
    pub requested_url: String,
    pub final_url: String,
    pub canonical_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_name: Option<String>,
    pub content_type: Option<String>,
    pub status_code: u16,
    pub links: Vec<WebLink>,
    pub provider: String,
}

pub fn capture_url_snapshot(
    vault: &ObsidianVault,
    url: &str,
) -> WebSnapshotResult<WebSnapshotCaptureReport> {
    let provider = HttpWebCaptureProvider::new().map_err(WebSnapshotError::Capture)?;
    capture_url_snapshot_with_provider(vault, url, &provider)
}

pub fn capture_url_snapshot_with_provider<P>(
    vault: &ObsidianVault,
    url: &str,
    provider: &P,
) -> WebSnapshotResult<WebSnapshotCaptureReport>
where
    P: WebCaptureProvider,
{
    let url = url.trim();
    if url.is_empty() {
        return Err(WebSnapshotError::MissingUrl);
    }

    vault.ensure_root()?;
    let capture = provider
        .capture(&WebCaptureRequest::new(url), &CallContext::default())
        .map_err(WebSnapshotError::Capture)?;
    let document = capture.capture.content;
    let captured_at = Utc::now();
    let raw_run = vault.prepare_pipeline_run(VaultPipelineStage::Raw, "web")?;

    let raw_root = path_to_relative_string(&raw_run.relative_root);
    let manifest_relative_path = raw_run.relative_root.join("manifest.json");
    let metadata_relative_path = raw_run.relative_root.join("metadata.json");
    let body_relative_path = raw_run
        .relative_root
        .join(snapshot_body_filename(document.content_type.as_deref()));
    let note_relative_path = allocate_snapshot_note_path(vault, &document, captured_at)?;
    let note_path = path_to_relative_string(&note_relative_path);
    let title = snapshot_title(&document);
    let canonical_url = document
        .canonical_url
        .clone()
        .unwrap_or_else(|| document.final_url.clone());

    write_text_file(vault, &body_relative_path, &document.body)?;
    write_text_file(
        vault,
        &metadata_relative_path,
        &serde_json::to_string_pretty(&WebSnapshotMetadata {
            requested_url: document.requested_url.clone(),
            final_url: document.final_url.clone(),
            canonical_url: document.canonical_url.clone(),
            title: document.title.clone(),
            description: document.description.clone(),
            site_name: document.site_name.clone(),
            content_type: document.content_type.clone(),
            status_code: document.status_code,
            links: document.links.clone(),
            provider: provider.name().to_string(),
        })?,
    )?;

    write_text_file(
        vault,
        &manifest_relative_path,
        &serde_json::to_string_pretty(&WebSnapshotManifest {
            manifest_version: 1,
            source_system: "web_capture".to_string(),
            run_id: raw_run.run_id.clone(),
            captured_at,
            raw_root: raw_root.clone(),
            requested_url: document.requested_url.clone(),
            canonical_url: canonical_url.clone(),
            title: title.clone(),
            description: document.description.clone(),
            site_name: document.site_name.clone(),
            status_code: document.status_code,
            content_type: document.content_type.clone(),
            metadata_path: path_to_relative_string(&metadata_relative_path),
            body_path: path_to_relative_string(&body_relative_path),
            note_path: note_path.clone(),
            provider: provider.name().to_string(),
        })?,
    )?;

    write_text_file(
        vault,
        &note_relative_path,
        &render_snapshot_note(
            &document,
            &title,
            captured_at,
            &path_to_relative_string(&manifest_relative_path),
            &path_to_relative_string(&metadata_relative_path),
            &path_to_relative_string(&body_relative_path),
        ),
    )?;

    Ok(WebSnapshotCaptureReport {
        run_id: raw_run.run_id,
        raw_root,
        manifest_path: path_to_relative_string(&manifest_relative_path),
        metadata_path: path_to_relative_string(&metadata_relative_path),
        body_path: path_to_relative_string(&body_relative_path),
        note_path,
        requested_url: document.requested_url,
        canonical_url,
        title,
        captured_at,
    })
}

fn write_text_file(vault: &ObsidianVault, path: &Path, body: &str) -> WebSnapshotResult<()> {
    let resolved = vault.resolve_relative_path(path)?;
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(resolved, body)?;
    Ok(())
}

fn allocate_snapshot_note_path(
    vault: &ObsidianVault,
    document: &WebDocument,
    captured_at: DateTime<Utc>,
) -> WebSnapshotResult<PathBuf> {
    let note_title = format!(
        "{} {}",
        captured_at.format("%Y-%m-%d"),
        truncate_title(&snapshot_title(document), SNAPSHOT_NOTE_TITLE_LIMIT)
    );
    Ok(vault.allocate_note_path(Path::new(SNAPSHOT_NOTE_DIRECTORY), &note_title)?)
}

fn render_snapshot_note(
    document: &WebDocument,
    title: &str,
    captured_at: DateTime<Utc>,
    manifest_path: &str,
    metadata_path: &str,
    body_path: &str,
) -> String {
    let canonical_url = document
        .canonical_url
        .clone()
        .unwrap_or_else(|| document.final_url.clone());
    let mut lines = vec![
        "---".to_string(),
        "kind: web_snapshot".to_string(),
        format!("source_url: {}", yaml_string(&document.requested_url)),
        format!("canonical_url: {}", yaml_string(&canonical_url)),
        format!("captured_at: {}", yaml_string(&captured_at.to_rfc3339())),
        format!(
            "vault_created_at: {}",
            yaml_string(&captured_at.to_rfc3339())
        ),
        format!(
            "vault_touched_at: {}",
            yaml_string(&captured_at.to_rfc3339())
        ),
        format!("indexed_at: {}", yaml_string(&captured_at.to_rfc3339())),
        format!("raw_manifest_path: {}", yaml_string(manifest_path)),
        format!("raw_metadata_path: {}", yaml_string(metadata_path)),
        format!("raw_body_path: {}", yaml_string(body_path)),
        format!("status_code: {}", document.status_code),
    ];

    if let Some(content_type) = &document.content_type {
        lines.push(format!("content_type: {}", yaml_string(content_type)));
    }
    if let Some(site_name) = &document.site_name {
        lines.push(format!("site_name: {}", yaml_string(site_name)));
    }

    lines.extend([
        "---".to_string(),
        String::new(),
        format!("# {title}"),
        String::new(),
        format!("Source: [{canonical_url}]({canonical_url})"),
        format!("Captured: {}", captured_at.format("%Y-%m-%d %H:%M UTC")),
    ]);

    if let Some(description) = document
        .description
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.extend([String::new(), description.trim().to_string()]);
    }

    lines.extend([
        String::new(),
        "## Raw Snapshot".to_string(),
        String::new(),
        format!("- Manifest: `{manifest_path}`"),
        format!("- Metadata: `{metadata_path}`"),
        format!("- Body: `{body_path}`"),
    ]);

    lines.join("\n") + "\n"
}

fn snapshot_title(document: &WebDocument) -> String {
    document
        .title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::trim)
        .map(str::to_string)
        .or_else(|| {
            document
                .site_name
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(str::trim)
                .map(str::to_string)
        })
        .unwrap_or_else(|| title_from_url(&document.final_url))
}

fn title_from_url(url: &str) -> String {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url)
        .trim();
    if host.is_empty() {
        "Web Snapshot".to_string()
    } else {
        host.to_string()
    }
}

fn snapshot_body_filename(content_type: Option<&str>) -> &'static str {
    match content_type
        .unwrap_or_default()
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
    {
        "text/markdown" => "body.md",
        "text/plain" => "body.txt",
        "application/json" => "body.json",
        _ => "body.html",
    }
}

fn yaml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn truncate_title(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }

    let truncated = value.chars().take(limit).collect::<String>();
    truncated.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::{capture_url_snapshot_with_provider, snapshot_body_filename};
    use crate::vault::ObsidianVault;
    use organism_intelligence::web::StubWebCaptureProvider;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn captures_raw_snapshot_and_publishes_note_stub() {
        let root = temp_dir("web-snapshot");
        let vault = ObsidianVault::from_root(&root);
        let report = capture_url_snapshot_with_provider(
            &vault,
            "https://example.com",
            &StubWebCaptureProvider,
        )
        .expect("snapshot should succeed");

        assert!(report.raw_root.starts_with(".raw/web/"));
        assert!(root.join(PathBuf::from(&report.manifest_path)).exists());
        assert!(root.join(PathBuf::from(&report.metadata_path)).exists());
        assert!(root.join(PathBuf::from(&report.body_path)).exists());
        assert!(root.join(PathBuf::from(&report.note_path)).exists());

        let note = fs::read_to_string(root.join(PathBuf::from(&report.note_path)))
            .expect("note stub should be readable");
        assert!(note.contains("kind: web_snapshot"));
        assert!(note.contains(&report.manifest_path));
    }

    #[test]
    fn derives_body_filenames_from_content_type() {
        assert_eq!(snapshot_body_filename(Some("text/plain")), "body.txt");
        assert_eq!(
            snapshot_body_filename(Some("text/markdown; charset=utf-8")),
            "body.md"
        );
        assert_eq!(
            snapshot_body_filename(Some("application/json")),
            "body.json"
        );
        assert_eq!(
            snapshot_body_filename(Some("text/html; charset=utf-8")),
            "body.html"
        );
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
