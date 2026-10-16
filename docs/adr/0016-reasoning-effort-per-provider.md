# ADR-0016: 推理档位记在中转站上

一张中转站卡片记一个当前推理档位。启用这张卡片时，把这个档位写进目标工具的「当前档位」字段；卡片上没填，就不动目标文件里已有的值。

三个工具的字段不一样，所以不共用一套写入位置：

- Codex 把同一个值写入 `model_reasoning_effort` 和 `plan_mode_reasoning_effort`。子代理的 `default_subagent_reasoning_effort` 不动。
- Claude 只写用户 `settings.json` 里的 `modelSettings.<当前模型>.effortLevel`。不写 `maxEffortLevel`，也不写 `CLAUDE_CODE_EFFORT_LEVEL`。
- Grok 只写当前模型块的 `reasoning_effort`。不改 `[models].default_reasoning_effort`，其他模型块不动，也不写 `reasoning_efforts` 菜单。

考虑过按模型各记一档。Codex 的文件本身只有一份当前档位，Claude 和 Grok 虽然能按模型保存，但 CXC 的配置单位是中转站。同一张卡片上换模型，档位保持这张卡片的值。从某个档位改回「不改文件」只让 CXC 忘掉它，不把目标文件改回去。

Gist 同步沿用 `grok_web_search` 的规则：本地空着、远端有值时采用远端；两边都有值时保留本地。不从工具配置把现有档位读回来。
