use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::models::{FormConfig, Profile, TaskQueue};

/// Get the data directory (./data/), creating it if needed
pub fn data_dir() -> Result<PathBuf> {
    let data_dir = PathBuf::from("data");
    fs::create_dir_all(&data_dir).context("Failed to create data directory")?;
    Ok(data_dir)
}

fn profiles_file() -> Result<PathBuf> {
    Ok(data_dir()?.join("profiles.json"))
}

fn forms_file() -> Result<PathBuf> {
    Ok(data_dir()?.join("forms.json"))
}

// --- Profile Storage ---

pub fn load_profiles() -> Result<Vec<Profile>> {
    let path = profiles_file()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path).context("Failed to read profiles file")?;

    let profiles: Vec<Profile> =
        serde_json::from_str(&content).context("Failed to parse profiles")?;

    Ok(profiles)
}

pub fn save_profiles(profiles: &[Profile]) -> Result<()> {
    let path = profiles_file()?;
    let content = serde_json::to_string_pretty(profiles).context("Failed to serialize profiles")?;
    fs::write(&path, content).context("Failed to write profiles file")?;
    Ok(())
}

// --- Form Config Storage ---

pub fn load_form_configs() -> Result<Vec<FormConfig>> {
    let path = forms_file()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path).context("Failed to read forms file")?;

    let forms: Vec<FormConfig> =
        serde_json::from_str(&content).context("Failed to parse form configs")?;

    Ok(forms)
}

pub fn save_form_configs(forms: &[FormConfig]) -> Result<()> {
    let path = forms_file()?;
    let content = serde_json::to_string_pretty(forms).context("Failed to serialize form configs")?;
    fs::write(&path, content).context("Failed to write forms file")?;
    Ok(())
}

// --- Task Queue Storage ---

fn tasks_file() -> Result<PathBuf> {
    Ok(data_dir()?.join("tasks.json"))
}

pub fn load_tasks() -> Result<TaskQueue> {
    let path = tasks_file()?;

    if !path.exists() {
        return Ok(TaskQueue::new());
    }

    let content = fs::read_to_string(&path).context("Failed to read tasks file")?;

    let queue: TaskQueue =
        serde_json::from_str(&content).context("Failed to parse tasks")?;

    Ok(queue)
}

pub fn save_tasks(queue: &TaskQueue) -> Result<()> {
    let path = tasks_file()?;
    let content = serde_json::to_string_pretty(queue).context("Failed to serialize tasks")?;
    fs::write(&path, content).context("Failed to write tasks file")?;
    Ok(())
}
