// Copyright 2026 Hybrid Mount Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use tgbot::{
    api::Client,
    types::{InputFile, SendDocument},
};

#[derive(Debug, Clone)]
pub struct NotifyRequest {
    pub output_dir: PathBuf,
    pub topic_id: Option<i64>,
    pub event_label: String,
}

impl NotifyRequest {
    pub fn new(output_dir: impl Into<PathBuf>, event_label: impl Into<String>) -> Self {
        Self {
            output_dir: output_dir.into(),
            topic_id: None,
            event_label: event_label.into(),
        }
    }

    pub fn with_topic_id(mut self, topic_id: Option<i64>) -> Self {
        self.topic_id = topic_id;
        self
    }
}

pub fn maybe_send_output_dir_notification(request: &NotifyRequest) -> Result<bool> {
    if env::var("TELEGRAM_BOT_TOKEN")
        .ok()
        .filter(|v| !v.is_empty())
        .is_none()
        || env::var("TELEGRAM_CHAT_ID")
            .ok()
            .filter(|v| !v.is_empty())
            .is_none()
    {
        return Ok(false);
    }

    send_output_dir_notification(request)?;
    Ok(true)
}

pub fn send_output_dir_notification(request: &NotifyRequest) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("failed to create Tokio runtime")?;
    runtime.block_on(send_output_dir_notification_async(request))
}

async fn send_output_dir_notification_async(request: &NotifyRequest) -> Result<()> {
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN not set")?;
    let chat_id = env::var("TELEGRAM_CHAT_ID").context("TELEGRAM_CHAT_ID not set")?;

    let repo = env::var("GITHUB_REPOSITORY").unwrap_or_default();
    let server_url =
        env::var("GITHUB_SERVER_URL").unwrap_or_else(|_| "https://github.com".to_string());
    let branch_name = env::var("GITHUB_REF_NAME").unwrap_or_else(|_| get_git_branch());

    let artifacts = find_zip_files(&request.output_dir)?;
    let total_size = artifacts.iter().map(|artifact| artifact.size_bytes).sum();

    let (commit_msg, commit_hash) = get_git_commit();
    let safe_commit_msg = escape_html(&commit_msg);
    let commit_link = format!("{}/{}/commit/{}", server_url, repo, commit_hash);

    println!(
        "Selecting {} yield(s), total {:.2} MB",
        artifacts.len(),
        bytes_to_mib(total_size)
    );

    let bot = Client::new(bot_token)?;
    for (index, artifact) in artifacts.iter().enumerate() {
        println!(
            "Dispatching yield to Granary (Telegram): {} ({:.2} MB)",
            artifact.file_name,
            bytes_to_mib(artifact.size_bytes)
        );

        let caption = if index == 0 {
            build_primary_caption(
                request,
                &branch_name,
                artifacts.len(),
                total_size,
                &safe_commit_msg,
                &commit_link,
            )
        } else {
            build_extra_caption(request, index + 1, artifacts.len(), artifact)
        };

        let mut action = SendDocument::new(
            chat_id.clone(),
            InputFile::path(artifact.path.clone()).await?,
        )
        .with_caption_parse_mode(tgbot::types::ParseMode::Html);

        if let Some(topic_id) = request.topic_id {
            action = action.with_message_thread_id(topic_id);
        }

        let action = if caption.len() < 1024 {
            action.with_caption(&caption)
        } else {
            action.with_caption(&commit_link)
        };
        bot.execute(action).await?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct Artifact {
    path: PathBuf,
    file_name: String,
    size_bytes: u64,
}

fn find_zip_files(output_dir: &Path) -> Result<Vec<Artifact>> {
    let entries = fs::read_dir(output_dir)
        .with_context(|| format!("failed to read output directory {}", output_dir.display()))?;
    let mut artifacts = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if is_zip_path(&path) {
            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let size_bytes = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
            artifacts.push(Artifact {
                path,
                file_name,
                size_bytes,
            });
        }
    }

    artifacts.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    if artifacts.is_empty() {
        bail!("no zip files found in {}", output_dir.display());
    }

    Ok(artifacts)
}

fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}

fn build_primary_caption(
    request: &NotifyRequest,
    branch_name: &str,
    artifact_count: usize,
    total_size: u64,
    safe_commit_msg: &str,
    commit_link: &str,
) -> String {
    format!(
        "🌾 <b>Hybrid-Mount: {}</b>\n\n\
        🌿 <b>分支 (Branch):</b> {}\n\n\
        📦 <b>产物 (Artifacts):</b> {}\n\n\
        ⚖️ <b>总重 (Total Weight):</b> {:.2} MB\n\n\
        📝 <b>新性状 (Commit):</b>\n\
        <pre>{}</pre>\n\n\
        🚜 <a href='{}'>查看日志 (View Log)</a>",
        request.event_label,
        branch_name,
        artifact_count,
        bytes_to_mib(total_size),
        safe_commit_msg,
        commit_link
    )
}

fn build_extra_caption(
    request: &NotifyRequest,
    index: usize,
    artifact_count: usize,
    artifact: &Artifact,
) -> String {
    format!(
        "🌾 <b>Hybrid-Mount: {}</b>\n\n\
        📦 <b>产物 (Artifact):</b> {}/{}\n\n\
        <pre>{}</pre>\n\n\
        ⚖️ <b>重量 (Weight):</b> {:.2} MB",
        request.event_label,
        index,
        artifact_count,
        escape_html(&artifact.file_name),
        bytes_to_mib(artifact.size_bytes)
    )
}

fn bytes_to_mib(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

fn get_git_commit() -> (String, String) {
    let msg = Command::new("git")
        .args(["log", "-1", "--pretty=%B"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|msg| !msg.is_empty())
        .unwrap_or_else(|| "No commit message available.".to_string());

    let hash = Command::new("git")
        .args(["log", "-1", "--pretty=%H"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|hash| !hash.is_empty())
        .unwrap_or_else(|| "000000".to_string());

    (msg, hash)
}

fn get_git_branch() -> String {
    Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|branch| !branch.is_empty())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn find_zip_files_returns_all_zips_sorted() -> Result<()> {
        let output_dir = make_temp_output_dir()?;
        File::create(output_dir.join("Hybrid-Mount-lite.zip"))?;
        File::create(output_dir.join("Hybrid-Mount.zip"))?;
        File::create(output_dir.join("notes.txt"))?;

        let artifacts = find_zip_files(&output_dir)?;
        let names: Vec<_> = artifacts
            .iter()
            .map(|artifact| artifact.file_name.as_str())
            .collect();

        assert_eq!(names, vec!["Hybrid-Mount-lite.zip", "Hybrid-Mount.zip"]);

        fs::remove_dir_all(output_dir)?;
        Ok(())
    }

    fn make_temp_output_dir() -> Result<PathBuf> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let output_dir = env::temp_dir().join(format!("notify-test-{nanos}"));
        fs::create_dir_all(&output_dir)?;
        Ok(output_dir)
    }
}
