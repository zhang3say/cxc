# 9. 使用固定 model_providers key 写入 Codex 配置

## Status

Accepted

## Context

CXC 管理多个中转 Provider（如"布丁"、"anyRouter"），切换时需要写入 Codex 的 `config.toml`。Codex 的配置结构是：

```toml
model_provider = "<active_key>"

[model_providers.<key>]
base_url = "..."
name = "..."
```

`model_provider` 顶层字段指向 `model_providers` 表中某个 key，Codex 才会真正使用该 provider。

我们面临的选择是：用 CXC Provider 的名字作为 TOML key（动态 key），还是使用固定 key。

## Decision

始终使用固定 key `"codex"` 写入 `model_providers.codex`，并将 `model_provider` 设为 `"codex"`。

## Consequences

- CXC Provider 名称不受字符限制（中文、emoji 等均可），因为它们不会出现在 Codex 的 TOML key 中。
- 不会在 config.toml 里累积大量废弃的 provider entries。
- 切换中转时会覆盖用户手动在 Codex Desktop 中设置的 `model_provider` 值——这是预期行为，用户主动切换中转即表达了覆盖意图。
- 如果用户之后在 Codex Desktop 中手动改回官方 API，CXC 写入的 `model_providers.codex` 会被闲置但不影响功能。
