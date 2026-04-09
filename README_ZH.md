# Hybrid Mount Notify

![Language](https://img.shields.io/badge/Language-Rust-orange?style=flat-square&logo=rust)
![Platform](https://img.shields.io/badge/Platform-Linux-lightgrey?style=flat-square&logo=linux)
![License](https://img.shields.io/badge/License-GPL--3.0-blue?style=flat-square)

Hybrid Mount Notify 是 Hybrid Mount 构建与发布流程使用的 Telegram 通知工具。
它会扫描 `output/` 中生成的产物，读取当前代码仓库的 Git 元数据，再把压缩包发送到指定 Telegram 聊天或话题。

这个仓库同时保留两种使用方式：

- 作为适合 shell / CI 的轻量 CLI 二进制
- 作为可直接被 `xtask` 或其他内部工具调用的 Rust library

**[🇺🇸 English](README.md)**

---

## 目录

- [设计目标](#设计目标)
- [架构说明](#架构说明)
- [仓库结构](#仓库结构)
- [环境变量](#环境变量)
- [CLI 命令](#cli-命令)
- [库集成方式](#库集成方式)
- [构建方式](#构建方式)
- [开源协议](#开源协议)

---

## 设计目标

1. **单一职责**：专注于构建产物通知。
2. **低接入成本**：可被 shell、CI workflow 或 Rust 代码直接使用。
3. **元数据格式统一**：统一展示分支、提交与产物信息。
4. **低维护成本**：可独立于主仓库演进与升级。

## 架构说明

这个 crate 分成两层：

1. `src/lib.rs`：可复用的通知逻辑
2. `src/main.rs`：基于库封装的 CLI 入口

运行时主要流程如下：

1. 从环境变量读取 Telegram 凭据
2. 在 `output/` 中查找第一个 `.zip` 产物
3. 从 Git / CI 环境解析分支与提交信息
4. 以 HTML caption 的形式把压缩包发送到 Telegram

## 仓库结构

```text
.
├─ src/lib.rs           # 可复用通知 API
├─ src/main.rs          # CLI 入口
├─ Cargo.toml           # crate 元数据与依赖
├─ Cargo.lock           # 锁定依赖图
└─ README*.md           # 中英文文档
```

## 环境变量

必需变量：

| 变量 | 说明 |
| --- | --- |
| `TELEGRAM_BOT_TOKEN` | 用于上传文件的 Telegram Bot Token |
| `TELEGRAM_CHAT_ID` | 目标 Telegram Chat ID |

可选 CI 元数据：

| 变量 | 说明 |
| --- | --- |
| `GITHUB_REPOSITORY` | commit 链接中展示的仓库名 |
| `GITHUB_SERVER_URL` | GitHub 站点基地址 |
| `GITHUB_REF_NAME` | caption 中展示的分支或 tag 名 |

如果 GitHub 环境变量不存在，crate 会尽量回退到本地 `git` 命令解析信息。

## CLI 命令

```bash
notify [TOPIC_ID] [EVENT_LABEL]
```

示例：

```bash
# 使用默认事件名
notify

# 发送到指定 Telegram 话题
notify 37 "Daily Tilling - v3.4.5-123"
```

CLI 默认要求当前工作目录下存在 `output/` 目录，并且其中至少有一个 `.zip` 产物。

## 库集成方式

示例：

```rust
use notify::{NotifyRequest, maybe_send_output_dir_notification};

let request = NotifyRequest::new("output", "Daily Tilling - v3.4.5-123")
    .with_topic_id(Some(37));

let sent = maybe_send_output_dir_notification(&request)?;
if !sent {
    eprintln!("Telegram secrets not set, skipping notification");
}
```

如果调用方希望在凭据缺失时直接报错，可使用 `send_output_dir_notification`；如果更希望安静跳过，可使用 `maybe_send_output_dir_notification`。

## 构建方式

环境要求：

- 支持 Rust 2024 edition 的 stable 或 nightly 工具链

命令示例：

```bash
# debug 构建
cargo build

# release 构建
cargo build --release
```

## 开源协议

本项目采用 [GPL-3.0](LICENSE)。
