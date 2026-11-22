## Android v0.1.0 — 2026-11-22

首个安卓版（Beta）发布 —— 基于 Amethyst-Android（PojavLauncher 后继，LGPL-3.0）品牌化构建的完整 Minecraft Java 版启动器。

### 功能

- 完整启动 Minecraft Java 版（版本下载 / Forge / Fabric / Quilt 安装 / 微软与离线登录 / 触屏操作 / 内置 JRE）
- 应用名「大肥鱼」、黑白 BF 全套图标（mdpi ~ xxxhdpi，方形 + 圆形）
- 版本号 0.1.0（BigFish Android 首发版）
- 保留 Amethyst / PojavLauncher 上游版权声明（LGPL-3.0 要求）

### 构建

- 通过 GitHub Actions 云端构建（`android.yml`），产物 `BigFish-android-debug.apk`（160.7 MB，debug 签名）
- Release: https://github.com/bigfinlab/bigfish/releases/tag/android-v0.1.0

### 已知限制

- Beta 测试版（debug 签名），未在真机充分验证
- 独立 Amethyst 品牌化构建，不含桌面版 Tauri 界面

## 0.2.2 — 2026-11-21

Windows 安装包改为中文：安装向导界面为简体中文，文件名不再带 `en-US` 后缀。

### Windows 安装包

- MSI 安装程序语言设置为简体中文（zh-CN），安装向导全中文
- 文件名从 `BigFish_0.2.2_x64_en-US.msi` 变为 `BigFish_0.2.2_x64_zh-CN.msi`
- 其他平台（macOS / Linux）产物不变

## 0.2.1 — 2026-11-21

全新品牌标识：黑白配色，更简洁、更专业。

### 新 logo

- 应用图标（macOS icns / Windows ico / 通用 png）全部更新为新的黑白「BF」标识
- 启动器界面左上角标题栏 logo 同步更新（不再使用 emoji 🐟，改为官方 logo 图片）
- 说明：logo 从原先的卡通鲸鱼改为「BF」字母标——参考 PCL2、HMCL 等同类启动器，字母标识在不同尺寸下都更清晰、更易识别

### 其他

- 版本号 0.2.0 → 0.2.1

## 0.2.0 — 2026-11-21

加载器自动化：不用再手装 Forge / Fabric，模组也能在启动器里管理。

### 加载器自动安装

- **Fabric / Quilt**：走官方 Meta API 生成版本 JSON 并下载依赖库（纯 Rust，不需要 Java）
- **Forge**：下载官方 installer jar，用本机 Java 执行 headless 安装
- 缺原版时自动先下载原版，再装加载器
- 入口：版本详情页 →「🧩 安装加载器」

### 版本继承合并

- 新增 `resolve_version`：递归解析 `inheritsFrom`，合并父版本的 libraries / arguments / jar / assetIndex
- 修复 `build_classpath` 不认 maven 坐标格式的库（Fabric profile 没有 `downloads` 字段）
- 效果：Forge / Fabric / NeoForge 版本可以正确组装 classpath 并启动（实测 Fabric 0.20.1 + 1.20.1 进主菜单）

### 模组管理

- 入口：版本详情页 →「📦 模组」
- 解析 `fabric.mod.json`（Fabric / Quilt）、`META-INF/mods.toml`、`META-INF/neoforge.mods.toml`、`mcmod.info`（旧版 Forge）
- 显示名称、版本、加载器、大小、简介
- 启用 / 禁用（重命名 `.jar` ↔ `.jar.disabled`）、删除、打开 mods 目录

### 微软正版登录（协议层）

- 完整实现：设备码 OAuth → Xbox Live → XSTS → Minecraft → 玩家档案
- token 持久化在 `~/.bigfish/auth.json`，过期自动刷新
- 设置页新增「🎮 微软正版登录」，设备码直接显示在启动器里
- **需要 `client_id`**：个人 Microsoft 账号无法注册 Azure 应用，需要有 Azure 订阅 / 学校邮箱 / M365 开发者计划之一；没有时仍可正常离线登录

### 其他

- 配置新增 `auth_client_id` / `auth_username` 字段（旧配置自动兼容）
- 新增依赖 `zip`（读取模组 jar 元数据）

### 已知问题

- 微软登录缺 `client_id`，暂时只能用离线登录
- 整合包（Modpack）管理尚未实现
- 启动器自动更新尚未实现

## 0.1.0 — 2026-11-15

首个可用版本，核心链路「下载 → 安装 → 登录 → 启动 → 管理」全部打通。

### 启动

- 版本下载：官方清单 + BMCLAPI 镜像回退，libraries / assets / natives 全自动
- 离线登录：正版 UUID 算法（`MD5("OfflinePlayer:" + 名字)`）
- Forge 实例启动：解析 `arguments.jvm` / `arguments.game`，支持 BootstrapLauncher
- 修正 features 条件参数，不再误加 `--demo` 进试玩模式

### Java

- 按版本 JSON 的 `javaVersion` 自动匹配 Java（1.16 及更早 → 8，1.17~1.20.4 → 17，1.20.5+ → 21）
- 扫描系统 / Homebrew / BigFish 三处 Java 安装
- 缺失时从 Adoptium 一键下载 JRE（实测 Java 8：42MB / 16 秒）

### 界面

- 左栏改为「已安装版本」，标 `local` + 加载器（forge / fabric / neoforge / vanilla）
- 下载页：搜索 + 全部 / 远古版 / 正式版 / 快照版分类，行内进度条
- 版本管理：批量勾选、导出（支持符号链接）、两步确认删除
- 内存分配滑块，自动检测物理内存并给推荐值
- 暗色主题，macOS 原生 .app 打包（双击启动）

### 存档

- NBT 解析 `level.dat`
- 扫描官方 / HMCL / PCL2 目录，一键导入存档

### 已知问题

- 仅支持离线登录，正版账号登录尚未实现
- 只能在 macOS 上运行，Windows / Linux 未验证
- 启动器自身不会自动更新

## 说明

本项目是非官方 Minecraft 启动器，不包含、也不分发任何游戏文件。
使用需要你自己拥有正版的 Minecraft: Java Edition。
