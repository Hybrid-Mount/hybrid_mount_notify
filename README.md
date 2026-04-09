# Hybrid Mount Notify

![Language](https://img.shields.io/badge/Language-Rust-orange?style=flat-square&logo=rust)
![Platform](https://img.shields.io/badge/Platform-Linux-lightgrey?style=flat-square&logo=linux)
![License](https://img.shields.io/badge/License-GPL--3.0-blue?style=flat-square)

Hybrid Mount Notify is the Telegram notification helper used by the Hybrid Mount build and release pipeline.
It scans `output/` for the generated artifact, collects Git metadata from the current checkout, and sends the package to a Telegram chat or topic.

It is intentionally usable in two ways:

- as a small CLI binary for shell-based automation
- as a Rust library that can be called directly from `xtask` or other internal tooling

**[🇨🇳 中文文档](README_ZH.md)**

---

## Table of Contents

- [Design Goals](#design-goals)
- [Architecture](#architecture)
- [Repository Layout](#repository-layout)
- [Environment](#environment)
- [CLI](#cli)
- [Library Integration](#library-integration)
- [Build](#build)
- [License](#license)

---

## Design Goals

1. **Single-purpose automation** for build artifact delivery.
2. **Low ceremony integration** from shell scripts, CI workflows, or Rust code.
3. **Consistent metadata formatting** across branch, commit, and artifact reporting.
4. **Small maintenance surface** so it can evolve independently from the main repository.

## Architecture

The crate is split into two layers:

1. `src/lib.rs` contains the reusable notification logic.
2. `src/main.rs` provides a CLI wrapper around that library.

At runtime, the tool:

1. reads Telegram credentials from environment variables
2. scans `output/` for the first `.zip` artifact
3. resolves branch and commit metadata from Git / CI environment
4. uploads the archive to Telegram with an HTML caption

## Repository Layout

```text
.
├─ src/lib.rs           # reusable notification API
├─ src/main.rs          # CLI entrypoint
├─ Cargo.toml           # crate metadata and dependencies
├─ Cargo.lock           # locked dependency graph
└─ README*.md           # English and Chinese docs
```

## Environment

Required variables:

| Key | Description |
| --- | --- |
| `TELEGRAM_BOT_TOKEN` | Telegram bot token used for upload |
| `TELEGRAM_CHAT_ID` | target Telegram chat id |

Optional CI metadata:

| Key | Description |
| --- | --- |
| `GITHUB_REPOSITORY` | repository shown in commit link |
| `GITHUB_SERVER_URL` | GitHub host base URL |
| `GITHUB_REF_NAME` | branch or tag name shown in caption |

If GitHub variables are missing, the crate falls back to local `git` commands where possible.

## CLI

```bash
notify [TOPIC_ID] [EVENT_LABEL]
```

Examples:

```bash
# use default event label
notify

# send into a Telegram topic
notify 37 "Daily Tilling - v3.4.5-123"
```

The CLI expects the current working directory to contain an `output/` folder with at least one `.zip` artifact.

## Library Integration

Example:

```rust
use notify::{NotifyRequest, maybe_send_output_dir_notification};

let request = NotifyRequest::new("output", "Daily Tilling - v3.4.5-123")
    .with_topic_id(Some(37));

let sent = maybe_send_output_dir_notification(&request)?;
if !sent {
    eprintln!("Telegram secrets not set, skipping notification");
}
```

Use `send_output_dir_notification` when missing credentials should be treated as an error, and `maybe_send_output_dir_notification` when callers prefer a clean skip.

## Build

Prerequisites:

- Rust stable or nightly with edition 2024 support

Commands:

```bash
# debug build
cargo build

# optimized build
cargo build --release
```

## License

Licensed under [GPL-3.0](LICENSE).
