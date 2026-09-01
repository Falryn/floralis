# Floralis v0.1.0 发布物料（草稿）

> 用途：GitHub Release 说明 + 分发渠道简介。截图已完成（`docs/screenshots/`）。
> 图标已定稿（拼图手柄图标），安装包已使用最终图标重新构建。

---

## 一、GitHub Release 说明

### Tag / 标题

`v0.1.0` —— 花譜 Floralis 首个公开版本

### 中文说明

花譜 Floralis 是一款面向视觉小说与独立游戏玩家的本地游戏库管理工具。
整理你的游戏库、一键启动、记录游玩时光、管理 Mod——全部离线完成，数据只属于你。

**主要功能**

- 📚 游戏库管理：分组、标签、网格/列表双视图，支持批量导入（压缩包解压 / 目录扫描）
- 🪄 智能补全：从 Bangumi / Steam / VNDB / IGDB 一键匹配元数据与封面
- ▶️ 一键启动：自动检测主程序，支持脚本与参数
- ⏱️ 游玩时长统计：后台自动追踪进程，日历视图查看会话历史
- 🧩 Mod 管理：启用/禁用、配置档一键切换整套组合
- 🔐 密码加密存储（AES-256-GCM + 系统密钥链）
- 💾 每日自动备份数据库，支持手动导出/恢复
- 🎨 6 套主题、自定义背景图；中 / English / 日本語

**下载与安装**

- 仅支持 Windows 10/11，下载 `Floralis_0.1.0_x64-setup.exe` 运行即可
- 压缩包导入需本机安装 [7-Zip](https://www.7-zip.org/)
- 软件尚未购买代码签名证书，出现未知发布者警告属正常现象

**支持项目**

花譜是免费开源软件（GPL-3.0）。如果它帮到了你，欢迎支持一下：
- 国内：微信 / 支付宝扫码（见 README 收款码）
- 海外：[Ko-fi](https://ko-fi.com/falryn) / [PayPal](https://paypal.me/falryn)

### English

Floralis is a local game library manager for visual novel & indie game players.
Organize your library, launch with one click, track playtime and manage mods — fully offline, your data stays yours.

Highlights: batch import (archive extraction / directory scan), metadata & cover auto-fill from Bangumi / Steam / VNDB / IGDB, automatic playtime tracking with calendar history, mod profiles, AES-256-GCM encrypted passwords, daily auto-backup, 6 themes, zh / English / 日本語.

Windows 10/11 only. Free & open-source under GPL-3.0.

Note: The software is not code-signed yet — an "Unknown publisher" warning is expected and safe to proceed.

---

## 二、分发渠道简介（一句话 / 短文案）

**一句话（≤30字）**

> 视觉小说与独立游戏的本地库管家：整理、启动、计时、Mod 一站搞定。

**短文案（约 100 字）**

> 花譜 Floralis：面向视觉小说与独立游戏玩家的本地游戏库管理工具。批量导入压缩包、自动匹配封面与元数据、一键启动、游玩时长日历、Mod 配置档、密码加密、每日自动备份。完全离线，数据只属于你。免费开源（GPL-3.0），支持中/英/日三语。

---

## 三、截图清单（待图标定稿后截取）

| # | 画面 | 说明 |
|---|------|------|
| 1 | 游戏库网格视图（有封面数据） | 主视觉，首图 |
| 2 | 游戏详情面板（游玩日历） | 时长统计卖点 |
| 3 | 批量导入清单 | 导入流程卖点 |
| 4 | Mod 管理视图 | Mod 功能卖点 |
| 5 | 关于页（含打赏入口） | 打赏触达展示 |

建议：深色主题 + 樱花主题各截一组，分辨率 1280×800。

---

## 四、发布执行清单

- [x] 图标定稿（用户提供）→ 已裁剪去文字并替换 `src-tauri/icons/` 全套 + `public/app-icon.png`
- [x] 提供微信/支付宝收款码图片 → 已放入 `public/donate/wechat.jpg`、`public/donate/alipay.jpg`
- [x] 仓库已改为公开（GitHub API 已验证 `Falryn/floralis` 可访问）
- [x] 注册 Ko-fi（绑 PayPal）→ 已替换为 `ko-fi.com/falryn`（AboutDialog.vue / README.md / FUNDING.yml）
- [x] 推送代码到已有仓库 `Falryn/floralis`（历史已清除硬编码密钥后推送成功）
- [x] 重新执行 `npm run tauri build`（带最终图标与收款码）→ 产物 `Floralis_0.1.0_x64-setup.exe`（6.93 MB）
- [x] 创建 Release `v0.1.0`，上传 setup.exe，粘贴本文件的 Release 说明
- [x] 截取截图清单中的 5 张图 → 已保存至 `docs/screenshots/` 并接入 README
- [ ] 验证：安装版启动后更新检查无报错（默认 repo 已改为 Falryn/floralis）
