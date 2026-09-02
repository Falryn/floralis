# Floralis v0.1.1 发布物料

> 用途：GitHub Release 说明与发布执行记录。公开说明正文见下方「一」，与 `.qoder/release-notes-v0.1.1.md`（实际上传的 notes 文件）一致。
> 本版为 v0.1.0 之后的首个更新版本，主题为「批量导入能不能自动认对游戏」。

---

## 一、GitHub Release 说明

### Tag / 标题

`v0.1.1` —— 认得出你的名字

### 正文（中英双语，与上传的 notes 一致）

# v0.1.1 —— 认得出你的名字

这一版把「导入游戏后能不能自动认对」当成主线：批量导入现在能跨 Bangumi / Steam / VNDB 三个公开库交叉检索，自动带出正确的名称、封面与简介；同时也把几处会让人丢数据、丢时间的静默失败补上了提示。

**新增**

- 📥 Epic 游戏库导入：读取本机 Epic 清单，连已安装游戏一并扫入库
- 🪄 批量导入智能匹配：一个游戏可拿多个候选名（引擎工程标题 / exe 产品名 / 随包说明里的原文名）逐条检索三源，高置信直接采纳，不确定时列出候选让你点选；日文 / 中文名不再因为语种索引差异而搜不到
- 🗂️ 库目录监视：往游戏根目录里新加一个游戏文件夹，应用内会横幅提示一键入库
- ⭐ 收藏与高级筛选：仅看收藏、按评分筛选、批量操作归纳
- 💾 存档备份 / 恢复：每个游戏独立备份，恢复前自动再做一次安全备份

**修复**

- 元数据匹配不再被「恰好同名的另一个作品」牵着改名——线索词与跨源桥接只能提名候选，改名由你确认
- 启动游戏失败不再静默闪退：数据目录 / 数据库初始化失败会弹窗告知；exe 缺失或启动失败不再留下永不闭合的游玩时长记录
- Mod 禁用时若 `.off` 目标文件已存在，不再被静默覆盖
- 网络错误统一转成中文提示（超时 / DNS / 代理 / 连接失败），检查更新支持走代理

**性能**

- 封面按需加载、网格分块渲染：大库启动不再一次性生成全部缩略图

**下载与安装**

- 仅支持 Windows 10/11，下载 `Floralis_0.1.1_x64-setup.exe` 运行即可；从 v0.1.0 覆盖安装即保留原有数据
- 压缩包导入需本机安装 [7-Zip](https://www.7-zip.org/)
- 软件尚未购买代码签名证书，出现未知发布者警告属正常现象

---

v0.1.1 is about one thing: getting your games recognized correctly.

- **Epic library import** — scan installed games straight from your local Epic manifests
- **Smarter batch matching** — each game now tries several name clues (engine project title, exe product name, original title found in bundled readmes) against Bangumi / Steam / VNDB, auto-accepting only confident hits and listing the rest for you to pick; Japanese and Chinese titles are no longer lost to per-language search indexes
- **Library watching** — drop a new game folder into a watched root and the app offers to import it
- **Favorites & advanced filtering**, **per-game save backup / restore**
- A match can no longer rename your game to an unrelated work that merely shares a word; network failures now surface as readable messages instead of silent "no results"
- Covers load on demand and the grid renders in chunks, so large libraries start faster

Windows 10/11 only. Free & open-source under GPL-3.0. Overwriting the v0.1.0 install keeps your data.

Note: The software is not code-signed yet — an "Unknown publisher" warning is expected and safe to proceed.

---

## 二、版本号需同步的位置

| 位置 | 用途 |
|------|------|
| `package.json` / `package-lock.json` | npm 版本（`npm version 0.1.1 --no-git-tag-version` 一次更新两个文件） |
| `src-tauri/tauri.conf.json` | 安装包版本 + **更新检查比对的当前版本**（`app.config().version`） |
| `src-tauri/Cargo.toml` / `Cargo.lock` | 二进制版本（改 toml 后任意 cargo 命令自动刷新 lock） |
| `src/components/AboutDialog.vue` | 「关于」页显示的 `APP_VERSION` 字面量（未接配置，需手动改） |
| `website/package.json` / `website/src/constants.ts` | 官网展示版本与 `FALLBACK_VERSION` 兜底值 |
| `.github/ISSUE_TEMPLATE/bug_report.yml` | 反馈模板里的版本占位符 |
| `design-assets-spec.md` | 设计资产规格里记录的当前版本 |

> 注意：`AboutDialog.vue` 的 `APP_VERSION` 与其余位置之间没有自动化校验，漏改会让「关于」页显示旧版本。

---

## 三、发布执行清单

- [x] 版本号升至 0.1.1（上表 7 处全部同步）
- [x] 质量门禁：`cargo test` 78 passed / `cargo clippy -D warnings` 通过 / `check:contract` 通过 / `npm test` 57 passed / `vue-tsc` 通过
- [x] 真实库端到端复验（28 款，`D:\Game\SLG`）：高置信直采 11 / 待确认 10 / 未命中 7，命中项封面 21/21、简介 19/21
- [x] `npm run tauri build` 产出 `Floralis_0.1.1_x64-setup.exe`
- [ ] 提交版本号变更并打 tag `v0.1.1`
- [ ] 推送 `master` 与 tag 到 `Falryn/floralis`（本次含 14 个未推送提交）
- [ ] 创建 Release `v0.1.1`，上传 setup.exe，粘贴正文
- [ ] 验证：已安装的 v0.1.0 启动后能收到 0.1.1 更新提示
