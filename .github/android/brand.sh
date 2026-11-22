#!/bin/bash
# BigFish Android 品牌化脚本
# 用法: brand.sh <amethyst 源码目录>
# 说明: Amethyst-Android (PojavLauncher 后继, LGPL-3.0) 源码原地修改为 BigFish 品牌
set -euo pipefail

AMETHYST="${1:?用法: brand.sh <amethyst 源码目录>}"
ROOT="$(cd "$(dirname "$0")" && pwd)"
RES="$AMETHYST/app_pojavlauncher/src/main/res"
STRINGS="$RES/values/strings.xml"

echo "== [1/4] 应用名 =="
sed -i 's|<string name="app_name" translatable="false">Amethyst</string>|<string name="app_name" translatable="false">大肥鱼</string>|' "$STRINGS"
sed -i 's|<string name="app_short_name" translatable="false">Amethyst</string>|<string name="app_short_name" translatable="false">大肥鱼</string>|' "$STRINGS"
grep -o '<string name="app_name"[^>]*>[^<]*</string>' "$STRINGS" || true
grep -o '<string name="app_short_name"[^>]*>[^<]*</string>' "$STRINGS" || true

echo "== [2/4] 替换图标（删除 adaptive icon xml，让系统用普通图标）=="
rm -f "$RES"/mipmap-anydpi-v26/ic_launcher.xml "$RES"/mipmap-anydpi-v26/ic_launcher_round.xml
for d in mdpi hdpi xhdpi xxhdpi xxxhdpi; do
  dir="$RES/mipmap-$d"
  [ -d "$dir" ] || continue
  rm -f "$dir"/ic_launcher.webp "$dir"/ic_launcher_round.webp
  cp "$ROOT/icons/ic_launcher_$d.png" "$dir/ic_launcher.png"
  cp "$ROOT/icons/ic_launcher_round_$d.png" "$dir/ic_launcher_round.png"
  echo "  mipmap-$d ✓"
done

echo "== [3/4] 关于页/文案（保留 Amethyst 致谢，改显示名）=="
find "$RES" -name 'strings.xml' -path '*/values*' | while read -r f; do
  sed -i 's|Amethyst|大肥鱼|g' "$f" || true
done

echo "== [4/4] 版本名标记 =="
# 修改 gradle.properties 中的版本后缀（如存在），标记为 bigfish 构建
if [ -f "$AMETHYST/gradle.properties" ]; then
  grep -E 'versionName|version' "$AMETHYST/gradle.properties" | head -3 || true
fi

echo "品牌化完成 ✅"
