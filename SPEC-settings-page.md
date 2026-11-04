# Capability Map: 设置页修正

| Module id | Responsibility | Depends on |
|---|---|---|
| settings-scrollbar | 设置弹窗内容区滚动条贴齐弹窗右边缘 | — |
| update-check | 检查更新：版本解析、Release 元数据、更新弹窗、按平台下载入口 | — |

Build order: settings-scrollbar → update-check

两个模块互不依赖，可分别验收；`settings-scrollbar` 是纯布局修正，先做先验证。

- `SPEC-settings-scrollbar.md` — 滚动条位置
- `SPEC-update-check.md` — 检查更新重做
