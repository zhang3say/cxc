# Spec: Agent 绑定列表

## Objective

当前 Agent 页把「配置表单」和「当前配置展示」塞进同一张卡片：协议、中转站、模型、推理档位、计划模式档位、上下文长度、压缩比例、Claude 多模型映射、Grok web_search 全部内联，卡片高度超过一屏；同时没有「切换」的概念，改配置就是覆盖。

本次把 Agent 页改回「绑定列表 + 编辑对话框」：

- 每个 `(工具, 环境)` 可以保存多条绑定，同一时刻只有一条「使用中」。
- 列表只显示摘要，表单移入对话框，卡片不再又长又杂。
- 不再单独放「当前启用」区块——使用中的那条直接在列表里高亮。这是上一版「当前启用 + 绑定目录」重复的根因。
- 启用一条绑定 = 把它写进该环境的目标配置文件。

谁在用：同时维护多个中转站/模型组合，需要在「日常」「省钱」「备用」之间来回切换的人。

成功时：Agent 页一屏能看到该 Agent 的全部绑定和各自参数摘要；点「启用」立即切换并写入；点「编辑」在对话框里改参数，不挤占列表空间。

## 设计

### 页面结构

```
┌──────────────────────────────────────────────────────────────┐
│ [Codex] [Claude Code] [Grok CLI] [OpenCode]      ● 已安装     │  工具切换
├──────────────────────────────────────────────────────────────┤
│ [WSL] [Windows]              Grok CLI · WSL                  │  环境切换
├──────────────────────────────────────────────────────────────┤
│ 绑定 3                                          [+ 新建绑定]  │
├──────────────────────────────────────────────────────────────┤
│ ▎ grok-4.5                                          [使用中]  │  ← 使用中：蓝色竖条 + 浅蓝底
│   openrouter · Chat                                          │
│   推理 high   上下文 500k   压缩 75%   web_search follow      │
│                                                    [测试][编辑][删除] │
├──────────────────────────────────────────────────────────────┤
│   grok-4-fast                                                │
│   openrouter · Chat                                          │
│   推理 minimal   上下文 200k                                 │
│                                          [启用] [测试][编辑][删除] │
├──────────────────────────────────────────────────────────────┤
│   gpt-5                                                      │
│   codex-relay · Responses                                    │
│   推理 high   上下文 200k   压缩 80%                         │
│                                          [启用] [测试][编辑][删除] │
└──────────────────────────────────────────────────────────────┘
```

### 行内信息层级

| 层级 | 内容 | 样式 |
|---|---|---|
| 主 | 模型名 | `text-xs font-bold text-foreground`，等宽字体 |
| 次 | `中转站名 · 协议` | `text-[10.5px] text-muted-foreground` |
| 三 | 参数 chips：推理档位、计划模式、上下文、压缩比例、Claude 映射、web_search | `text-[9px]` 细边框，弱化 |
| 状态 | 使用中 / 未启用 | 「使用中」用 primary 徽章；未启用不显示徽章 |
| 动作 | 启用、测试、编辑、删除 | 图标按钮，hover 才明显；「启用」是文字按钮 |

使用中的行：左侧 2px 竖条 `bg-primary`、背景 `bg-primary/[0.04]`、边框 `border-primary/20`。整行不再重复显示「当前启用」标题。

### 编辑对话框

沿用现有 `SupplierDialog` 的视觉：`sm:max-w-lg`、圆角 2xl、`bg-card/95` + `backdrop-blur-xl`、头部标题 + 说明、可滚动主体、底部「取消 / 测试 / 保存」。

字段顺序：

1. 协议、中转站（两列）
2. 模型输入 + 候选项 chips
3. 推理档位、计划模式档位（Codex 才有第二列）
4. 上下文长度、压缩比例
5. Claude 多模型映射（仅 Claude）
6. Grok web_search 策略（仅 Grok）

按钮语义：

- 新建：`保存`（只入库，不启用）、`保存并启用`（入库 + 写入）。
- 编辑使用中的绑定：`保存` 后自动重写目标文件（因为它是生效中的那份）。
- 编辑未启用的绑定：`保存` 只入库。

### 空状态

- 该 `(工具, 环境)` 没有绑定：虚线框 + 「还没有绑定」+「新建绑定」按钮。
- 搜索无匹配：虚线框 + 「没有匹配的结果」。

## Tech Stack

不变：Tauri 2、React 19、TypeScript、Vite 7、Tailwind 4、Radix UI；核心 `cxc-v2-core`（Rust edition 2024）。不新增依赖。

## Commands

```bash
cd cxc-v2
cargo test -p cxc-v2-core
cargo clippy -p cxc-v2-core -p cxc-v2-desktop --all-targets -- -D warnings
cargo fmt --all -- --check

cd cxc-v2/cxc-v2-desktop
npx tsc --noEmit
npm run build
npm run dev          # 浏览器预览（mock IPC）
```

## Project Structure

```text
cxc-v2/cxc-v2-core/src/
  agent.rs              → AgentConfig 增加 id；AgentSlot { active, bindings }；绑定 CRUD 与启用
  config.rs             → 配置版本 4；v3 → v4 升级
  migration.rs          → 旧版导入落到 bindings[0] 并设为 active
  sync/mod.rs           → 载荷版本 4；读取 v3 / v2 / v1
  sync/merge.rs         → 按 (工具, 环境, 绑定 id) 合并

cxc-v2/cxc-v2-desktop/src/
  components/agents/AgentPage.tsx        → 工具 + 环境 + 绑定列表
  components/agents/BindingRow.tsx       → 单行绑定（新增）
  components/agents/BindingDialog.tsx    → 编辑对话框（新增）
  components/agents/AgentConfigCard.tsx  → 删除
  lib/types.ts、api.ts、mock.ts、i18n.ts → 类型、命令、演示数据、文案
  src-tauri/src/lib.rs                   → 新命令与托盘
```

## 数据模型

配置版本升到 4：

```yaml
version: 4
suppliers: [ ... ]
agents:
  codex:
    wsl:
      active: "b1"
      bindings:
        - id: "b1"
          supplier_id: "s1"
          protocol: responses
          model: "gpt-5"
          reasoning_effort: high
          plan_reasoning_effort: xhigh
          context_length: 200k
          compact_ratio: 80
        - id: "b2"
          supplier_id: "s2"
          protocol: chat_completions
          model: "gpt-5-mini"
    windows: null
  claude: {}
  grok: {}
  opencode: {}
```

规则：

- `AgentSlot { active: Option<String>, bindings: Vec<AgentConfig> }`，`None` 表示该环境还没配置。
- `AgentConfig` 重新带上 `id`（16 字节随机十六进制），创建后不变。
- `active` 必须指向本 slot 内的一条绑定；删除使用中的绑定会被拒绝，提示先启用别的。
- 同一 `(工具, 环境)` 内绑定 id 唯一；`(supplier_id, protocol, model)` 允许重复（不同参数可以各存一条）。
- 校验规则不变：供应商必须存在；协议必须同时被工具与供应商支持；Claude 只允许 `messages`；Codex 不允许 `messages`；Claude 上下文只接受 `1m`。

## 命令

| 命令 | 入参 | 行为 |
|---|---|---|
| `get_config` | — | 返回完整配置 |
| `get_envs` | — | 可达环境列表 |
| `detect_agents` | — | 按 `(工具, 环境)` 探测 |
| `add_binding` | `agent, env, input, activate: bool` | 校验 → 入库；`activate` 为真时同时写入目标文件并设为 active |
| `edit_binding` | `agent, env, id, input` | 校验 → 更新；若该绑定是 active，保存后自动重写目标文件 |
| `delete_binding` | `agent, env, id` | active 绑定拒绝删除 |
| `activate_binding` | `agent, env, id` | 写入目标文件成功后设置 active |
| `rewrite_agent` | `agent, env` | 按当前 active 绑定重新写入 |
| `save_settings` | `customDirs` | 自定义目录（按工具 × 环境） |
| 其余 | — | 中转站、探测、测试、同步、更新保持不变 |

写入失败时配置保持不变（先写文件，成功后再落盘配置）。

## 迁移

- 版本 3 → 4：每个 `(工具, 环境)` 的单份配置转成 `bindings: [config]`，`active` 指向它。
- 版本 2 → 4：`active_wsl` / `active_app` 指向的绑定优先，其余绑定按原顺序进入 `bindings`，`active` 指向激活项；WSL 侧没有激活项时取第一条。
- 版本 1（旧版应用）：沿用现有供应商合并逻辑，每个旧 provider 生成一条绑定，旧激活项设为 active。
- 同步载荷版本 4；读取版本 3 / 2 / 1 载荷时先收敛再合并。
- 旧配置文件只读。

## Code Style

沿用现有风格：核心用 `Result<T, Error>` + `thiserror`，错误信息中文；前端组件函数式 + hooks，样式用 Tailwind 原子类，文案走 `i18n.ts`。

```rust
pub fn activate_binding(
    cfg: &mut Config,
    kind: AgentKind,
    env: Env,
    id: &str,
    adapter: &dyn AgentAdapter,
) -> Result<(), AgentError> {
    let slot = cfg.agent(kind).get(env).ok_or(AgentError::NotConfigured)?;
    let binding = slot
        .bindings
        .iter()
        .find(|binding| binding.id == id)
        .ok_or_else(|| AgentError::BindingNotFound(id.to_string()))?;
    adapter
        .apply(supplier, binding, cfg.custom_dirs.get(kind, env), env)
        .map_err(|error| AgentError::Write(error.to_string()))?;
    cfg.agent_mut(kind).get_mut(env).unwrap().active = Some(id.to_string());
    Ok(())
}
```

## Testing Strategy

核心单元测试（`cargo test -p cxc-v2-core`）：

- 绑定 CRUD：新增、编辑、删除；删除 active 拒绝；active 指向不存在的 id 时视为无。
- 启用：写入失败时 active 不变、配置不变；写入成功时 active 更新。
- 编辑 active 绑定触发重写；编辑非 active 绑定不触发。
- 校验：供应商不存在、协议不受支持、Claude 仅 `messages`、模型为空。
- 迁移：v3 → v4 单份转一条绑定；v2 → v4 多条绑定 + active 映射；v1 导入；幂等。
- 同步：v4 往返；v3 / v2 / v1 载荷收敛；合并按绑定 id。

前端：`npx tsc --noEmit`、`npm run build`；浏览器 mock 走通新增、编辑、启用、删除、空状态、搜索过滤、明暗主题。

## Boundaries

- Always: 写入前生成 `.bak`；写入成功后才更新 active；错误提示不含密钥；中文文案。
- Ask first: 改配置版本号以外的数据结构；新增 npm / crate 依赖；改同步载荷字段名。
- Never: 修改旧版 `~/.config/cxc/config.yaml`；删除使用中的绑定；把密钥写进日志或 toast。

## Success Criteria

1. Agent 页有工具切换、环境切换、绑定列表、「新建绑定」；没有独立的「当前启用」区块。
2. 列表每行显示模型名（主）、`中转站 · 协议`（弱）、参数 chips；使用中的行有蓝色竖条与「使用中」徽章。
3. 新建绑定后出现在列表里；点「启用」立即写入目标文件并切换高亮。
4. 编辑使用中的绑定保存后自动重写目标文件；编辑未启用的绑定只入库。
5. 删除使用中的绑定被拒绝并提示；删除其他绑定正常。
6. 卡片高度不再随字段数量增长；960×640 窗口下一屏至少显示 3 条绑定。
7. 版本 3 配置自动升级为版本 4，原有配置成为唯一一条绑定并处于使用中。
8. 中转站、协议探测、模型测试、Gist 同步、托盘、明暗主题、中英文保持可用。

## Open Questions

1. 绑定是否需要「复制」动作（基于现有绑定另存一条）？默认不做。
2. 列表是否显示最近一次测试的延迟？默认显示（有测试结果时）。
3. 托盘菜单是否列出该 Agent 的全部绑定供切换？默认只列使用中的那条，点击重新写入。
