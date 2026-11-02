# ADR-0019: 三协议探测

## 背景

中转站可能支持 Anthropic Messages、OpenAI Chat Completions、OpenAI Responses 中的一种或多种。Codex 依赖 Responses，Claude 依赖 Messages，Grok 和 OpenCode 常用 Chat Completions。只区分「OpenAI / Anthropic」会把「能聊天但跑不了 Codex」的中转站误判为可用。

## 决策

中转站保存一个协议集合，取值为 `chat_completions`、`responses`、`messages`。

探测对三种协议分别发一次最小请求：

- `chat_completions`：`{base}/chat/completions`
- `responses`：`{base}/responses`
- `messages`：地址已以 `/v1/messages` 结尾则原样使用，以 `/v1` 结尾则加 `/messages`，否则加 `/v1/messages`

判定规则：

- 2xx 且正文是 JSON 对象：确认支持。
- 400 / 404 / 415 / 422 且正文是 JSON 对象：端点存在，确认支持。
- 401 / 403：鉴权失败，不加入集合。
- 429、网络错误、超时、非 JSON：本次未确认，不加入集合。

探测只并入新确认的协议，不删除用户手选的协议。报告逐项给出状态和原因，不包含密钥。

绑定保存具体协议。创建绑定时同时校验 Agent 能力和中转站协议集合；Claude 只允许 `messages`，Codex 只允许 `responses` 或 `chat_completions`。

## 后果

- 同一个中转站可以同时服务多个 Agent，各自选择兼容协议。
- 探测会产生少量真实请求；用户可以选择只探测缺失协议。
- 400/404 启发式可能误判，因此报告保留原因，手选协议始终可以覆盖。

## 状态

已采纳。实现见 `cxc-core/src/protocol.rs`。
