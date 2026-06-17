# CXC — Code Cross-Connect

简体中文 | [English](README_en.md)

> 为 Codex、Claude 等 AI 编程工具快速切换 API 中转站/代理端点。

[![Rust Version](https://img.shields.io/badge/rust-1.96%2B-orange)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-green)](LICENSE)

![CXC TUI Preview](docs/images/cxc.webp)

CXC 是一个用于管理 AI 编程工具的多个 API 中转端点配置的 CLI/TUI 工具。无需手动编辑 TOML 和 JSON 配置文件，使用 `cxc` 即可在几秒钟内添加、测试和切换服务商。

## 功能特性

- **CLI 模式** — 支持脚本化子命令，便于自动化
- **TUI 模式** — 全屏交互式界面（使用 `cxc` 启动）
- **服务商管理** — 支持添加、列表、测试、切换、删除
- **连通性测试** — 使用真实的聊天补全请求并测量延迟
- **安全切换** — 在修改任何配置文件之前自动创建 `.bak` 备份
- **Codex 集成** — 自动更新 `~/.codex/config.toml` 和 `~/.codex/auth.json`

## 安装

### 一键安装脚本 (Linux / macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/zhang3say/cxc/main/install.sh | sh
```

### 使用 Cargo 安装

```bash
cargo install --git https://github.com/zhang3say/cxc.git
```

### 从源码编译

```bash
git clone https://github.com/zhang3say/cxc
cd cxc
cargo build --release
```

## 使用方法

### TUI 模式

```bash
cxc
```

启动全屏交互式 TUI。快捷键如下：

| 按键 | 动作 / 功能 |
|-----|--------|
| `↑`/`↓` | 浏览/选择服务商 |
| `a` | 添加新服务商 |
| `e` | 编辑选中的服务商 |
| `t` | 测试选中的服务商连通性 |
| `T` | 并发测试所有服务商连通性 |
| `Enter`/`s` | 切换到选中的服务商 |
| `d`/`Delete` | 删除选中的服务商 |
| `q`/`Esc` | 退出 |

### CLI 模式

```bash
# 添加服务商（省略参数则进入交互式输入）
cxc provider add --name my-relay \
  --base-url https://api.example.com/v1 \
  --api-key sk-xxx \
  --model gpt-4

# 列出所有服务商
cxc provider list

# 测试服务商连通性
cxc provider test               # 测试当前激活的服务商
cxc provider test my-relay      # 测试指定名称的服务商
cxc provider test --all         # 并发测试所有已保存的服务商（或使用 -a）

# 切换当前激活的服务商
cxc provider switch my-relay

# 删除服务商
cxc provider remove my-relay
```

## 工作原理

CXC 将其自身的服务商列表存储在 `~/.config/cxc/config.yaml`（权限为 0600）。

当您切换服务商时，CXC 会更新：
- `~/.codex/config.toml` — 设置 `model`、`model_providers.codex.base_url`、`wire_api`
- `~/.codex/auth.json` — 设置 `OPENAI_API_KEY`

在进行任何写入之前，这两个文件都会被自动备份为 `.bak`。

## 配置说明

CXC 自身的配置文件位于 `~/.config/cxc/config.yaml`：

```yaml
active: my-relay
providers:
  - name: my-relay
    base_url: https://api.example.com/v1
    api_key: sk-xxx
    model: gpt-4
    wire_api: responses
```

## 系统架构

- **CLI**: [Clap](https://github.com/clap-rs/clap)（派生接口）— 子命令路由
- **TUI**: [Ratatui](https://github.com/ratatui-org/ratatui)（使用 [crossterm](https://github.com/crossterm-rs/crossterm) 后端）— 异步事件循环 TUI
- **TOML**: [toml_edit](https://github.com/toml-rs/toml) — 保持格式的 AST 修改
- **配置序列化**: 基于 [serde_yaml](https://github.com/dtolnay/serde-yaml) 的 YAML 序列化

关于架构决策记录，请参见 [docs/adr/](docs/adr/)。

## 许可证

MIT
