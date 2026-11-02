# Spec: 中转站多账号（多 Key）

## Objective

现在一个中转站只能保存一个 API Key。同一个中转站有多个账号时，只能把它建成两条中转站记录：地址、协议、模型目录全部重复，模型测试结果也各存一份；换个账号就要重新选一遍模型。

本次把「中转站」和「账号」拆成两层：

- **中转站**：一个端点。保存 Base URL、协议集合、模型目录、备注。
- **账号**：端点下的一个 Key。保存名称、Key、备注。一个中转站可以有多个账号。
- **绑定**：引用「中转站 + 账号 + 模型 + 参数」。账号可以显式指定，也可以跟随中转站的默认账号。

谁在用：同一个中转站开了多个账号（个人/团队、不同额度、不同模型权限），需要在账号之间切换或让不同 Agent 用不同账号的人。

成功时：一个中转站下能保存多个账号；绑定可以选择用哪个账号；切换默认账号后，跟随默认的绑定下次写入即使用新账号；协议探测、模型拉取、连通性测试都能指定账号。

## 设计

### 数据模型（配置版本 5）

```yaml
version: 5
suppliers:
  - id: "s1"
    name: "openrouter"
    base_url: "https://openrouter.ai/api/v1"
    protocols: [chat_completions, messages]
    remark: "主力中转"
    default_key_id: "k1"
    keys:
      - id: "k1"
        name: "个人"
        api_key: "sk-or-..."
        remark: ""
      - id: "k2"
        name: "团队"
        api_key: "sk-or-..."
    models:
      - model: "anthropic/claude-sonnet-4.5"
        protocols: [chat_completions, messages]
        latency_ms: 820
        last_ok: true
        tested_key_id: "k1"
agents:
  codex:
    wsl:
      active: "b1"
      bindings:
        - id: "b1"
          supplier_id: "s1"
          key_id: null          # null = 跟随中转站默认账号
          protocol: responses
          model: "openai/gpt-5"
```

规则：

- `SupplierKey { id, name, api_key, remark }`，`id` 创建时生成，之后不变。
- 每个中转站至少一个账号；账号名称在同一中转站内唯一（去空白后比较）。
- `default_key_id` 指向本中转站内的一个账号；为空或指向不存在的账号时，回退到第一个账号。
- `AgentConfig.key_id: Option<String>`：`Some` 表示显式使用该账号；`None` 表示跟随默认。
- 账号解析顺序：`binding.key_id`（存在且有效）→ `supplier.default_key_id`（有效）→ `supplier.keys[0]`。
- 模型测试结果记录 `tested_key_id`，表示这次结果是用哪个账号测出来的。
- 协议集合、模型目录属于端点，不随账号变化。

### 删除与校验

| 场景 | 行为 |
|---|---|
| 删除被某条绑定显式引用的账号 | 拒绝，提示先改掉相关绑定 |
| 删除当前默认账号 | 允许；默认账号回退到剩余账号中的第一个 |
| 删除最后一个账号 | 拒绝，中转站至少要有一个账号 |
| 保存绑定时 `key_id` 指向不存在的账号 | 拒绝，提示账号已删除 |
| 保存中转站时账号名重复或 Key 为空 | 拒绝 |

### 界面

**中转站卡片**

```
openrouter                                    CHAT  MESSAGES
主力中转 · 2 个账号
https://openrouter.ai/api/v1
3 个模型                                      820 ms
```

**中转站编辑对话框**

在名称/地址/备注下面新增「账号」区块：

```
账号                                        [+ 添加账号]
┌──────────────────────────────────────────────────────┐
│ ● 个人      sk-or-...a1b2   [设为默认] [编辑] [删除]  │
│   团队      sk-or-...c3d4   [设为默认] [编辑] [删除]  │
└──────────────────────────────────────────────────────┘
```

- 每行显示名称、Key 掩码（前 4 后 4）、默认标记、操作。
- 「添加账号」展开一行内联表单：名称 + Key + 备注。
- 编辑 Key 时留空表示保留原 Key（与现在编辑中转站的语义一致）。
- 删除账号前弹确认；被引用时显示拒绝原因。

**中转站详情对话框**

- 顶部工具条新增「测试账号」下拉，默认选中默认账号；「测试全部模型」和单模型测试都用这个账号。
- 模型表格的状态列显示「成功 · 410 ms · 个人」，`tested_key_id` 对应的账号名。
- 「探测协议」「拉取模型」固定用默认账号（端点级操作）。

**绑定对话框**

- 「选择中转站」旁边新增「账号」下拉：
  - `跟随默认（个人）`
  - `个人`
  - `团队`
- 选中转站时账号下拉自动刷新为该中转站的账号列表，默认选中「跟随默认」。
- 绑定行摘要里，显式指定账号时追加 `· 个人`；跟随默认时不显示账号名。

**设置 / 托盘 / 同步**

- 同步载荷升到版本 5，账号随中转站一起同步。
- 托盘菜单仍然只列使用中的绑定，模型名后不加账号名（保持简洁）。

### 迁移

- 版本 4 → 5：每个中转站的 `api_key` 变成 `keys: [{ id, name: "默认", api_key }]`，`default_key_id` 指向它；所有绑定的 `key_id` 置为 `None`（跟随默认）。模型测试结果补 `tested_key_id: None`。
- 版本 3 / 2 / 1：先走现有迁移链得到版本 4 结构，再套用上面的 4 → 5 规则。
- 旧配置文件只读。

### 探测、拉取、测试的账号来源

| 操作 | 使用的账号 |
|---|---|
| 探测协议 | 默认账号 |
| 拉取模型 | 默认账号 |
| 单模型测试（中转站页） | 详情对话框里选中的账号 |
| 批量测试（中转站页） | 详情对话框里选中的账号 |
| 绑定对话框里的「测试」 | 该绑定解析后的账号 |
| 写入目标文件 | 该绑定解析后的账号 |

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
  supplier.rs          → SupplierKey、账号 CRUD 校验、effective_key、删除保护
  agent.rs             → AgentConfig.key_id、写入时解析账号
  config.rs            → 版本 5、v4 → v5 迁移
  migration.rs         → 旧版导入生成单账号中转站
  protocol.rs          → 探测接收显式 api_key
  connectivity.rs      → 测试接收显式 api_key
  sync/mod.rs、merge.rs → 载荷版本 5、读取 v4 及更早

cxc-v2/cxc-v2-desktop/src/
  components/suppliers/SupplierDialog.tsx      → 账号列表编辑
  components/suppliers/SupplierPage.tsx        → 账号数、测试账号选择、结果带账号名
  components/agents/BindingDialog.tsx          → 账号下拉
  components/agents/BindingRow.tsx             → 摘要显示显式账号
  lib/types.ts、api.ts、mock.ts、i18n.ts        → 类型、命令、演示数据、文案
  src-tauri/src/lib.rs                         → 命令签名带 key_id
```

## Code Style

沿用现有风格：核心用 `Result<T, Error>` + `thiserror`，中文错误；前端函数组件 + hooks，Tailwind 原子类，文案走 `i18n.ts`。

```rust
/// 解析绑定实际使用的账号：显式指定 → 中转站默认 → 第一个账号。
pub fn effective_key<'a>(supplier: &'a Supplier, key_id: Option<&str>) -> Option<&'a SupplierKey> {
    key_id
        .and_then(|id| supplier.keys.iter().find(|key| key.id == id))
        .or_else(|| {
            supplier
                .default_key_id
                .as_deref()
                .and_then(|id| supplier.keys.iter().find(|key| key.id == id))
        })
        .or_else(|| supplier.keys.first())
}
```

## Testing Strategy

核心单元测试：

- 账号 CRUD：新增、重名拒绝、空 Key 拒绝、编辑留空保留原 Key。
- 删除：显式引用的账号拒绝删除；默认账号删除后回退到第一个；最后一个账号拒绝删除。
- 账号解析：显式 → 默认 → 第一个；`key_id` 指向已删除账号时回退到默认。
- 写入：绑定显式指定账号时，目标文件写入该账号的 Key；跟随默认时写入默认账号；切换默认后重写会换 Key。
- 探测 / 拉取 / 测试：使用传入的 Key，不再读 `supplier.api_key`。
- 迁移：v4 → v5 单 Key 变一个「默认」账号、绑定 `key_id` 为空；v3 / v2 / v1 链路仍可到达 v5；幂等。
- 同步：v5 往返；读取 v4 载荷时迁移；合并保留账号。

前端（浏览器 mock）：

- 中转站编辑对话框添加、编辑、删除账号；删除被引用账号时看到拒绝提示。
- 详情对话框切换测试账号，结果列显示账号名。
- 绑定对话框账号下拉包含「跟随默认」与各账号；保存后绑定行显示显式账号。
- 切换默认账号后，跟随默认的绑定重新写入时使用新 Key（mock 记录写入的 Key）。
- 明暗主题、960×640 与 800×500。

## Boundaries

- Always: Key 在界面只显示掩码；错误提示不含完整 Key；写目标文件前生成 `.bak`；写入成功后才更新配置。
- Ask first: 引入自动轮换或额度统计；把账号提升为独立顶层实体；改同步载荷字段名。
- Never: 在日志或 toast 里输出完整 Key；删除最后一个账号；删除被显式引用的账号；修改旧版配置文件。

## Success Criteria

1. 一个中转站下可以保存多个账号（名称 + Key + 备注），至少一个。
2. 绑定可以选择具体账号，或选择「跟随默认」。
3. 切换中转站默认账号后，跟随默认的绑定在下次写入时使用新账号的 Key。
4. 绑定显式指定账号时，写入目标文件的是该账号的 Key。
5. 协议探测、模型拉取使用默认账号；单模型 / 批量测试可以指定账号，结果记录该账号名。
6. 删除被绑定显式引用的账号被拒绝；删除最后一个账号被拒绝。
7. 版本 4 配置自动升级为版本 5，原 Key 成为「默认」账号，所有绑定跟随默认。
8. 中转站卡片显示账号数量；同步、托盘、明暗主题、中英文保持可用。

## 状态

已实现。实现位置：`cxc-v2/cxc-v2-core/src/supplier.rs`（`SupplierKey`、`effective_key`、账号 CRUD 与删除保护）、`agent.rs`（`key_id` 与写入时解析）、`config.rs`（版本 5 与 v4 迁移）、`sync/`（载荷版本 5）；界面见 `cxc-v2/cxc-v2-desktop/src/components/suppliers/` 与 `components/agents/`。

## Open Questions

1. **账号级测试结果**：现在模型测试结果只记「最后一次用哪个账号测的」。是否要为每个 (模型, 账号) 各存一份结果？默认不做，避免表格和存储膨胀。
2. **账号级模型目录**：不同账号可能看到不同模型列表。默认仍按端点共享一份模型目录，用测试结果判断可用性。
3. **账号级协议**：默认协议属于端点，不按账号区分。
4. **批量操作**：是否需要「把某账号应用到所有绑定」的一键动作？默认不做，先靠切换默认账号。
5. **账号排序**：是否需要拖拽排序？默认按创建顺序，第一个为初始默认。
