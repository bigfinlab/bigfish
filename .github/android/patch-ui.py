#!/usr/bin/env python3
"""patch-ui.py — BigFish 安卓版界面改造（构建时执行，基于 Amethyst 源码）

做三件事：
1. 离线登录修复：放开 Amethyst 对「本地登录」的微软账号限制
   - SelectAuthFragment：本地按钮直接进 LocalLoginFragment（不再弹 hasNoOnlineProfileDialog）
   - LocalLoginFragment：去掉「没有微软账号就踢回主菜单」的检查
2. 主界面换成 BigFish 风格：深蓝星空渐变背景 + BF logo + 品牌标题 + 圆角卡片按钮
   （必须保留原有 view id：news_button / discord_button / custom_control_button /
     install_jar_button / share_logs_button / open_files_button /
     mc_version_spinner / edit_profile_button / play_button / _background_display_view）
3. 主题配色：深蓝黑 + 绿色启动按钮（和桌面版一致）

依赖 ui/ 目录：fragment_launcher.xml / colors.xml / bg_starry.xml / bg_menu_button.xml / logo_bigfish.png
"""
import os
import shutil
import sys

AMETHYST = 'amethyst'
RES = os.path.join(AMETHYST, 'app_pojavlauncher/src/main/res')
JAVA = os.path.join(AMETHYST, 'app_pojavlauncher/src/main/java/net/kdt/pojavlaunch')
HERE = os.path.dirname(os.path.abspath(__file__))
UI = os.path.join(HERE, 'ui')

failures = []


def read(path):
    with open(path, encoding='utf-8') as f:
        return f.read()


def write(path, content):
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)


def patch_local_login():
    print('[1/4] 离线登录修复...')
    # 1) SelectAuthFragment：本地按钮直接跳本地登录
    path = os.path.join(JAVA, 'fragments/SelectAuthFragment.java')
    src = read(path)
    old = ('mLocalButton.setOnClickListener(v -> hasNoOnlineProfileDialog(requireActivity(), '
           '() -> Tools.swapFragment(requireActivity(), LocalLoginFragment.class, LocalLoginFragment.TAG, null)));')
    new = ('mLocalButton.setOnClickListener(v -> Tools.swapFragment(requireActivity(), '
           'LocalLoginFragment.class, LocalLoginFragment.TAG, null));')
    if old not in src:
        failures.append('SelectAuthFragment: 目标行未找到（上游可能已改）')
        print('  ⚠️ SelectAuthFragment 目标行未找到')
    else:
        write(path, src.replace(old, new))
        print('  ✓ SelectAuthFragment 本地登录按钮已放开')

    # 2) LocalLoginFragment：删掉「没有微软账号就跳回主菜单」
    path = os.path.join(JAVA, 'fragments/LocalLoginFragment.java')
    src = read(path)
    old = (
        '        // This is overkill but meh\n'
        '        if (!hasOnlineProfile()){\n'
        '            Tools.swapFragment(requireActivity(), MainMenuFragment.class, MainMenuFragment.TAG, null);\n'
        '        }\n'
    )
    if old not in src:
        failures.append('LocalLoginFragment: 检查块未找到（上游可能已改）')
        print('  ⚠️ LocalLoginFragment 检查块未找到')
    else:
        write(path, src.replace(old, ''))
        print('  ✓ LocalLoginFragment 已去掉微软账号检查')


def copy_ui_assets():
    print('[2/4] 复制 BigFish 界面资源...')
    layout_dir = os.path.join(RES, 'layout')
    drawable_dir = os.path.join(RES, 'drawable')
    nodpi_dir = os.path.join(RES, 'drawable-nodpi')

    # 主界面布局（竖屏）
    dst = os.path.join(layout_dir, 'fragment_launcher.xml')
    shutil.copy(os.path.join(UI, 'fragment_launcher.xml'), dst)
    print('  ✓ layout/fragment_launcher.xml')

    # 颜色
    shutil.copy(os.path.join(UI, 'colors.xml'), os.path.join(RES, 'values/colors.xml'))
    print('  ✓ values/colors.xml')

    # drawable
    os.makedirs(drawable_dir, exist_ok=True)
    for name in ('bg_starry.xml', 'bg_menu_button.xml'):
        shutil.copy(os.path.join(UI, name), os.path.join(drawable_dir, name))
        print(f'  ✓ drawable/{name}')

    # logo（避免被 dpi 缩放，放 nodpi）
    os.makedirs(nodpi_dir, exist_ok=True)
    shutil.copy(os.path.join(UI, 'logo_bigfish.png'), os.path.join(nodpi_dir, 'logo_bigfish.png'))
    print('  ✓ drawable-nodpi/logo_bigfish.png')


def patch_styles():
    print('[3/4] 调整按钮样式...')
    path = os.path.join(RES, 'values/styles.xml')
    src = read(path)
    old = ('<style name="LauncherMenuButton" parent="Widget.AppCompat.Button">\n'
           '        <item name="android:background">?android:attr/selectableItemBackground</item>\n'
           '    </style>')
    new = ('<style name="LauncherMenuButton" parent="Widget.AppCompat.Button">\n'
           '        <item name="android:background">@drawable/bg_menu_button</item>\n'
           '    </style>')
    if old not in src:
        # 上游可能已经是别的背景，幂等处理：不视为失败
        print('  ⚠️ styles.xml 未找到标准 LauncherMenuButton 定义（可能上游已改），跳过')
        return
    src = src.replace(old, new)
    # 按钮高度 66sdp -> 58dp，更适合卡片列表
    src = src.replace('<item name="android:layout_height">@dimen/_66sdp</item>',
                      '<item name="android:layout_height">58dp</item>')
    write(path, src)
    print('  ✓ LauncherMenuButton 改为圆角卡片背景')


def verify():
    print('[4/4] 校验...')
    required_ids = [
        'news_button', 'discord_button', 'custom_control_button', 'install_jar_button',
        'share_logs_button', 'open_files_button', 'mc_version_spinner',
        'edit_profile_button', 'play_button', '_background_display_view',
    ]
    layout_path = os.path.join(RES, 'layout/fragment_launcher.xml')
    content = read(layout_path)
    missing = [i for i in required_ids if f'@+id/{i}' not in content and f'@id/{i}' not in content]
    if missing:
        failures.append(f'布局缺少必须保留的 id: {missing}')
        print(f'  ⚠️ 缺少 id: {missing}')
    else:
        print('  ✓ 所有必要 view id 保留')


def main():
    if not os.path.isdir(AMETHYST):
        print('❌ 找不到 amethyst 目录，请先 clone 源码')
        sys.exit(1)
    if not os.path.isdir(UI):
        print('❌ 找不到 ui 资源目录')
        sys.exit(1)

    patch_local_login()
    copy_ui_assets()
    patch_styles()
    verify()

    if failures:
        print('\n❌ patch-ui 失败：')
        for f in failures:
            print('  -', f)
        sys.exit(1)
    print('\n✅ patch-ui 完成（BigFish 界面 + 离线登录）')


if __name__ == '__main__':
    main()
