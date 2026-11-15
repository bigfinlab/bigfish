# 大肥鱼启动器（BigFish）

> 一个**非官方**的开源 Minecraft Java 版启动器 🐟
>
> 由一名学生开发者和 DeepSeek「大肥鱼」一起做出来。

---

## ⚠️ 重要声明（请先阅读）

- 本项目**不是** Mojang、Microsoft、网易的官方产品，与其**无任何关联、无任何授权**。
- 本项目**不包含、不托管、不重新分发任何 Minecraft 游戏文件**。启动器只是按官方的版本清单，从 Mojang 官方地址（或国内镜像 BMCLAPI）下载用户请求的文件。
- 使用本项目**需要你拥有合法的 Minecraft: Java Edition 正版账号**，或自行合法获取游戏文件。
- 本项目**不提供**任何破解、盗版分发、防沉迷绕过、内购绕过功能。
- 本项目**不收集**任何用户数据，不上报任何信息，不联网回传任何内容（唯一的网络请求是下载游戏文件本身）。
- 使用者自行承担使用本软件的全部风险；因使用本软件造成的任何损失，作者不承担责任。

## ✨ 功能现状

**已完成**

- [x] 版本下载：官方清单 + BMCLAPI 国内镜像回退，libraries / assets / natives 自动处理
- [x] 离线登录：使用正版 UUID 算法（`MD5("OfflinePlayer:" + 名字)`）
- [x] 启动：原版 / Forge（BootstrapLauncher 参数完整解析）都能进游戏
- [x] 加载器识别：`local` + `forge` / `fabric` / `neoforge` / `quilt` / `optifine` / `vanilla`
- [x] 存档导入：NBT 解析 `level.dat`，扫描官方 / HMCL / PCL2 目录一键导入
- [x] Java 管理：按版本 JSON 自动匹配，缺失时从 Adoptium 下载 JRE
- [x] 内存分配：滑条调节，自动检测物理内存
- [x] 版本管理：批量导出（支持符号链接）、两步确认删除
- [x] 关于页：版本信息 + 内嵌更新日志

**未完成**

- [ ] 微软正版登录（目前仅离线登录）
- [ ] Windows / Linux 实际构建验证（代码已适配，需要对应平台）
- [ ] Forge / Fabric 自动安装
- [ ] 模组与整合包管理
- [ ] 启动器自动更新

## 💻 系统要求

| 平台 | 状态 | 说明 |
|---|---|---|
| macOS 12+ | ✅ 已验证 | Intel / Apple Silicon |
| Windows 10+ | 🟡 代码已适配，未验证 | 需要 WebView2（Win10+ 自带） |
| Linux | 🟡 代码已适配，未验证 | 需要 WebKitGTK 4.1 |

游戏本身还需要 Java：1.16 及更早要 Java 8，1.17~1.20.4 要 Java 17，1.20.5+ 要 Java 21。**启动器会自动检测并下载**，不需要你手动装。

## 📦 安装

**macOS**

目前需要从源码构建（见下方），产物是 `BigFish.app`，双击即可运行。

首次打开如果提示「无法验证开发者」（因为没做代码签名）：

```
右键 → 打开
```

或者在终端执行：

```bash
xattr -dr com.apple.quarantine /path/to/BigFish.app
```

**Windows / Linux**

见 [PACKAGING.md](PACKAGING.md)。

## 🚀 使用

1. 打开启动器，左上角 `＋` 号进入下载页
2. 搜索或按分类（全部 / 远古版 / 正式版 / 快照版）选一个版本，点「下载」
3. 下载完成后在左栏选中它 → 「▶ 启动游戏」
4. 存档可以在「📂 存档」里从旧启动器导入
5. 「⚙ 设置」里可以改玩家名、游戏目录、内存、Java

## 🛠️ 从源码构建

需要 [Rust](https://rustup.rs) 1.80+（2024 edition）。

```bash
# 编译 Tauri 应用
cd src-tauri
cargo build

# macOS 一键打包成 .app（在工作区生成 BigFish.app）
cd ..
./package-macos.sh
```

界面是单文件 HTML（`ui/index.html`），编译时通过 Tauri 的 `generate_context!` **嵌入二进制**，所以改完界面必须重新 `cargo build`。

## 📁 项目结构

```
bigfish/
├── src/
│   ├── lib.rs          核心库（下载 / 启动 / 存档 / Java / 配置）
│   └── main.rs         CLI（调试用：versions / download / launch / java）
├── src-tauri/
│   ├── src/main.rs     Tauri 命令层
│   ├── tauri.conf.json 窗口与打包配置
│   └── icons/          应用图标（png / icns / ico）
├── ui/index.html       界面（暗色主题，单文件）
├── CHANGELOG.md        更新日志（编译进「关于」页）
├── PACKAGING.md        各平台打包说明
├── PLAN.md             开发计划与进度
└── package-macos.sh    macOS 一键打包
```

## 🧭 设计取舍

- **不用前端框架**：界面就是一个 HTML 文件，启动快、没有构建步骤。
- **不内置游戏文件**：所有文件按需从官方清单下载，避免版权问题。
- **不做账号系统**：离线登录直接本地算 UUID，不经过任何服务器。

## 📄 许可证

[MIT](LICENSE)

简单说：随便用、随便改、可以商用，保留署名即可，作者不为使用后果负责。

## 🙏 致谢

- 版本清单与下载：Mojang 官方 API、[BMCLAPI](https://bmclapi2.bangbang93.com) 镜像
- Java 运行时：[Adoptium Temurin](https://adoptium.net)
- 桌面框架：[Tauri](https://tauri.app)

---

> Minecraft 是 Mojang Studios 的商标。本项目与 Mojang 无关。
