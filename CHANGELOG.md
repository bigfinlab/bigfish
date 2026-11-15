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
