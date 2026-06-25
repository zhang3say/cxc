# CXC — Code Cross-Connect

简体中文 | [English](README_en.md)

> 为 Codex、Claude 等 AI 编程工具快速切换 API 中转站/代理端点。

[![Rust Version](https://img.shields.io/badge/rust-1.96%2B-orange)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-green)](LICENSE)

![CXC Desktop Preview](docs/images/cxc.webp)

CXC 是一个用于管理 AI 编程工具的多个 API 中转端点配置的桌面端 GUI 应用。无需手动编辑复杂的 TOML 和 JSON 配置文件，使用 CXC 即可在直观的图形界面中轻松添加、测试和一键切换服务商。

## 功能特性

- **直观的 GUI 界面** — 基于极简优雅的毛玻璃与暗色调视觉设计，操作直观。
- **全局环境切换 (App / WSL)** — App / WSL 写入环境一次切换即可全局生效，同时 Codex 与 Claude CLI 仍分别记录各自在 App、WSL 下的 Active Provider。
- **服务商管理** — 支持轻松添加、编辑、测试、删除服务商，还可以为服务商添加备注（Remark）。
- **模型发现 (Model Discovery)** — 一键从服务商端点自动拉取并选择可用模型，无需手动记忆或盲打输入。
- **连通性测试** — 采用真实的 API 连通性请求（并非简单的 ping）并测量延迟，确保服务有效。
- **系统托盘** — 支持常驻系统托盘，可一键快速切换当前激活的 Codex Provider，无需唤醒主界面。
- **安全切换** — 在修改目标工具配置文件之前自动创建 `.bak` 备份，防止配置丢失。
- **多目标工具集成** — 自动配置并无缝集成 Codex (`~/.codex/config.toml`) 和 Claude CLI (`~/.claude/settings.json`)。

## 安装与开发构建

### 开发调试

在本地运行开发版本的桌面端应用：

```bash
# 克隆仓库
git clone https://github.com/zhang3say/cxc
cd cxc

# 进入桌面端目录并安装依赖
cd cxc-desktop
npm install

# 启动开发调试环境
npm run tauri dev
```

### 生产打包

在本地构建发布版本的桌面端安装包：

```bash
cd cxc-desktop
npm run tauri build
```

构建生成的安装包将位于 `cxc-desktop/src-tauri/target/release/bundle/` 下。

## 工作原理

CXC 将其自身的服务商列表和配置存储在系统特定的配置目录中：
- Windows: `%USERPROFILE%\.config\cxc\config.yaml`
- Linux / WSL: `~/.config/cxc/config.yaml`

当您在 CXC 中切换服务商时，它会自动更新目标工具的配置：
- **Codex** — 更新配置目录下的 `config.toml` (设置 model, base_url, wire_api) 和 `auth.json` (设置 API Key)
- **Claude CLI** — 更新 `settings.json` 中的 `env` 对象 (设置 API Key、Base URL 及多模型映射关系)

在进行任何写入之前，目标配置文件都会被自动备份为 `.bak` 文件。

## 系统架构

- **后端 (Rust)**: 基于 [Tauri](https://github.com/tauri-apps/tauri) (v2) 框架，配合 [Tokio](https://github.com/tokio-rs/tokio) 异步运行时和 [Reqwest](https://github.com/seanmonstar/reqwest) 执行高性能、非阻塞 of API 连通性测试和文件读写操作。
- **前端 (React)**: 基于 React 19 和 TypeScript，采用 [Vite](https://github.com/vitejs/vite) 作为构建工具，样式库采用 [Tailwind CSS v4](https://tailwindcss.com) 结合 [Radix UI](https://www.radix-ui.com) 实现极具现代感、响应式且顺滑的交互界面。
- **保持格式的 AST 修改**: 读写 Codex 的 TOML 文件时使用 [toml_edit](https://github.com/toml-rs/toml)，能够精准修改目标值而保留原配置文件中的所有格式和注释。

关于详细的架构决策记录，请参见 [docs/adr/](docs/adr/)。


## 许可证

MIT
