# ADR-0014: 基于 GitHub Gist 的配置跨设备同步与端到端加密设计

## 状态

已接受 (2026-09-12)

## 背景

CXC 作为一个管理 AI 编程工具中转站配置的桌面应用，用户常在多台设备（工作台式机、笔记本、Mac、Windows、WSL 等）之间切换。以往新增或修改 Provider 节点需要用户手动在多端重复录入，体验割裂。

同时，中转节点配置中包含高价值的明文 `api_key`，且不同设备具有不同的本地环境路径（如 Windows WSL UNC 路径），不能无脑覆盖。

## 决策

### 1. 同步后端选型

**决定：采用 GitHub Secret Gist 作为云端同步存储后端**

- **零额外服务器**：开发者用户天然拥有 GitHub 账号，通过具备 `gist` 权限的 Personal Access Token 即可直连。
- **自动化建仓**：若用户未指定 Gist ID，首次同步时客户端自动调用 GitHub API 创建 `public: false` 的 Secret Gist，并将 ID 回填本地。
- **单文件结构**：云端 Gist 中维护 `cxc_sync.json` 文件。

### 2. 同步数据边界（跨平台环境隔离）

**决定：严格区分跨设备通用数据与本地特异性数据**

- **同步内容**：
  - Codex、Claude、Grok 的全量 Provider 节点信息（`name`, `base_url`, `api_key`, `model`, `wire_api`, `remark`, `claude_models`, `last_test`, `latency_ms`, `last_ok`）。
- **不同步内容（保留本地）**：
  - 设备运行环境：`codex_source`, `claude_source`, `grok_source` (`app` 或 `wsl`)。
  - 本地路径配置：`codex_custom_dir`, `claude_custom_dir`, `grok_custom_dir`（WSL 路径跨操作系统无效）。
  - 各工具当前激活状态：`*_active_app`, `*_active_wsl`。

### 3. 端到端加密（E2EE）安全机制

**决定：支持基于主密码的 AES-256-GCM 本地加解密**

- 用户可设置同步主密码（Passphrase）。
- 密钥由 SHA-256 派生出 256 位密钥，并生成随机 12 字节 Nonce。
- 上传至云端的格式为加密 Envelope：
  - `encrypted: true` 时，仅包含 Base64 编码的密文与 Nonce。
  - 即便 Gist 意外泄露，无主密码也无法窃取明文 API Key。
- 用户亦可留空密码采用明文 JSON 结构。

### 4. 冲突检测与合并策略

**决定：提供智能合并、推送到云端、从云端拉取三种模式**

- **智能合并（Merge）**：按 Provider `name` 对齐，两端不重合的节点完整合并，重合节点优先采纳属性更新与最新测速记录。
- **安全备份机制**：在执行从云端拉取覆盖或合并前，本地自动备份 `config.yaml.backup.{timestamp}`，确保随时可回滚。

## 影响

- 用户仅需配置一次 GitHub Token 即可在多端无缝共享 Provider 列表。
- 保障了 API 密钥的安全性，同时避免不同操作系统之间的路径与状态冲突。
