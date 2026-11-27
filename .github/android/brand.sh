#!/bin/bash
# BigFish Android 品牌化脚本
# 用法: brand.sh <amethyst 源码目录>
# 说明: Amethyst-Android (PojavLauncher 后继, LGPL-3.0) 源码原地修改为 BigFish 品牌
set -euo pipefail

AMETHYST="${1:?用法: brand.sh <amethyst 源码目录>}"
ROOT="$(cd "$(dirname "$0")" && pwd)"
RES="$AMETHYST/app_pojavlauncher/src/main/res"
STRINGS="$RES/values/strings.xml"

echo "== [1/7] 应用名 =="
sed -i 's|<string name="app_name" translatable="false">Amethyst</string>|<string name="app_name" translatable="false">大肥鱼</string>|' "$STRINGS"
sed -i 's|<string name="app_short_name" translatable="false">Amethyst</string>|<string name="app_short_name" translatable="false">大肥鱼</string>|' "$STRINGS"
grep -o '<string name="app_name"[^>]*>[^<]*</string>' "$STRINGS" || true
grep -o '<string name="app_short_name"[^>]*>[^<]*</string>' "$STRINGS" || true

echo "== [2/7] 替换图标（删除 adaptive icon xml，让系统用普通图标）=="
rm -f "$RES"/mipmap-anydpi-v26/ic_launcher.xml "$RES"/mipmap-anydpi-v26/ic_launcher_round.xml
for d in mdpi hdpi xhdpi xxhdpi xxxhdpi; do
  dir="$RES/mipmap-$d"
  [ -d "$dir" ] || continue
  rm -f "$dir"/ic_launcher.webp "$dir"/ic_launcher_round.webp
  cp "$ROOT/icons/ic_launcher_$d.png" "$dir/ic_launcher.png"
  cp "$ROOT/icons/ic_launcher_round_$d.png" "$dir/ic_launcher_round.png"
  echo "  mipmap-$d ✓"
done

echo "== [3/7] 关于页/文案（保留 Amethyst 版权署名，不全局替换）=="
# 注意：LGPL-3.0 要求保留上游版权声明，绝不全局替换 "Amethyst"。
# 只改启动器显示名（app_name / app_short_name），致谢页保留原署名。
echo "  保留 Amethyst 版权声明（LGPL 要求）"

echo "== [4/7] 版本号 = 0.2.3（BigFish 模组管理版）=="
GRADLE="$AMETHYST/app_pojavlauncher/build.gradle"
if [ -f "$GRADLE" ]; then
  sed -i 's|versionName getVersionName()|versionName "0.2.3"|' "$GRADLE"
  sed -i 's|versionCode 10000000|versionCode 5|' "$GRADLE"
  grep -E 'versionName|versionCode' "$GRADLE" | head -3
fi

echo "== [5/7] 内置 Vulkan 后端（ArtVK，MC 26.x 原生 Vulkan）=="
VULKAN_DIR="$AMETHYST/app_pojavlauncher/src/main/assets/vulkan"
mkdir -p "$VULKAN_DIR"
ARTVK_URL="https://cdn.modrinth.com/data/FtxBk2DJ/versions/dDReXr6B/artvk-0.1.0.jar"
if curl -sL --max-time 180 "$ARTVK_URL" -o "$VULKAN_DIR/artvk-0.1.0.jar"; then
  SIZE=$(wc -c < "$VULKAN_DIR/artvk-0.1.0.jar" 2>/dev/null || echo 0)
  if [ "$SIZE" -gt 100000 ]; then
    echo "  ArtVK ✓ ${SIZE} bytes"
  else
    echo "  ArtVK 下载不完整（${SIZE} bytes），跳过内置 Vulkan"
    rm -f "$VULKAN_DIR/artvk-0.1.0.jar"
  fi
else
  echo "  ArtVK 下载失败（跳过内置 Vulkan，其余照常构建）"
fi

echo "== [6/7] 渲染引擎设置 + 启动注入补丁 =="
python3 "$ROOT/patch-vulkan.py" "$AMETHYST"

echo "== [7/7] BigFish 界面 + 离线登录 + 模组管理 =="
python3 "$ROOT/patch-ui.py" "$AMETHYST"

echo "品牌化完成 ✅"
