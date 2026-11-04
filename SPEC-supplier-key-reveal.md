# Spec: 中转站编辑时 Key 可切换明文查看

## Objective

中转站编辑对话框里，账号 Key 输入框固定 `type="password"`；编辑已有账号时输入框为空，只显示掩码占位符，用户无法核对自己保存的 Key 是否完整（例如确认前缀、后缀、是否贴错账号）。

本次在 Key 输入框右侧加一个小眼睛按钮：默认隐藏，点击显示该账号的完整 Key，再点隐藏。查看已保存账号时，点击眼睛会把已保存的 Key 填入输入框；隐藏时若未改动，恢复为空，保持「留空保留原 Key」的保存语义。

谁在用：维护多个中转站账号、需要核对 Key 的人。

成功时：编辑中转站 → 点 Key 右侧小眼睛 → 看到完整 Key → 再点隐藏 → 保存后原 Key 不变。

## 现状与假设

ASSUMPTIONS:

1. 前端已拿到完整 Key 明文（`SupplierKey.api_key` 由 Rust 原样序列化，mock 同样）；不需要新增后端命令。
2. 范围只到中转站编辑对话框（`SupplierDialog.tsx`）的账号 Key；卡片、详情、绑定对话框、设置里的 Token 本次不改。
3. 每个账号的眼睛状态独立；关闭对话框后重置为隐藏。
4. 不新增依赖：`lucide-react` 已有 `Eye` / `EyeOff`。
5. 中英文文案都补齐。

## 交互设计

Key 输入框（`type=password`）右侧内嵌小眼睛按钮（`icon-xs`，absolute 定位，Input 加 `pr-8`）。

| 状态 | 点击后 |
|---|---|
| 隐藏 | 切换为 `type=text`；若输入框为空且该账号已保存 Key，把已保存 Key 填入输入框 |
| 显示 | 切换为 `type=password`；若当前值等于已保存 Key（即只是查看，没有改动），清空输入框，恢复掩码占位符 |
| 新建账号、隐藏 | 只切换 `type=text`，输入框内容不变 |
| 显示中修改了内容 | 隐藏时保留修改，以密码形式显示 |

- 眼睛按钮 `type="button"`，`aria-label` / `title` 为「显示 Key」/「隐藏 Key」。
- 图标：隐藏时 `Eye`，显示时 `EyeOff`。
- 已保存 Key 的掩码占位符继续用 `maskKey`，文案不变。

## 数据结构

`KeyDraft` 增加：

- `savedKey: string`：已保存的完整 Key，仅用于查看；新建账号为空。
- `revealed: boolean`：当前是否显示明文。

打开对话框时从 `supplier.keys[].api_key` 填入 `savedKey`，`revealed` 初始为 `false`。

## Tech Stack

不变：Tauri 2、React 19、TypeScript、Vite 7、Tailwind 4、Radix UI、lucide-react。不新增依赖，不改 Rust。

## Commands

```bash
cd cxc-desktop
npx tsc --noEmit
npm run build
npm run dev          # 浏览器预览（mock IPC）
```

## Project Structure

```text
cxc-desktop/src/
  components/suppliers/SupplierDialog.tsx  → KeyDraft 字段、眼睛按钮、显示/隐藏逻辑
  lib/i18n.ts                              → showKey / hideKey 中英文案
```

## Code Style

沿用现有风格：函数组件 + hooks，Tailwind 原子类，文案走 `i18n.ts`。

```tsx
<div className="relative flex-1">
  <Input
    type={key.revealed ? "text" : "password"}
    value={key.apiKey}
    className="... pr-8"
  />
  <Button
    type="button"
    size="icon-xs"
    variant="ghost"
    className="absolute right-1 top-1/2 -translate-y-1/2"
    aria-label={key.revealed ? t.hideKey : t.showKey}
    onClick={() => toggleReveal(index)}
  >
    {key.revealed ? <EyeOff className="size-3" /> : <Eye className="size-3" />}
  </Button>
</div>
```

## Testing Strategy

前端没有单元测试框架，沿用现有验证方式：

- `npx tsc --noEmit`、`npm run build` 通过。
- 浏览器 mock（`npm run dev`）手动验证：
  - 编辑已有中转站，点眼睛看到完整 Key，再点隐藏恢复掩码占位符。
  - 显示已保存 Key 后直接保存，Key 不变。
  - 显示后修改 Key 再保存，保存的是新 Key。
  - 新建账号输入 Key，眼睛可切换明文/密文。
  - 多账号时眼睛状态互不影响。
  - 中英文文案、明暗主题、960×640 与 800×500。

## Boundaries

- Always: 默认隐藏；只在用户点击眼睛后显示明文；显示状态不持久化（关闭对话框即重置）；错误提示不含完整 Key。
- Ask first: 在卡片/详情/绑定对话框也加眼睛；加「复制 Key」按钮。
- Never: 在日志或 toast 输出完整 Key；把完整 Key 写进 `localStorage`；默认显示明文。

## Success Criteria

1. 中转站编辑对话框里，每个账号的 Key 输入框右侧有小眼睛按钮。
2. 点击眼睛能在明文和密文之间切换；已保存账号点击后能看到完整 Key。
3. 查看后不修改直接保存，原 Key 保持不变。
4. 显示状态下修改后保存，保存的是修改后的 Key。
5. 新建账号、多账号、中英文、明暗主题下行为一致。
6. `npx tsc --noEmit` 与 `npm run build` 通过。

## Open Questions

1. 已保存 Key 的查看方式：推荐「填入输入框」，这样能选中复制；备选是只替换 placeholder（聚焦即消失，不能复制）。
2. 是否同时给设置里的 Gist Token、加密口令加眼睛？默认不做，保持本次范围。
3. 是否加复制按钮？默认不做。

## 状态

已实现。改动：`cxc-desktop/src/components/suppliers/SupplierDialog.tsx`（`KeyDraft` 新增 `savedKey` / `revealed`，Key 输入框内嵌小眼睛按钮，`toggleReveal` 处理显示/隐藏与「未改动则清空」）、`cxc-desktop/src/lib/i18n.ts`（`showKey` / `hideKey` 中英文案）。

验证：`npx tsc --noEmit`、`npm run build` 通过；浏览器 mock 下已验证编辑已有账号显示/隐藏、查看后不改保存 Key 不变、显示后修改保存新 Key、新建账号切换、多账号状态独立、中英文文案、明暗主题、800×500 视口。
