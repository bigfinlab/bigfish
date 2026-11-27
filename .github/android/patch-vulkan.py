#!/usr/bin/env python3
# BigFish: 内置 Vulkan 后端（ArtVK）+ 渲染引擎设置
# 用法: python3 patch-vulkan.py <amethyst 源码目录>
# 说明: 修改 Amethyst 源码（LGPL-3.0），只为 BigFish 增加内置 Vulkan mod 注入与设置项；
#       不修改上游版权声明，改动均为可选的增量功能。
import sys
import pathlib

root = pathlib.Path(sys.argv[1]).resolve()
res = root / "app_pojavlauncher/src/main/res"
java = root / "app_pojavlauncher/src/main/java/net/kdt/pojavlaunch"
tools_java = java / "Tools.java"
pref_video = res / "xml/pref_video.xml"

print("== [A] 写入渲染引擎设置数组 ==")
(res / "values").mkdir(parents=True, exist_ok=True)
(res / "values/bf_arrays.xml").write_text(
    '<?xml version="1.0" encoding="utf-8"?>\n'
    "<resources>\n"
    '    <string-array name="bf_vulkan_names">\n'
    "        <item>Auto (recommended) - MC 26.2+ uses native Vulkan</item>\n"
    "        <item>Force enable - inject ArtVK on all versions</item>\n"
    "        <item>Disable - OpenGL only</item>\n"
    "    </string-array>\n"
    '    <string-array name="bf_vulkan_values">\n'
    "        <item>auto</item>\n"
    "        <item>on</item>\n"
    "        <item>off</item>\n"
    "    </string-array>\n"
    "</resources>\n",
    encoding="utf-8",
)
(res / "values-zh-rCN").mkdir(parents=True, exist_ok=True)
(res / "values-zh-rCN/bf_arrays.xml").write_text(
    '<?xml version="1.0" encoding="utf-8"?>\n'
    "<resources>\n"
    '    <string-array name="bf_vulkan_names">\n'
    "        <item>自动（推荐）— MC 26.2+ 使用原生 Vulkan</item>\n"
    "        <item>强制开启 — 所有版本注入 ArtVK</item>\n"
    "        <item>关闭 — 只用 OpenGL</item>\n"
    "    </string-array>\n"
    "</resources>\n",
    encoding="utf-8",
)
print("  values/bf_arrays.xml + values-zh-rCN/bf_arrays.xml ✓")

print("== [B] 视频设置加「Vulkan 渲染引擎」选项 ==")
src = pref_video.read_text(encoding="utf-8")
anchor = """        <Preference
            android:title="@string/mcl_setting_title_renderer_settings"
            android:summary="@string/mcl_setting_title_renderer_subtitle"
            android:icon="@drawable/ic_setting_engine"
            android:fragment="net.kdt.pojavlaunch.prefs.screens.LauncherPreferenceRendererSettingsFragment"
            />"""
if anchor not in src:
    sys.exit("ERROR: pref_video.xml 未找到渲染器设置锚点（Amethyst 版本可能有变）")
insert = anchor + """

        <ListPreference
            android:key="bf_vulkan_mode"
            android:title="Vulkan 渲染引擎（大肥鱼）"
            android:summary="%s"
            android:entries="@array/bf_vulkan_names"
            android:entryValues="@array/bf_vulkan_values"
            android:defaultValue="auto" />"""
src = src.replace(anchor, insert, 1)
pref_video.write_text(src, encoding="utf-8")
print("  pref_video.xml ✓")

print("== [C] 启动时注入 ArtVK ==")
tsrc = tools_java.read_text(encoding="utf-8")

# C1. 在 launchMinecraft 里拿到 gamedir 之后调用注入
gamedir_anchor = "        File gamedir = Tools.getGameDirPath(minecraftProfile);"
if gamedir_anchor not in tsrc:
    sys.exit("ERROR: Tools.java 未找到 gamedir 锚点（Amethyst 版本可能有变）")
if "injectBundledVulkan(activity, gamedir, versionId)" not in tsrc:
    tsrc = tsrc.replace(
        gamedir_anchor,
        gamedir_anchor + "\n        injectBundledVulkan(activity, gamedir, versionId);",
        1,
    )

# C2. 在类末尾追加方法
method = '''
    /** BigFish: 注入内置 Vulkan 后端（ArtVK，MC 26.x 原生 Vulkan 渲染） */
    public static void injectBundledVulkan(android.content.Context ctx, File gamedir, String versionId) {
        try {
            String mode = net.kdt.pojavlaunch.prefs.LauncherPreferences.DEFAULT_PREF
                    .getString("bf_vulkan_mode", "auto");
            if ("off".equals(mode)) return;
            boolean isNewMc = versionId != null && versionId.contains("26.2");
            if ("auto".equals(mode) && !isNewMc) return;
            File modsDir = new File(gamedir, "mods");
            if (!modsDir.exists() && !modsDir.mkdirs()) return;
            File artvk = new File(modsDir, "artvk-0.1.0.jar");
            if (artvk.exists() && artvk.length() > 100000) return;
            try (java.io.InputStream in = ctx.getAssets().open("vulkan/artvk-0.1.0.jar");
                 java.io.FileOutputStream out = new java.io.FileOutputStream(artvk)) {
                byte[] buf = new byte[8192];
                int n;
                while ((n = in.read(buf)) > 0) out.write(buf, 0, n);
            }
            Log.i("BigFish", "已注入 ArtVK: " + artvk.getAbsolutePath());
        } catch (Exception e) {
            Log.w("BigFish", "ArtVK 注入失败: " + e.getMessage());
        }
    }
'''
if "public static void injectBundledVulkan" not in tsrc:
    last = tsrc.rstrip().rfind("\n}")
    if last < 0:
        sys.exit("ERROR: Tools.java 结构异常，找不到类结束符")
    tsrc = tsrc[:last] + method + tsrc[last:]
    tools_java.write_text(tsrc, encoding="utf-8")
print("  Tools.java ✓（注入 + 方法追加）")

print("Vulkan 补丁完成 ✅")
