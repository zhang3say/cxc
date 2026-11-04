# Spec: 设置弹窗滚动条贴齐右边缘

## Objective

设置弹窗内容区可以滚动，但滚动条缩在弹窗内部：弹窗左右各有 24px 内边距，滚动容器又额外留了 4px 右内边距，滚动条距弹窗右边缘约 28px，看起来像悬在内容中间。

本次让滚动条贴住弹窗右边缘，内容仍保持原来的左右留白。

谁在用：打开设置页、需要滚动查看同步与关于区块的人。

成功时：滚动条轨道紧贴弹窗右边缘，文字与输入框的左边缘、右边缘位置不变。

## 现状与假设

ASSUMPTIONS:

1. 只改设置弹窗（`SettingsDialog.tsx`）；中转站编辑弹窗有同样的 `pr-1` 写法，本次不动。
2. 滚动条样式（`index.css` 里的 `::-webkit-scrollbar`，宽 10px、滑块 4px）保持不变。
3. 弹窗内边距保持 24px（`p-6`），标题、分隔线、底部按钮的位置不变。
4. 浏览器预览环境用 overlay 滚动条（宽度为 0），无法截图看到滚动条本体；以「滚动容器右边缘 = 弹窗右边缘」的几何断言为准，实机（Windows / Linux）再确认视觉效果。

## 设计

`SettingsDialog` 的 `DialogContent` 保持 `p-6`。滚动容器：

```tsx
// 之前
<div className="min-h-0 space-y-5 overflow-y-auto pr-1">

// 之后
<div className="min-h-0 space-y-5 overflow-y-auto -mr-6 pr-6">
```

- `-mr-6`（-24px）让滚动容器向右延伸到弹窗内边距之外，右边缘与弹窗右边缘重合，滚动条因此贴边。
- `pr-6`（24px）把内容重新收回来，文字与输入框的右边缘和标题、底部按钮对齐。
- 去掉 `pr-1`，避免再额外缩进 4px。
- 滚动容器位于标题与底部按钮之间，不会碰到弹窗圆角。

## Tech Stack

不变：React 19、TypeScript、Tailwind 4。无新增依赖。

## Commands

```bash
cd cxc-desktop
npx tsc --noEmit
npm run build
npm run dev          # 浏览器预览（mock IPC）
```

## Project Structure

```text
cxc-desktop/src/components/settings/SettingsDialog.tsx  → 滚动容器外边距与内边距
```

## Code Style

沿用现有 Tailwind 原子类写法，负外边距与正内边距成对出现，并加一行注释说明意图。

## Testing Strategy

- `npx tsc --noEmit`、`npm run build` 通过。
- 浏览器（960×640 与 800×500）：
  - 打开设置，滚动容器右边缘与弹窗右边缘的 `getBoundingClientRect().right` 相等。
  - 输入框右边缘与标题文字右边缘对齐，没有被滚动条压住。
  - 明暗主题、中英文正常。
- Windows / Linux 实机：滚动条轨道贴住弹窗右边缘。

## Boundaries

- Always: 保持内容左右留白 24px；只改设置弹窗。
- Ask first: 同时修正中转站编辑弹窗与其它弹窗的同类问题；调整滚动条宽度或颜色。
- Never: 为了贴边把内容也顶到弹窗边缘；改动弹窗圆角或阴影。

## Success Criteria

1. 设置弹窗滚动容器的右边缘与弹窗右边缘重合（差值 0）。
2. 内容左右留白仍为 24px，输入框与标题对齐。
3. 960×640、800×500、明暗主题、中英文下布局正常。
4. `npx tsc --noEmit` 与 `npm run build` 通过。

## Open Questions

1. 中转站编辑弹窗、绑定弹窗是否也要一起贴边？默认本次只做设置弹窗。
2. 滚动条是否需要更显眼（加宽或加深）？默认保持现状。

## 状态

已实现。改动：`cxc-desktop/src/components/settings/SettingsDialog.tsx` 的滚动容器改为 `-mr-6 pr-6`，去掉 `pr-1`。

验证：浏览器 960×640 与 800×500 下，滚动容器 `right` 与弹窗 `right` 均为 736 / 相等（差值 0），输入框右边缘仍比弹窗右边缘内缩 24px；明暗主题、中英文正常。Windows / Linux 实机滚动条外观待发布后确认。
