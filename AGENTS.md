# AGENTS.md

## 项目简介

花譜 Floralis 是一款面向视觉小说和独立游戏玩家的本地化游戏库管理工具。

技术栈：Tauri 2 + Vue 3 + Pinia + TypeScript + Tailwind CSS 4

核心能力：游戏/Mod 信息管理、状态追踪、一键启动、游戏时间统计、智能封面下载（IGDB/VNDB）、多主题切换、中/英/日多语言支持。

## 核心文件边界

以下路径为高敏感区域，修改时需格外谨慎：

| 路径 | 说明 |
|------|------|
| `src/stores/` | Pinia 状态管理（gameStore、modStore），全局数据流核心 |
| `src-tauri/src/` | Rust 后端（Tauri commands、数据库、外部 API 集成） |
| `src/types.ts` | 前后端共享类型定义 |
| `src/i18n/locales/` | 国际化语言包（zh-CN、en-US、ja-JP） |

## 关键约定

1. **Tauri command 前后端同步**：新增或修改 Tauri command 时，必须同时更新 Rust 端（`src-tauri/src/commands/`）和前端调用（`src/stores/` 或组件中的 `invoke()`），并在 `src-tauri/src/main.rs` 中注册。
2. **i18n 三语言同步**：任何用户可见文案变更必须同步更新 `zh-CN.json`、`en-US.json`、`ja-JP.json` 三个语言文件。
3. **类型一致性**：Rust models（`src-tauri/src/models.rs`）与前端类型（`src/types.ts`）字段需保持一致。
4. **仅支持 Windows**：项目仅面向 Windows 平台，无需考虑跨平台兼容。

## 构建与验证命令

```bash
# 前端类型检查 + 构建
npm run build

# 启动开发模式（前端 + Tauri）
npm run tauri dev

# 仅启动前端开发服务器
npm run dev

# 构建 Tauri 安装包
npm run tauri build

# Rust models 与前端 types.ts 契约一致性检查（修改共享数据结构后必须运行）
npm run check:contract
```
