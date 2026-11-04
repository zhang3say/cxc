# Spec: 检查更新重做

## Objective

设置页的「检查更新」现在点了没有效果：无论 GitHub 上有没有新版本，都只会弹一条 4 秒的 toast，内容还是当前版本号。

根因：发布标签是 `cxc-desktop-v1.0.1`，后端用 `tag_name.trim_start_matches('v')` 去前缀，`cxc-desktop-v` 没被去掉；`version_gt` 把 `cxc-desktop-v1` 解析成 0，于是 `[0,0,1]` 永远小于当前版本 `[1,0,1]`，`has_update` 恒为 false。

本次重做「检查更新」：

- 正确解析 `cxc-desktop-vX.Y.Z` 形式的标签并比较版本。
- 在设置页里给出明确状态：检查中 / 已是最新 / 发现新版本 / 检查失败（可重试）。
- 发现新版本时打开更新弹窗，显示版本跨度、发布时间、更新说明，以及按当前系统挑选的安装包下载入口。
- 不再自动打开浏览器。

谁在用：想知道自己是不是最新版、想升级到新版本的人。

成功时：1.0.1 的安装包点「检查更新」显示「已是最新版本」；GitHub 上出现 1.0.2 后，同一按钮显示「发现新版本 1.0.2」，点开能看到更新说明并下载对应安装包。

## 现状与假设

ASSUMPTIONS:

1. 更新来源固定为 `https://api.github.com/repos/zhang3say/cxc/releases/latest`，仓库地址写死在代码里，与现有实现一致。
2. 只做「检查 + 跳转下载」，不做应用内下载与静默安装（那需要 `tauri-plugin-updater`、签名密钥与 CI 改造）。
3. 下载用系统浏览器打开安装包直链；没有匹配当前平台的资产时回退到 Release 页面。
4. 预发布版本（`-beta` 等）按去掉后缀后的版本号比较，不单独提示。
5. 中英文文案都补齐。

## 设计

### 后端（`cxc-desktop/src-tauri/src/lib.rs`）

`UpdateInfo` 扩展为：

```rust
#[derive(Serialize)]
struct UpdateAsset {
    name: String,
    url: String,
    size: u64,
}

#[derive(Serialize)]
struct UpdateInfo {
    current_version: String,
    latest_version: String,
    has_update: bool,
    release_name: String,
    release_url: String,
    release_notes: String,
    published_at: String,
    /// 当前平台最合适的安装包；没有匹配时为 None。
    download_url: Option<String>,
    assets: Vec<UpdateAsset>,
}
```

版本解析：

```rust
/// 从 GitHub 标签里取出版本号：`cxc-desktop-v1.0.1` / `v1.0.1` / `1.0.1` → `1.0.1`。
/// 取第一个数字开始、连续的数字与点；遇到其它字符停止，所以 `v1.0.1-beta.1` → `1.0.1`。
fn parse_tag_version(tag: &str) -> String
```

- 找不到数字时返回空串，视为「没有可用版本」，`has_update = false`，不报错。
- `version_gt` 保留，输入改为解析后的版本号；缺失段按 0 补齐。

平台匹配（纯函数，便于测试）：

```rust
/// os: "windows" | "macos" | "linux"；arch: "x86_64" | "aarch64"。
fn pick_asset_for(assets: &[UpdateAsset], os: &str, arch: &str) -> Option<String>
```

| 平台 | 优先 | 回退 |
|---|---|---|
| Windows | 名称以 `-setup.exe` 结尾 | 名称以 `.msi` 结尾 |
| macOS arm64 | 名称以 `aarch64.dmg` 结尾 | 任意 `.dmg` |
| macOS x64 | 名称以 `x64.dmg` 结尾 | 任意 `.dmg` |
| Linux | 无（不发布 Linux 安装包） | Release 页面 |

匹配时忽略大小写。`pick_asset` 用 `std::env::consts::OS` / `ARCH` 调用它。

请求：

- `Client::builder()` 加 `user_agent(format!("CXC/{}", env!("CARGO_PKG_VERSION")))` 与 `timeout(Duration::from_secs(10))`。
- 请求头 `Accept: application/vnd.github+json`。
- 状态码映射成可读错误：
  - 403 / 429 → 「GitHub API 访问受限（未登录每小时 60 次），请稍后再试」
  - 404 → 「还没有可用的发布版本」
  - 其它非 2xx → `HTTP {status}`
- 解析字段：`tag_name`、`name`、`html_url`、`body`、`published_at`、`assets[].name / browser_download_url / size`。

把网络与解析逻辑抽成 `async fn fetch_update_info(current: &str) -> Result<UpdateInfo, String>`，命令 `check_update` 只负责取 `app.package_info().version` 并调用它。这样可以用一个 `#[ignore]` 的网络测试手动验证真实接口。

### 前端

**类型与命令**（`lib/types.ts`、`lib/api.ts`）：`UpdateInfo` 增加新字段，新增 `UpdateAsset`；命令名 `check_update` 不变。

**设置页「关于」区块**（`SettingsDialog.tsx`）：

```
关于
版本 1.0.1                                  [检查更新]
已是最新版本
```

- 按钮三态：`检查更新` / `检查中…`（转圈、禁用）/ 检查完成后恢复。
- 结果行（`role="status"`）：
  - 已是最新：灰色 `已是最新版本 · 1.0.1`
  - 有新版本：主题色 `发现新版本 1.0.2` + 「查看更新」按钮，点开更新弹窗
  - 失败：红色错误文案 + 「重试」按钮
- 结果行在关闭设置弹窗后清空，下次打开重新开始。

**更新弹窗**（新增 `components/settings/UpdateDialog.tsx`，挂在 `App.tsx` 层）：

```
发现新版本 1.0.2
当前版本 1.0.1 · 发布于 2026-11-04 15:50

更新说明
┌────────────────────────────────────┐
│ （Release 正文，可滚动）            │
└────────────────────────────────────┘

[在 GitHub 查看]              [稍后] [下载安装包]
```

- 标题：`发现新版本 {latest_version}`；副标题：`当前版本 {current} · 发布于 {published_at 本地时间}`。
- 更新说明按行渲染成纯文本：`#` / `##` / `###` 开头当作标题，`-` 开头当作列表项，其余原样显示；`max-h-64` 可滚动。正文为空时显示「这次发布没有填写说明」。不解析 HTML。
- 主按钮：有 `download_url` 时「下载安装包」，否则「打开 Release 页面」；都用系统浏览器打开。
- 「在 GitHub 查看」用文字链接打开 `release_url`。
- 设置弹窗保持打开，更新弹窗叠在上层；关闭更新弹窗回到设置。

**演示数据**（`lib/mock.ts`）：`get_app_version` 返回 `0.1.0`，`check_update` 返回 `1.0.2` 的演示数据（含说明与两个资产），浏览器预览可完整走通「检查 → 查看 → 下载」。

**文案**（`lib/i18n.ts` 新增，中英各一份）：`checkingUpdate`、`upToDate`、`newVersion`、`releaseNotes`、`noReleaseNotes`、`downloadInstaller`、`openReleasePage`、`viewOnGithub`、`publishedAt`、`later`、`retry`、`updateFailed`。

## Tech Stack

不变：Tauri 2、React 19、TypeScript、Vite 7、Tailwind 4、Radix UI；`reqwest` 已在依赖中。不新增依赖。

## Commands

```bash
cargo test -p cxc-desktop
cargo clippy -p cxc-desktop --all-targets -- -D warnings
cargo fmt --all -- --check

cd cxc-desktop
npx tsc --noEmit
npm run build
npm run dev          # 浏览器预览（mock IPC）
```

## Project Structure

```text
cxc-desktop/src-tauri/src/lib.rs              → UpdateInfo、parse_tag_version、pick_asset_for、fetch_update_info
cxc-desktop/src/lib/types.ts                  → UpdateInfo / UpdateAsset
cxc-desktop/src/lib/mock.ts                   → 演示用的新版本数据
cxc-desktop/src/lib/i18n.ts                   → 更新相关中英文案
cxc-desktop/src/components/settings/SettingsDialog.tsx  → 关于区块的状态与结果行
cxc-desktop/src/components/settings/UpdateDialog.tsx    → 新增更新弹窗
cxc-desktop/src/App.tsx                       → 检查更新状态、更新弹窗挂载
```

## Code Style

核心用 `Result<T, String>` + 中文错误；纯函数单独抽出便于测试；前端函数组件 + hooks，Tailwind 原子类，文案走 `i18n.ts`。

```rust
/// 从 GitHub 标签里取出版本号：`cxc-desktop-v1.0.1` → `1.0.1`。
fn parse_tag_version(tag: &str) -> String {
    let start = tag.find(|c: char| c.is_ascii_digit());
    let Some(start) = start else {
        return String::new();
    };
    tag[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .trim_end_matches('.')
        .to_string()
}
```

## Testing Strategy

Rust 单元测试（`cxc-desktop/src-tauri/src/lib.rs` 内）：

- `parse_tag_version`：`cxc-desktop-v1.0.1`、`v1.0.1`、`1.0.1`、`v1.0.1-beta.1`、无数字的标签。
- `version_gt`：`1.0.10 > 1.0.9`、`1.1.0 > 1.0.9`、`1.0.1 == 1.0.1`、`1.0 > 1.0.0`。
- `pick_asset_for`：Windows 选 `-setup.exe`、Windows 只有 msi 时选 msi、macOS arm64 选 `aarch64.dmg`、macOS x64 选 `x64.dmg`、Linux 返回 None、名称大小写混排。
- `#[ignore]` 的网络测试 `live_check_update`：用 `current = "0.9.0"` 调 `fetch_update_info`，断言 `latest_version == "1.0.1"` 且 `has_update`；手动执行 `cargo test -p cxc-desktop live_check_update -- --ignored`。

前端（浏览器 mock）：

- 点「检查更新」看到「检查中…」→「发现新版本 1.0.2」。
- 点「查看更新」打开更新弹窗，显示版本跨度、发布时间、更新说明。
- 「下载安装包」打开资产直链；「在 GitHub 查看」打开 Release 页面。
- 关闭更新弹窗回到设置；关闭设置后重新打开，结果行清空。
- 失败态：在 mock 里临时让 `check_update` 抛错，确认红色文案与「重试」。
- 中英文、明暗主题、960×640 与 800×500。

## Boundaries

- Always: 检查更新只读，不改配置；错误提示可读且不含敏感信息；下载用系统浏览器打开。
- Ask first: 引入 `tauri-plugin-updater` 做应用内下载与静默安装；启动时自动检查；「跳过此版本」；修改发布工作流。
- Never: 在应用内直接执行下载的安装包；用 GitHub token 做更新检查；把 Release 正文当 HTML 渲染（只做纯文本）。

## Success Criteria

1. 1.0.1 安装包点「检查更新」显示「已是最新版本 · 1.0.1」。
2. GitHub 上存在更高版本时显示「发现新版本 x.y.z」，可打开更新弹窗。
3. 更新弹窗显示当前版本、最新版本、发布时间与更新说明。
4. 「下载安装包」打开与当前系统匹配的安装包；没有匹配资产时打开 Release 页面。
5. 网络失败或限流显示可读错误，并可重试。
6. 检查过程中按钮显示「检查中…」且不可重复点击。
7. 浏览器 mock 可完整演示；中英文、明暗主题、960×640 与 800×500 正常。
8. `cargo test -p cxc-desktop`、`npx tsc --noEmit`、`npm run build` 通过。

## Open Questions

1. 是否加启动时自动检查（静默，发现新版本在设置按钮上加小红点）？默认不做，先保留手动检查。
2. 是否加「跳过此版本」？默认不做。
3. 是否改为 `tauri-plugin-updater` 做应用内下载并安装？需要签名密钥、`latest.json` 与 CI 改造，默认不做。
4. 「下载安装包」直接给安装包直链（推荐），还是统一打开 Release 页面？默认直链，无匹配时回退页面。

## 状态

已实现。改动：`cxc-desktop/src-tauri/src/lib.rs`（`parse_tag_version`、`pick_asset_for`、`fetch_update_info`、`UpdateInfo` 扩展、错误映射与 8 个单元测试）、`lib/types.ts`（`UpdateInfo` / `UpdateAsset` / `UpdateStatus`）、`lib/mock.ts`（1.0.2 演示数据）、`lib/i18n.ts`（更新相关中英文案）、`components/settings/SettingsDialog.tsx`（关于区块状态与结果行）、`components/settings/UpdateDialog.tsx`（新增）、`App.tsx`（检查状态、更新弹窗挂载、关闭设置时清空结果）。

验证：`cargo test -p cxc-desktop` 7 个单元测试通过、1 个网络测试手动执行通过（真实接口返回 `1.0.1`）；`cargo test -p cxc-core` 76 个通过；`cargo clippy`、`cargo fmt --check`、`npx tsc --noEmit`、`npm run build` 通过。浏览器 mock 下已验证检查中 / 发现新版本 / 更新弹窗 / 下载链接 / 在 GitHub 查看 / 稍后返回 / 关闭设置后清空结果 / 错误态与重试 / 已是最新态 / 中英文 / 明暗主题 / 800×500。
