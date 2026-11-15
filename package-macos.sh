#!/usr/bin/env bash
# 重新编译 BigFish 并更新工作区里的 .app（双击即可启动）
# 注意：不会往 ~/Applications 或 /Applications 安装任何东西
set -euo pipefail
cd "$(dirname "$0")"

echo "==> 编译 Tauri 应用..."
cargo build --manifest-path src-tauri/Cargo.toml

echo "==> 更新 BigFish.app..."
cp src-tauri/target/debug/bigfish-app BigFish.app/Contents/MacOS/BigFish
chmod +x BigFish.app/Contents/MacOS/BigFish

echo ""
echo "✅ 完成：$(pwd)/BigFish.app"
echo "   双击启动，或运行：open \"$(pwd)/BigFish.app\""
