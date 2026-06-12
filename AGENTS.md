# CXC — Agent Instructions
- 始终使用中文回复、交流、文档等。
## Agent skills

### Issue tracker

Issues and PRDs live as GitHub issues on `zhang3say/cxc`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default label vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context repo — one `CONTEXT.md` + `docs/adr/` at the root. See `docs/agents/domain.md`.

## Git 提交规范
- **会话结束前必须提交 Git**：在一场会话结束前（或单个完整需求开发并校验完成后），必须将所有有意义的改动（如修改代码、更新文档、新增功能或修复 Bug 等）进行一次完整的 Git 提交（commit）。避免在会话中途提交零碎或未经验证的改动。
- **提交信息规范**：提交信息必须为中文且应清晰、准确地描述所做的改动（例如使用 `feat:`, `fix:`, `docs:`, `style:`, `refactor:` 等标准前缀）。
