# ADR-0016: 推理档位记在中转站上

一张中转站卡片记一个当前推理档位。启用这张卡片时，选了档位就写进目标工具的当前档位；选「不设置」就删掉对应的键。档位列表写在程序里，不从模型接口读取。`none` 视为不设置。

三个工具的字段不一样，所以不共用一套写入位置：

- Codex 的普通会话和计划模式分开记。`reasoning_effort` 只写 `model_reasoning_effort`，`plan_reasoning_effort` 只写 `plan_mode_reasoning_effort`。没填的那一档在启用时删掉对应键。子代理的 `default_subagent_reasoning_effort` 不动。
- Claude 只写 `env.CLAUDE_CODE_EFFORT_LEVEL`。环境变量是 Claude Code 的全局覆盖，之后换任何模型都继承这一档。写入具体档位时删掉顶层 `effortLevel`，并去掉 `modelSettings` 里各模型的 `effortLevel`，避免旧的按模型档位盖住全局值；`maxEffortLevel` 和其他键保留。选不设置时删除顶层 `effortLevel` 和 `CLAUDE_CODE_EFFORT_LEVEL`，不改 `modelSettings`。
- Grok 只写当前模型块的 `reasoning_effort`。选不设置时只删当前模型块的这个键。不改 `[models].default_reasoning_effort`，其他模型块不动，也不写 `reasoning_efforts` 菜单。清单里没有 `none`。

考虑过按模型各记一档。CXC 的配置单位是中转站。同一张卡片上换模型，档位保持这张卡片的值。Codex 的计划模式是这张卡片上的另一档，不跟普通会话绑在一起。从某个档位改回「不设置」，下次启用这张卡片时会删掉目标文件里对应的键。

Gist 同步沿用 `grok_web_search` 的规则：本地空着、远端有值时采用远端；两边都有值时保留本地。不从工具配置把现有档位读回来。
