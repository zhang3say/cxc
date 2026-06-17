---
name: CXC Desktop
colors:
  background: '#f6f5f4'
  foreground: '#000000e0'
  card: '#ffffff'
  card-foreground: '#000000e0'
  popover: '#ffffff'
  popover-foreground: '#000000e0'
  primary: '#0075de'
  primary-foreground: '#ffffff'
  secondary: '#213183'
  secondary-foreground: '#ffffff'
  muted: '#f6f5f4'
  muted-foreground: '#615d59'
  border: '#e6e6e6'
  input: '#ffffff'
  ring: '#0075de'
  sticker-green: '#1aae39'
  sticker-orange: '#dd5b00'
  sticker-sky: '#62aef0'
  sticker-purple: '#d6b6f6'
  sticker-pink: '#ff64c8'
  sticker-teal: '#2a9d99'
typography:
  display-lg:
    fontFamily: 'Inter Variable', -apple-system, BlinkMacSystemFont, sans-serif
    fontSize: 24px
    fontWeight: '700'
    lineHeight: 32px
    letterSpacing: -0.02em
  body-base:
    fontFamily: 'Inter Variable', -apple-system, BlinkMacSystemFont, sans-serif
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
    letterSpacing: '0'
  body-bold:
    fontFamily: 'Inter Variable', -apple-system, BlinkMacSystemFont, sans-serif
    fontSize: 14px
    fontWeight: '600'
    lineHeight: 20px
    letterSpacing: '0'
  label-caps:
    fontFamily: 'Inter Variable', -apple-system, BlinkMacSystemFont, sans-serif
    fontSize: 11px
    fontWeight: '700'
    lineHeight: 14px
    letterSpacing: 0.05em
rounded:
  sm: 4px
  md: 8px
  lg: 12px
  xl: 16px
  full: 9999px
spacing:
  unit: 4px
  xs: 4px
  sm: 8px
  md: 16px
  lg: 24px
  xl: 32px
---

# Design System: CXC Desktop

CXC Desktop 是一款中转代理节点管理器。本设计系统基于 Notion-like 温暖、沉静且高效的纸张质感，通过极简的布局排版与自信的品牌蓝色点缀，为网络代理工具赋予优雅易用的生产力调性。

## 1. Visual Theme & Atmosphere

CXC 的视觉风格如同在明亮日光下摆放整齐的书桌。背景不是 clinical（临床般冷冰冰）的纯白，而是采用温暖的柔纸画布（`#f6f5f4`），使界面具有极佳的可读性，长时间阅读也不易产生视觉疲劳。

打字排版基于高可读性的 `Inter Variable` 字体。主色调保持墨黑（`#000000e0`），在需要操作的焦点或核心动作按键上，会亮起极为自信且克制的品牌蓝（Notion Blue, `#0075de`）。卡片和弹窗的边缘运用精致的细线描边（`1px Hairline, #e6e6e6`）而非重度投影，营造出干净而轻盈的卡片层级。

为了给冷静克制的 Chrome 外壳注入一丝活力，系统引入了一套**多彩贴纸色板**（Sticker Palette），这些颜色不参与骨架的渲染，仅用来装饰状态点、标签与延迟数值，在呼吸之间为界面带来轻松明快的性格色彩。

**核心特征：**
- 温暖舒适的纸张质感画布，不采用刺眼纯白。
- 只有一种核心操作色 Notion Blue（`#0075de`），确保界面重点分明。
- 使用 1px 细线描边配合轻量化投影定义容器与卡片。
- 多彩贴纸色板赋予节点状态、延迟等语义化标签高度识别性。

---

## 2. Color Palette & Roles

### Primary Foundation (主要基石)
- **Warm Canvas** (`#f6f5f4`): 页面基础画布背景，温暖纸质感。
- **Surface Card** (`#ffffff`): 卡片、面板、弹窗等容器背景色。
- **Ink Text** (`#000000e0`): 主文字与正文色，近乎全黑的灰度，保留印刷墨迹的高级感。
- **Muted Stone** (`#615d59`): 次要与说明文字，温和的暖灰色。
- **Hairline Border** (`#e6e6e6`): 1px 的边框、分割线描边。

### Accent & Interactive (交互与强调)
- **Notion Blue** (`#0075de`): 激活状态、主行动按键、链接色。
- **Deep Indigo** (`#213183`): 深邃靛蓝，通常用于高对比度顶部导航、特定强调段落或页头。

### Sticker Palette (状态贴纸)
- **Sticker Green** (`#1aae39`): 代表极佳延迟（如 < 100ms）或成功状态。
- **Sticker Orange** (`#dd5b00`): 代表中等延迟、一般警告。
- **Sticker Sky** (`#62aef0`): 信息标签、普通状态。
- **Sticker Purple** (`#d6b6f6`): 特定类别、专属标签。
- **Sticker Pink** (`#ff64c8`): 亮点提醒或高对比标记。
- **Sticker Teal** (`#2a9d99`): 备用健康节点、辅助状态。

---

## 3. Typography Rules

### Hierarchy & Weights
- **Display Heading** (`display-lg`): 用于大标题、总览数额。32px，粗体，略微收紧的字符间距（`-0.02em`），富有视觉冲击力。
- **Body Text** (`body-base`): 核心正文字体，14px，字重 400，行高 20px，保证小字在各种分辨率下都清晰锐利。
- **Body Bold** (`body-bold`): 强调或标签文字，14px，字重 600。
- **Eyebrow / Tiny Label** (`label-caps`): 最小的附属标签、全大写修饰词，11px，粗体，字间距略微加宽（`0.05em`）。

### Spacing Principles
- 所有的文字排版基于 4px 基础网格对齐，标题与正文段落之间严格保留 8px (`sm`) 的间隙。

---

## 4. Component Stylings

### Buttons
- **Primary Action**: 采用充满能量的 Notion Blue 填充，文字为纯白，圆角为 8px (`rounded-md`) 或 12px (`rounded-lg`)。
- **Secondary / Ghost Action**: 白色背景配以 1px 细线 border，悬浮时拥有微缩放与背景灰度改变。
- **Destructive Confirm**: 拥有从红到玫瑰色 (`from-red-500 to-rose-600`) 的渐变填充，配以高质感发光阴影，警示力度明确。

### Cards & Node Containers
- **Node Row**: 12px 圆角卡片容器，拥有 1px 极细描边，背景在悬浮时从纯白平滑过渡到微弱高亮，且不具备大面积投影，保持排版轻盈。
- **Card Active Border**: 被激活启用的节点，卡片左边缘会显示代表激活状态的蓝色徽章，同时伴有轻量发光轮廓。

### Inputs & Forms
- 采用纯白背景与 Hairline 边框，聚焦时边框变更为 `Notion Blue` 并生成 `ring` 发光环。输入框具有 8px 圆角与充足的内边距。

---

## 5. Layout Principles

### Grid & Structure
- 桌面端最大内容容器宽度限制在 960px 或 1200px 左右，以保证桌面端应用的视线聚焦度。
- 支持 List 列表模式与 Card 卡片网格模式的平滑切换。

### Whitespace Strategy
- 组件内部使用 16px (`md`) 的标准内边距。
- 节点卡片之间具有 12px 的间距，从而避免信息过度密集，使状态浏览具有极佳的呼吸感。

---

## 6. Design System Notes for Stitch Generation

Stitch 重新生成与迭代本系统相关页面时的引导指南：

### Language to Use
> **Vibe**: Warm paper-calm desktop utility, minimalist chrome, confident blue accents, precise typography, playful status stickers.

### Color References
- **Notion Blue**: `#0075de`
- **Warm Background**: `#f6f5f4`
- **Hairline Border**: `#e6e6e6`
- **Ink Dark**: `#000000e0`

### Component Prompts
- **New Provider Row**: *Create a minimalist provider card row with white background, 1px `#e6e6e6` border, a green status sticker `#1aae39` for latency, an active toggle switch, and clean Inter text.*
- **Settings Modal**: *Create a clean modal container with `#ffffff` background, `rounded-2xl`, smooth entry fade-in scale animation, containing input forms with 1px border and a blue active ring.*
