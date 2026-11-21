# 跨平台打包说明

BigFish 基于 Tauri 2（Rust + WKWebView/WebView2/WebKitGTK），每个平台都要在**对应系统上构建**，无法在一台机器上直接交叉编译出全部平台。

## 当前状态

| 平台 | 状态 | 说明 |
|---|---|---|
| macOS | ✅ 可构建 | 本机已验证，见下方「macOS」 |
| Windows | ⏳ 代码已适配，未验证 | 需要 Windows 机器或 CI |
| Linux | ⏳ 代码已适配，未验证 | 需要 Linux 机器或 CI |

代码层面的跨平台处理已经完成：

- 主目录：macOS/Linux 用 `$HOME`，Windows 用 `%USERPROFILE%`
- Java 扫描：三个平台各自的 JDK 常见安装路径
- Java 下载：Windows 解压 `.zip`，其他平台解压 `.tar.gz`
- 物理内存：macOS `sysctl` / Linux `/proc/meminfo` / Windows 默认值
- 系统版本：macOS `sw_vers` / Linux `/etc/os-release`
- 启动参数：macOS 自动补 `-XstartOnFirstThread`
- 窗口样式：Tauri 各平台原生

## macOS（本机已验证）

```bash
cd bigfish
./package-macos.sh
```

产物：`bigfish/BigFish.app`，双击即可运行。

> 注意：这是 debug 构建。要出正式版用 `cargo tauri build`（会跑 release 编译，第一次约 20~40 分钟）。

## Windows

在 Windows 机器上（需要先装 [Rust](https://rustup.rs) 和 [Node.js](https://nodejs.org) 可选）：

```powershell
cd bigfish\src-tauri
cargo install tauri-cli --version "^2" --locked
cargo tauri build
```

产物：`src-tauri\target\release\bundle\msi\BigFish_0.1.0_x64_en-US.msi`

前置条件：

- **WebView2**：Windows 10（1803+）和 Windows 11 自带；更老的系统需要单独装
- Visual Studio Build Tools（C++ 生成工具）

## Linux

Ubuntu / Debian：

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

cd bigfish/src-tauri
cargo install tauri-cli --version "^2" --locked
cargo tauri build
```

产物：

- `src-tauri/target/release/bundle/deb/*.deb`
- `src-tauri/target/release/bundle/appimage/*.AppImage`

## 用 GitHub Actions 一次构建全部平台

已经准备好了 workflow：`.github/workflows/build.yml`

使用方法：

1. 把项目推到 GitHub（去掉 `.gitignore` 里排除的东西）
2. 在仓库的 **Actions** 页面点 **Build BigFish** → **Run workflow**
3. 或者打 tag：`git tag v0.2.0 && git push origin v0.2.0`

构建完成后，在 Actions 的 artifacts 里能下载：

- `BigFish-macos-arm64` / `BigFish-macos-x64`
- `BigFish-linux-x64`
- `BigFish-windows-x64`

## 已知的平台差异

| 项目 | macOS | Windows | Linux |
|---|---|---|---|
| 窗口框架 | WKWebView | WebView2 | WebKitGTK |
| 打包格式 | `.app` / `.dmg` | `.msi` / `.exe` | `.deb` / `.AppImage` |
| 代码签名 | 需要（否则 Gatekeeper 拦） | 可选 | 不需要 |
| 首次启动 | 右键 → 打开 | 直接双击 | 需要 `chmod +x` |

## 发布前的检查清单

- [ ] 各平台都实际启动过一次
- [ ] Java 自动下载在各平台验证（Windows 的 zip 解压、Linux 的 tar 解压）
- [ ] 各平台路径分隔符（Windows 用 `\`，但我们内部统一用 `PathBuf`，应该没问题）
- [ ] 图标在各平台显示正常（`.icns` / `.ico` / `.png`）
- [ ] LICENSE 文件已加
