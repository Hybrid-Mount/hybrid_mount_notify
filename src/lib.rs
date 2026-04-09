// Copyright 2026 Hybrid Mount Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    env, fs,
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

    let file_path = find_first_zip(&request.output_dir)?;
    let file_name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_size =
        fs::metadata(&file_path).map(|meta| meta.len()).unwrap_or(0) as f64 / 1024.0 / 1024.0;

    println!("Selecting yield: {} ({:.2} MB)", file_name, file_size);

    let (commit_msg, commit_hash) = get_git_commit();
    let safe_commit_msg = escape_html(&commit_msg);
    let commit_link = format!("{}/{}/commit/{}", server_url, repo, commit_hash);

    let caption = format!(
        "🌾 <b>Hybrid-Mount: {}</b>\n\n\
        🌿 <b>分支 (Branch):</b> {}\n\n\
        ⚖️ <b>重量 (Weight):</b> {:.2} MB\n\n\
        📝 <b>新性状 (Commit):</b>\n\
        <pre>{}</pre>\n\n\
        🚜 <a href='{}'>查看日志 (View Log)</a>",
        request.event_label, branch_name, file_size, safe_commit_msg, commit_link
    );

    println!("Dispatching yield to Granary (Telegram)...");

    let bot = Client::new(bot_token)?;
    let mut action = SendDocument::new(chat_id, InputFile::path(file_path).await?)
        .with_caption_parse_mode(tgbot::types::ParseMode::Html);

    if let Some(topic_id) = request.topic_id {
        action = action.with_message_thread_id(topic_id);
    }

    let action = if caption.len() < 1024 {
        action.with_caption(&caption)
    } else {
        action.with_caption(commit_link)
    };
    bot.execute(action).await?;

    Ok(())
}

fn find_first_zip(output_dir: &Path) -> Result<PathBuf> {
    let entries = fs::read_dir(output_dir)
        .with_context(|| format!("failed to read output directory {}", output_dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "zip") {
            return Ok(path);
        }
    }

    bail!("no zip files found in {}", output_dir.display())
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
