# Floralis

一款使用 Tauri 2 + Vue 3 构建的桌面游戏管理应用。

## 功能特性

- **游戏管理**：添加、编辑、删除游戏，支持分组和标签分类
- **游戏导入**：支持从压缩包自动解压并识别游戏信息
- **游戏启动**：一键启动游戏，支持脚本启动和参数传递
- **游戏时间统计**：自动记录游戏时间，支持日历视图查看
- **封面管理**：支持从 IGDB/VNDB 自动搜索下载封面，或手动设置
- **密码管理**：加密存储游戏密码，使用 AES-256-GCM 加密
- **主题切换**：支持多种主题（浅色/深色/樱花/海洋/绯红）
- **数据备份**：支持导出/导入所有数据
- **多语言**：支持中文、英文、日文

## 技术栈

### 前端
- **Vue 3** - 渐进式 JavaScript 框架
- **Pinia** - Vue 状态管理库
- **TypeScript** - 类型安全的 JavaScript 超集
- **Tailwind CSS** - 实用优先的 CSS 框架
- **Vite** - 下一代前端构建工具

### 后端 (Rust)
- **Tauri 2** - 跨平台桌面应用框架
- **SQLite** - 轻量级嵌入式数据库
- **rusqlite** - SQLite Rust 绑定
- **image** - 图像处理库（用于生成缩略图）
- **aes-gcm** - AES-256-GCM 加密
- **keyring** - 系统密钥链访问
- **ureq** - HTTP 客户端

## 项目结构

```
floralis/
├── src/                    # 前端源码
│   ├── components/         # Vue 组件
│   │   ├── App.vue         # 主应用组件
│   │   ├── GameGrid.vue    # 游戏网格/列表视图
│   │   ├── GameDetail.vue  # 游戏详情面板
│   │   ├── Sidebar.vue     # 侧边栏
│   │   ├── ImportDialog.vue    # 导入对话框
│   │   ├── EditGameDialog.vue  # 编辑游戏对话框
│   │   ├── SettingsDialog.vue  # 设置对话框
│   │   └── ...
│   ├── stores/             # Pinia 状态管理
│   │   └── gameStore.ts    # 游戏状态
│   ├── i18n/               # 国际化
│   │   └── locales/        # 语言文件
│   ├── types.ts            # TypeScript 类型定义
│   └── main.ts             # 应用入口
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # 主模块，Tauri 命令
│   │   ├── db.rs           # 数据库模块
│   │   ├── igdb.rs         # IGDB 游戏数据库集成
│   │   └── vndb.rs         # VNDB 视觉小说数据库集成
│   ├── Cargo.toml          # Rust 依赖配置
│   └── tauri.conf.json     # Tauri 配置
└── package.json            # Node.js 依赖配置
```

## 开发

### 环境要求

- Node.js 18+
- Rust 1.70+
- Windows 10/11（当前仅支持 Windows）

### 安装依赖

```bash
# 安装前端依赖
npm install

# 确保 Rust 工具链已安装
rustup update
```

### 开发模式

```bash
npm run tauri:dev
```

开发模式通过 `src-tauri/tauri.dev.json` 覆盖应用标识符（`com.echon.floralis.dev`），
数据目录为 `%APPDATA%\com.echon.floralis.dev\`，与安装版（`com.echon.floralis`，
数据在 `%APPDATA%\com.echon.floralis\`）完全隔离，两者可同时运行、互不干扰。
首次构建可能需要较长时间（5-10分钟），因为需要编译 Rust 依赖。

### 构建发布版本

```bash
npm run tauri build
```

构建产物位于 `src-tauri/target/release/` 目录。

## 数据存储

应用数据存储在以下位置：

- **数据库**：`%APPDATA%/floralis/floralis.db`
- **封面图片**：`%APPDATA%/floralis/covers/`
- **缩略图**：`%APPDATA%/floralis/thumbnails/`

## 配置

### IGDB 集成

要使用 IGDB 游戏搜索功能，需要在 [Twitch Developer Console](https://dev.twitch.tv/console) 创建应用并获取 Client ID 和 Client Secret。

### 7-Zip

解压压缩包需要安装 7-Zip，并在设置中指定 7z.exe 的路径。

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+F` | 聚焦搜索框 |
| `Ctrl+,` | 打开设置 |
| `Enter` | 启动选中的游戏 |
| `Space` | 启动选中的游戏 |
| `Delete` | 删除选中的游戏 |
| `Esc` | 关闭面板/取消选择 |

## 许可证

MIT License
