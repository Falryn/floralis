# 花譜 Floralis — 图标与图片资产设计清单

## 项目简介

**花譜（かふ）Floralis** 是一款基于 Tauri + Vue 3 构建的 Windows 桌面游戏管理工具。

- **花**（はな）：花，象征每一个游戏如同花园中的一朵花
- **譜**（ふ）：乐谱、图谱、编年史，意为"将花编排成谱"——即把散落的游戏整理、归档、记录
- **Floralis**：源自拉丁语 *flora*（花神/植物群）+ 后缀 *-is*（属于…的），意为"属于花的"

整体寓意：**将玩家的游戏库视为一座花园，逐一记录、编排、照料**——管理游戏生命周期、追踪游玩时间、整理 Mod、归档存档。

应用当前版本：v0.1.0，支持中/英/日三语，多主题（浅色×3 + 深色×3），自定义无边框窗口。

---

## 一、应用图标（静态图片文件）

| 文件路径 | 用途 | 当前描述 | 格式/尺寸 |
|---------|------|---------|----------|
| `public/app-icon.png` | 侧边栏 Logo（界面内展示） | 粉紫渐变圆角矩形背景 + 五瓣樱花 + 可爱表情 + 散落花瓣/爱心装饰 | PNG, 512×512 |
| `src-tauri/icons/32x32.png` | Windows 任务栏小图标 | 同上 | PNG, 32×32 |
| `src-tauri/icons/128x128.png` | 标准桌面图标 | 同上 | PNG, 128×128 |
| `src-tauri/icons/128x128@2x.png` | 高 DPI 图标 | 同上 | PNG, 256×256 |
| `src-tauri/icons/256x256.png` | 大尺寸图标 | 同上 | PNG, 256×256 |
| `src-tauri/icons/512x512.png` | 商店/安装包展示 | 同上 | PNG, 512×512 |
| `src-tauri/icons/1024x1024.png` | 最大尺寸源图 | 同上 | PNG, 1024×1024 |
| `src-tauri/icons/icon.ico` | Windows exe 图标 + 系统托盘 | 同上 | ICO（内含 256×256） |

> 设计提示：应用图标需在 16×16 极小尺寸下仍可辨识，建议主体元素居中、避免过细线条。

---

## 二、UI 图标 — Emoji 替换清单

以下 emoji 当前直接写在代码中，需全部替换为统一设计的 SVG 图标。

### 2.1 导航 / 视图切换

| 当前 | 语义 | 出现位置 | 建议文件名 |
|------|------|---------|-----------|
| 📚 | 游戏库 | 侧栏视图切换 + "全部游戏" | `nav-games.svg` |
| 🧩 | Mod 管理 | 侧栏视图切换 + "全部Mod" | `nav-mods.svg` |
| 📁 | 分组 | 侧栏游戏分组列表项 | `nav-folder.svg` |
| ⚙️ | 设置 | 侧栏底部按钮 | `nav-settings.svg` |

### 2.2 游戏状态

| 当前 | 语义 | 出现位置 | 建议文件名 |
|------|------|---------|-----------|
| 🆕 | 未游玩 | 侧栏筛选 / 详情状态 / 统计面板 | `status-not-played.svg` |
| ▶️ | 游玩中 | 同上 | `status-playing.svg` |
| ✅ | 已通关 | 同上 | `status-completed.svg` |
| 📌 | 搁置 | 同上 | `status-shelved.svg` |

### 2.3 Mod 相关

| 当前 | 语义 | 出现位置 | 建议文件名 |
|------|------|---------|-----------|
| 🧩 | Mod（无封面占位） | Mod 详情页 / 侧栏 | `placeholder-mod.svg` |
| 🎮 | 游戏（Mod关联分组） | 侧栏 Mod 视图 / 游戏无封面占位 | `placeholder-game.svg` |
| 📦 | 独立 Mod（未关联游戏） | 侧栏 Mod 视图 | `icon-independent.svg` |
| 🖼️ | 封面占位 | Mod 编辑对话框 | `placeholder-cover.svg` |

### 2.4 右键菜单 / 操作

| 当前 | 语义 | 出现位置 | 建议文件名 |
|------|------|---------|-----------|
| ✏️ | 编辑 | 游戏/Mod 右键菜单、分组操作、设置标签 | `action-edit.svg` |
| 🗑️ | 删除 | 游戏/Mod 右键菜单、分组删除、编辑对话框 | `action-delete.svg` |
| 📂 | 打开所在目录 | 游戏/Mod 右键菜单 | `action-open-folder.svg` |
| 🔗 | 关联游戏 | Mod 右键菜单 | `action-link.svg` |
| 🔴 | 禁用（当前为启用态） | Mod 右键菜单 | `status-disabled.svg` |
| 🟢 | 启用（当前为禁用态） | Mod 右键菜单 | `status-enabled.svg` |
| 💾 | 数据库备份 | 设置-数据管理 | `action-backup.svg` |

### 2.5 功能按钮

| 当前 | 语义 | 出现位置 | 建议文件名 |
|------|------|---------|-----------|
| ✨ | 导入游戏 / 新版本通知 | 底部操作栏 + 更新横幅 | `action-import.svg` |
| 📊 | 统计 | 底部操作栏 + 侧栏状态标题 | `icon-stats.svg` |
| 🏷️ | 标签（全部标签） | 侧栏标签筛选标题 | `icon-tags.svg` |

### 2.6 设置 — 主题选择

| 当前 | 语义 | 建议文件名 |
|------|------|-----------|
| 💜 | 薰衣草（浅色） | `theme-lavender.svg` |
| 🌸 | 樱花（浅色） | `theme-sakura.svg` |
| 🍃 | 薄荷（浅色） | `theme-mint.svg` |
| 🌙 | 暗夜（深色） | `theme-night.svg` |
| 🌊 | 海洋（深色） | `theme-ocean.svg` |
| 🍷 | 深红（深色） | `theme-crimson.svg` |

### 2.7 设置 — 关闭行为

| 当前 | 语义 | 建议文件名 |
|------|------|-----------|
| ❓ | 询问 | `behavior-ask.svg` |
| 🚪 | 直接退出 | `behavior-exit.svg` |
| 📌 | 最小化到托盘 | `behavior-minimize.svg` |

### 2.8 文本符号（建议统一为 SVG）

| 当前 | 语义 | 出现位置 | 建议文件名 |
|------|------|---------|-----------|
| ☰ | 列表视图 | 视图切换按钮 | `view-list.svg` |
| ⊞ | 网格视图 | 视图切换按钮 | `view-grid.svg` |
| ▾ | 下拉指示 | 批量操作按钮 | `chevron-down-sm.svg` |
| ✕ | 关闭 | 面板/对话框关闭按钮 | `action-close.svg` |
| ✓ | 确认/全选 | 多选组件 / 设置 | `action-check.svg` |
| ▶ | 启动 / 展开 | 游戏启动按钮 / 折叠面板 | `action-play.svg` / `chevron-right.svg` |
| + | 新增 | 侧栏新建分组 | `action-add.svg` |

---

## 三、内联 SVG 图标（需统一重绘）

当前以 `<svg>` 直接写在 Vue 模板中，需提取为独立 SVG 文件并统一风格。

| 语义 | 出现位置 | 当前 viewBox | 建议文件名 |
|------|---------|-------------|-----------|
| 搜索 | 游戏/Mod 搜索框 | 24×24 | `icon-search.svg` |
| 星标（实心） | 游戏详情评分 | 24×24 | `icon-star-filled.svg` |
| 星标（空心） | 游戏详情评分（未选中） | 24×24 | `icon-star.svg` |
| 可执行文件 | 游戏详情-路径区 | 24×24 | `icon-exe.svg` |
| 启动参数 | 游戏详情-路径区 | 24×24 | `icon-args.svg` |
| 脚本文件 | 游戏详情-路径区 | 24×24 | `icon-script.svg` |
| 存档路径 | 游戏详情-路径区 | 24×24 | `icon-save-path.svg` |
| 备注 | 游戏详情-路径区 | 24×24 | `icon-notes.svg` |
| 文件夹 | Mod 详情-路径区 | 24×24 | `icon-folder.svg` |
| 链接/URL | Mod 编辑-来源链接 | 24×24 | `icon-link.svg` |
| 对勾（启用） | Mod 列表开关 | 24×24 | `icon-check.svg` |
| 叉号（禁用） | Mod 列表开关 | 24×24 | `icon-cross.svg` |
| 折叠箭头 | Mod 分组标题 | 24×24 | `icon-chevron-right.svg` |
| 下拉箭头 | CustomSelect / MultiSelect | 20×20 | `icon-chevron-down.svg` |
| 播放三角 | 游戏列表悬浮-启动 | 24×24 | `icon-play.svg` |
| 编辑铅笔 | 游戏列表悬浮-编辑 | 24×24 | `icon-edit.svg` |
| 窗口最小化 | 自定义标题栏 | 10×1 | `win-minimize.svg` |
| 窗口最大化 | 自定义标题栏 | 10×10 | `win-maximize.svg` |
| 窗口还原 | 自定义标题栏 | 10×10 | `win-restore.svg` |
| 窗口关闭 | 自定义标题栏 | 10×10 | `win-close.svg` |
| 最小化到托盘 | 关闭下拉菜单 | 14×14 | `action-tray.svg` |
| 退出应用 | 关闭下拉菜单 | 14×14 | `action-exit.svg` |

---

## 四、动态用户图片（无需设计，仅说明）

| 类型 | 来源 | 存储位置 | 说明 |
|------|------|---------|------|
| 游戏封面 | IGDB/VNDB API 或用户自定义 | `$APPDATA/covers/` | 3:4 竖版 |
| Mod 封面 | 用户上传 | `$APPDATA/mod_covers/` | 横版 |
| 自定义横幅 | 用户设置 | `$APPDATA/custom_images/` | 全宽横幅 |
| 侧边栏背景 | 用户设置 | 同上 | 竖版，支持模糊/亮度调节 |
| 空态插图 | 用户设置（可选） | 同上 | — |

---

## 五、设计规范

### 5.1 风格

- **整体调性**：柔和圆润、日系清新，与"花譜"（花 × 乐谱）主题呼应
- **线条**：统一 **1.5px 描边**（stroke-width），圆角端点（stroke-linecap: round, stroke-linejoin: round）
- **填充**：功能图标单色 `currentColor`；状态/主题图标允许双色
- **视觉重量**：所有图标在 24×24 视口内保持相近的视觉面积与留白
- **禁止**：不使用 emoji 替代、不使用多色渐变（应用图标除外）、不使用阴影/发光效果

### 5.2 格式

| 类型 | 格式 | 尺寸 | 技术要求 |
|------|------|------|---------|
| UI 功能图标 | SVG | viewBox="0 0 24 24" | `fill="none" stroke="currentColor" stroke-width="1.5"` |
| 窗口控制图标 | SVG | viewBox="0 0 10 10" | 极简单色，stroke-width="1" |
| 托盘/菜单小图标 | SVG | viewBox="0 0 14 14" | 单色描边 |
| 应用图标 | PNG + ICO | 32/128/256/512/1024 + ico | 圆角矩形（rx≈18.75%），需适配 Windows 任务栏 16px |
| 占位图 | SVG | 自适应容器 | 单色低透明度，居中 |

### 5.3 命名规范

```
{类别前缀}-{语义}.svg
```

| 前缀 | 含义 | 示例 |
|------|------|------|
| `nav-` | 导航/视图 | `nav-games.svg` |
| `action-` | 用户操作 | `action-delete.svg` |
| `status-` | 状态指示 | `status-playing.svg` |
| `icon-` | 通用信息图标 | `icon-search.svg` |
| `theme-` | 主题预览 | `theme-sakura.svg` |
| `behavior-` | 行为选项 | `behavior-exit.svg` |
| `win-` | 窗口控制 | `win-close.svg` |
| `view-` | 视图模式 | `view-grid.svg` |
| `placeholder-` | 占位/空态 | `placeholder-game.svg` |
| `chevron-` | 方向箭头 | `chevron-down.svg` |

- 全部小写，单词间用 `-` 连接
- 不使用下划线、驼峰、空格

### 5.4 交付物汇总

| 类别 | 数量 | 格式 |
|------|------|------|
| 应用图标 | 1 套（8 个文件） | PNG × 7 + ICO × 1 |
| UI 功能图标 | ~45 个 | SVG |
| 占位图 | 3 个（游戏/Mod/封面） | SVG |
| **合计** | **~56 个文件** | — |

---

## 六、参考：当前应用图标描述

当前图标由代码生成（`scripts/gen-icon.cjs`），描述如下：

- 背景：粉→紫→蓝三色线性渐变圆角矩形（rx=96/512）
- 主体：五瓣花（粉色渐变花瓣，72° 均匀分布）
- 花心：暖黄径向渐变圆形 + 可爱表情（眼睛 + 微笑 + 腮红）
- 装饰：散落的半透明圆点（紫/粉）+ 两颗小爱心
- 阴影：花瓣组有柔和投影（flood-color: #c77dba, opacity 0.3）

设计师可参考此意境（花 + 可爱 + 柔和），但鼓励全新创作。
