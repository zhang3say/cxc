# ADR-0021: v2 配置与一次性迁移

## 背景

旧版保存配置时只序列化自己的 `Config` 结构，未知字段会被丢掉。如果新旧两版共用 `~/.config/cxc/config.yaml`，旧版任意一次保存都会清空新版写入的供应商和绑定。

## 决策

- 新版使用独立配置：`~/.config/cxc-v2/config.yaml`（Windows 为 `%USERPROFILE%\.config\cxc-v2\config.yaml`）。
- 首次启动时，如果新版配置不存在，读取旧版 `~/.config/cxc/config.yaml`，把三份中转站列表合并成全局供应商和 Agent 绑定，写入新版配置。
- 合并键是 `(规范化 base_url, api_key)`；协议取并集，模型取并集，旧激活名称映射到新绑定 id。
- 旧配置文件全程只读，旧版继续可用。
- 设置里提供「重新导入旧版配置」，把旧列表合并进已存在的新版配置，按 `(supplier_id, protocol, model)` 去重。
- Gist 同步载荷升到版本 2，包含供应商和四个 Agent 的绑定，不包含来源、自定义目录、激活状态和测试结果。拉取到版本 1 载荷时在内存中迁移成版本 2 再合并。

## 后果

- 两版可以并排安装、并排运行，互不覆盖。
- 新版替换旧版时，再做一次最终迁移并切换配置路径。
- 旧版无法读取新版配置，这是有意的隔离。

## 状态

已采纳。实现见 `cxc-core/src/config.rs`、`migration.rs`、`sync/mod.rs`。
