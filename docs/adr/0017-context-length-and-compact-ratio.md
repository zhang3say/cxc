# ADR-0017: 上下文长度和压缩比例记在中转站上

一张中转站卡片可以记上下文长度和压缩比例。两个字段都可空，旧配置不用补。启用这张卡片时才写入目标工具；选不设置就删掉 CXC 负责的键。通用的 `write()` 不删这些键。

上下文长度用 Claude 那套写法，大小写不敏感。`200k` 是 200000，`1m` 是 1000000，不是 1048576。纯正整数也可以。系数必须是正整数，`1.5k` 不算。存进 CXC 时能被 1000000 整除就写成 `1m`，否则能被 1000 整除就写成 `200k`，其余写成十进制。空着、空白或 `none` 表示不设置。

压缩比例是 70–100 的整数百分比。69、101 和空着都是不设置。

三个工具的字段不一样：

- Claude 表单只提供不设置和 `1m`。保存 Claude 卡片时，其它长度按不设置处理，旧数据或同步数据也走同一条规则。选 `1m` 时，在 `ANTHROPIC_MODEL`、各档默认模型和 `CLAUDE_CODE_SUBAGENT_MODEL` 末尾加上一个 `[1m]`。写入前先去掉重复的 `[1m]`，大小写不敏感。上下文不设置时，模型名原样保留，手写的 `[1m]` 不动。不改顶层 `settings.model`。压缩写成 `CLAUDE_CODE_AUTO_COMPACT_WINDOW`，值是上下文 token 乘百分比再除以 100，并夹在 100000 到 1000000。`1m` 加 75% 得到 `750000`。上下文或比例缺一就不写这个环境变量，启用时删掉它。`CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` 不动。连通性测试发出去的模型名会去掉末尾 `[1m]`，因为带这个后缀的名字在中转上测不通。模型发现不加后缀。
- Codex 把解析后的 token 写到顶层 `model_context_window`，把同样算出的压缩 token 写到 `model_auto_compact_token_limit`。不改模型名，也不写 `model_post_turn_compact_threshold_percent` 和 `model_async_compact_threshold_percent`。不设置时删掉这两个 token 键。
- Grok 只给这次写入的模型块写 `context_window`，包括默认模型和 web_search 指定的模型。`1m` 写成 1000000。不写 `context_windows` 列表。其它模型块不动。不设置时只从这些受管理的块删掉 `context_window`。压缩比例原样写到 `[session].auto_compact_threshold_percent`，不换算 token；上下文空着也写。不设置时只删这个键，不删整个 `[session]`。按中转站整理配置时仍保留已有的 `context_window`。

Claude 不再单独配置 Fable。`ANTHROPIC_DEFAULT_FABLE_MODEL` 始终跟主模型，并套用同一套 `[1m]` 规则。Opus、Sonnet、Haiku、Subagent 留空或只有空白时，把主模型写进对应环境变量。旧配置里的 `fable` 字段读入时忽略。

Gist 同步沿用推理档位的规则：本地空着、远端有值时采用远端；两边都有值时保留本地。
