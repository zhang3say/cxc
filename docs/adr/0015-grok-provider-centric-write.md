# ADR-0015: Grok 配置以中转站为主体写入

## 状态

已接受 (2026-10-15)

## 背景

Grok CLI（`~/.grok/config.toml`）支持两种组织推理端点的方式：

1. 模型维度：每个 `[model.<id>]` 自带 `base_url` / `api_key` / `api_backend`（ADR-0013 采用的方式）。
2. 中转站维度：端点级设置写在 `[model_providers.<id>]`，模型用 `model_provider = "<id>"` 引用它。

ADR-0013 的写入方式在同一个中转站下配置多个模型时，会把同一份 `base_url` 与 `api_key` 重复写进每个模型块；本机 Grok 1.0.52 实测确认 `[model_providers]` 引用可用（请求确实命中 provider 块的端点与密钥），且模型自身字段的优先级高于 provider 块。

另外实测确认：`[models].web_search` 是独立于 `[models].default` 的一份 pin——未设置时搜索请求打到官方端点 `api.x.ai`（凭据取 `XAI_API_KEY` 或登录会话），指向中转站模型时才经由该模型的 `base_url`；指向 catalog 之外的模型时会静默禁用 web_search 工具。

## 决策

### 1. 写入以中转站为主体

CXC 切换 Grok 中转站时：

- `[model_providers."<中转站名称>"]` 承载 `base_url`、`api_key`、`api_backend`（由 `Provider.wire_api` 映射）。
- `[model."<id>"]` 只保留 `model` 与 `model_provider` 引用。
- `[models].default` 指向该中转站的默认模型。

键直接使用 CXC 里的「中转站名称」（带引号的 TOML 键，允许点、空格与中文）。重命名中转站会在配置文件里留下孤儿块，属于已知取舍。

### 2. 模型块端点字段的清理规则

受 CXC 管理的模型块（默认模型、附加模型、web_search 指定的模型）里的 `base_url` / `api_key` / `api_backend` 会被清掉。模型自身字段的优先级高于 provider 块，留着旧值会让本次切换不生效。其余字段（`context_window`、`reasoning_effort`、`extra_headers` 等）原样保留。

### 3. 一个中转站可挂多个模型

`Provider` 新增 `grok_models: Option<Vec<String>>`（默认模型之外的附加模型）。写入时为列表内每个模型生成引用块；从列表移除模型不会删除配置文件里的对应块（避免误删用户配置）。

### 4. web_search 单独处置

`Provider` 新增 `grok_web_search: Option<GrokWebSearch>`，四态：

| 取值 | 行为 |
|------|------|
| `None` / `keep` | 不写也不删 `[models].web_search` |
| `follow` | 写成该中转站的默认模型 |
| `model` | 写成指定模型（写入时会保证该模型也有引用块） |
| `unset` | 删除 `[models].web_search`，搜索回到官方端点 |

新建中转站的界面默认值为 `follow`：中转站场景下官方端点通常不可用（登录会话未授权时会 401）。被指定为中转搜索的模型需要支持 Responses 的 `web_search` 工具，否则搜索会报错。

### 5. 一次性迁移

`GrokAdapter::reorganize_provider_centric` 扫描 `[model.*]`，按 `(base_url, api_key, api_backend)` 分组并收拢进中转站块：

- 端点匹配到某个 CXC 中转站 → 用该中转站名称作键（必要时把已有同端点块改名过去）；
- 否则复用已存在的同端点 `[model_providers.*]` 块；
- 再否则用 `base_url` 的主机名新建（重名加后缀）。

迁移会同时把每个中转站下引用的模型回填到 CXC 的 `grok_models`，并把 `[models].web_search` 映射到对应中转站的 `grok_web_search`（对不上则保持不管理）。写前备份 `config.toml.bak`，返回报告供界面展示。

### 6. 读取兼容

`read()` 先取模型块自带字段，缺失时跟随 `model_provider` 引用读取 provider 块；引用缺失时返回空值而不报错（Grok 对悬空引用的处理是静默回落官方端点）。ADR-0013 时代的 model 维度配置仍可正确读取。

## 影响

- Grok 配置不再是每个模型一份端点副本，切换中转站只改一处。
- ADR-0013 的「写入字段」表被本 ADR 取代；`wire_api → api_backend` 映射、跨环境路径解析、连通性测试策略仍然有效。
- `Provider` 结构与同步载荷新增 `grok_models`、`grok_web_search` 两个可选字段，同步合并沿用 `claude_models` 的规则（本地缺失、远端有则采纳）。