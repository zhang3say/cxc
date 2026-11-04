# ADR-0025: 0.2.0 接管 CXC 名称与配置路径

## 背景

0.1.x 的 `cxc-desktop` 与并行的 `cxc-v2` 是两套应用：产品名分别是 `cxc-desktop` 与 `CXC 2`，identifier 分别是 `com.cxc.desktop` 与 `com.cxc.desktop.v2`，配置分别在 `~/.config/cxc/` 与 `~/.config/cxc-v2/`。

`cxc-v2` 的功能已经覆盖并超过旧版，继续维护两套没有收益。决定废弃旧版，让新版接管 `CXC` 这个名称。

## 决策

1. 删除 `cxc-core/` 与 `cxc-desktop/` 的旧实现，把 `cxc-v2/cxc-v2-core`、`cxc-v2/cxc-v2-desktop` 移到这两个目录，crate 与包名去掉 `v2`。
2. 产品名回到 `CXC`，identifier 用 `com.cxc.desktop`，版本 `0.2.0`。安装包因此可以覆盖安装旧版。
3. 配置路径改为 `~/.config/cxc/config.yaml`（Windows `%APPDATA%\cxc\config.yaml`）。
4. 首次启动按优先级接管数据，覆盖前先备份为 `config.yaml.backup.<时间戳>`：
   - 主路径已是新格式 → 直接用；
   - 存在上一代 `~/.config/cxc-v2/config.yaml` → 优先接管（它比旧版数据新）；
   - 主路径是旧版 v1 格式 → 备份后原地迁移；
   - 都没有 → 空配置。
5. 旧版格式靠「没有 `version` 字段，且存在 `codex_providers` / `claude_providers` / `grok_providers` / `providers` 之一」识别。
6. 删除设置里的「重新导入旧版配置」按钮与 `import_legacy_config` 命令：旧版已下线，接管是自动的，保留一个点了没反应的按钮更糟。
7. 发布标签继续使用 `cxc-desktop-v*`，避免与历史发布割裂。
8. 例外：1.0.0 用一次性桥接标签 `v1.0.0`。0.1.x 的检查更新只去掉标签开头的 `v`，`cxc-desktop-v1.0.0` 会被解析成 `[NaN,0,0]` 而判定为「已是最新」；`v1.0.0` 能被正确解析为 `1.0.0`，老用户开机即可收到升级提示。之后的版本回到 `cxc-desktop-v*`，新版解析器两种形式都支持。

## 后果

- 用户覆盖安装后，配置自动迁移，旧文件保留一份备份。
- 上一代 `cxc-v2` 配置优先于旧版 v1，避免接管到更旧的数据。
- 旧版 `cxc-desktop` 的源码只存在于 git 历史中，需要时可从历史恢复。
- 产品名与 identifier 与 0.1.x 相同，系统会把新版视为旧版的升级，而不是并存的两个应用。

## 状态

已采纳，随 0.2.0 发布。
