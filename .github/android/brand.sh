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

echo "== [3/4] 关于页/文案（保留 Amethyst 版权署名，不全局替换）=="
# 注意：LGPL-3.0 要求保留上游版权声明，绝不全局替换 "Amethyst"。
# 只改启动器显示名（app_name / app_short_name），致谢页保留原署名。
echo "  保留 Amethyst 版权声明（LGPL 要求）"

echo "== [4/4] 版本号 = 0.1.0（BigFish Android 首发版）=="
GRADLE="$AMETHYST/app_pojavlauncher/build.gradle"
if [ -f "$GRADLE" ]; then
  sed -i 's|versionName getVersionName()|versionName "0.1.0"|' "$GRADLE"
  sed -i 's|versionCode 10000000|versionCode 1|' "$GRADLE"
  grep -E 'versionName|versionCode' "$GRADLE" | head -3
fi

echo "品牌化完成 ✅"
