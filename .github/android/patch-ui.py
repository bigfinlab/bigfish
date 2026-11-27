#!/usr/bin/env python3
"""patch-ui.py — BigFish 安卓版界面改造（构建时执行，基于 Amethyst 源码）

步骤：
1. 离线登录修复：放开 Amethyst 对「本地登录」的微软账号限制
   - SelectAuthFragment：本地按钮直接进 LocalLoginFragment（不再弹 hasNoOnlineProfileDialog）
   - LocalLoginFragment：去掉「没有微软账号就踢回主菜单」的检查
2. 主界面换成 BigFish 风格（竖屏单列 + 横屏两栏）
   - layout/fragment_launcher.xml：竖屏单列（logo + 圆角卡片按钮 + 底部版本条）
   - layout-land/fragment_launcher.xml：横屏两栏（左侧版本列表 ListView + 右侧功能面板）
   - colors.xml 深蓝黑 + 绿色启动键、bg_starry.xml、bg_menu_button.xml、logo_bigfish.png
3. 主题配色：深蓝黑 + 绿色启动按钮（和桌面版一致）
4. 主菜单版本列表绑定：MainMenuFragment 给 ListView 设置 ProfileAdapter（仅横屏布局存在）
5. 离线用户解锁功能：去掉「打开游戏文件夹」「安装 JAR」对微软正版账号的依赖
6. 模组管理：新增 BigFishModManagerFragment（列出 mods 目录、启用/停用/删除/打开文件夹），
   主界面「模组管理」按钮（原 WIKI 按钮位）进入
7. 校验：两套布局都必须保留原有 view id（否则 MainMenuFragment 会 NPE）

依赖 ui/ 目录：fragment_launcher.xml / layout-land/fragment_launcher.xml / colors.xml /
bg_starry.xml / bg_menu_button.xml / logo_bigfish.png / layout/*.xml / fragments/*.java
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
    print('[1/7] 离线登录修复...')
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
    print('[2/7] 复制 BigFish 界面资源...')
    layout_dir = os.path.join(RES, 'layout')
    layout_land_dir = os.path.join(RES, 'layout-land')
    drawable_dir = os.path.join(RES, 'drawable')
    nodpi_dir = os.path.join(RES, 'drawable-nodpi')

    # 竖屏主界面（默认 layout）
    shutil.copy(os.path.join(UI, 'fragment_launcher.xml'),
                os.path.join(layout_dir, 'fragment_launcher.xml'))
    print('  ✓ layout/fragment_launcher.xml（竖屏单列）')

    # 横屏主界面（两栏：左侧版本列表 + 右侧功能面板）
    os.makedirs(layout_land_dir, exist_ok=True)
    shutil.copy(os.path.join(UI, 'layout-land/fragment_launcher.xml'),
                os.path.join(layout_land_dir, 'fragment_launcher.xml'))
    print('  ✓ layout-land/fragment_launcher.xml（横屏两栏）')

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

    # 模组管理界面的布局（fragment + item）
    layout_ui_dir = os.path.join(UI, 'layout')
    for name in sorted(os.listdir(layout_ui_dir)):
        if name.endswith('.xml'):
            shutil.copy(os.path.join(layout_ui_dir, name), os.path.join(layout_dir, name))
            print(f'  ✓ layout/{name}')

    # 模组管理 Fragment 的 Java 源码
    fragments_dir = os.path.join(JAVA, 'fragments')
    java_ui_dir = os.path.join(UI, 'fragments')
    for name in sorted(os.listdir(java_ui_dir)):
        if name.endswith('.java'):
            shutil.copy(os.path.join(java_ui_dir, name), os.path.join(fragments_dir, name))
            print(f'  ✓ fragments/{name}')


def patch_styles():
    print('[3/7] 调整按钮样式...')
    path = os.path.join(RES, 'values/styles.xml')
    src = read(path)
    old = ('<style name="LauncherMenuButton" parent="Widget.AppCompat.Button">\n'
           '        <item name="android:background">?android:attr/selectableItemBackground</item>\n'
           '    </style>')
    new = ('<style name="LauncherMenuButton" parent="Widget.AppCompat.Button">\n'
           '        <item name="android:background">@drawable/bg_menu_button</item>\n'
           '    </style>')
    if old not in src:
        print('  ⚠️ styles.xml 未找到标准 LauncherMenuButton 定义（可能上游已改），跳过')
        return
    src = src.replace(old, new)
    src = src.replace('<item name="android:layout_height">@dimen/_66sdp</item>',
                      '<item name="android:layout_height">58dp</item>')
    write(path, src)
    print('  ✓ LauncherMenuButton 改为圆角卡片背景')


def patch_main_menu():
    print('[4/7] 主菜单版本列表绑定...')
    path = os.path.join(JAVA, 'fragments/MainMenuFragment.java')
    src = read(path)
    anchor = 'mVersionSpinner = view.findViewById(R.id.mc_version_spinner);'
    if anchor not in src:
        failures.append('MainMenuFragment: 锚点未找到（上游可能已改）')
        print('  ⚠️ MainMenuFragment 锚点未找到')
        return
    inject = (
        anchor + '\n'
        '\n'
        '        // BigFish: bind the landscape version list (only present in layout-land)\n'
        '        android.widget.ListView bfVersionList = view.findViewById(R.id.version_list);\n'
        '        if (bfVersionList != null) {\n'
        '            bfVersionList.setAdapter(mVersionSpinner.getProfileAdapter());\n'
        '            bfVersionList.setOnItemClickListener((parent, itemView, position, id) -> {\n'
        '                mVersionSpinner.setProfileSelection(position);\n'
        '                bfVersionList.invalidateViews();\n'
        '            });\n'
        '        }'
    )
    write(path, src.replace(anchor, inject, 1))
    print('  ✓ MainMenuFragment 已绑定版本列表（横屏）')


def patch_offline_and_mods():
    print('[5/7] 离线用户解锁功能 + 模组管理入口...')
    path = os.path.join(JAVA, 'fragments/MainMenuFragment.java')
    src = read(path)

    # 1) 主界面「模组管理」按钮（复用原 WIKI 按钮位）→ 打开 BigFishModManagerFragment
    old = '        mNewsButton.setOnClickListener(v -> Tools.openURL(requireActivity(), Tools.URL_HOME));'
    new = ('        // BigFish: WIKI button replaced with the mod manager entry\n'
           '        mNewsButton.setOnClickListener(v -> Tools.swapFragment(requireActivity(), '
           'BigFishModManagerFragment.class, BigFishModManagerFragment.TAG, null));')
    if old not in src:
        failures.append('MainMenuFragment: 模组管理入口锚点未找到（上游可能已改）')
        print('  ⚠️ 模组管理入口锚点未找到')
    else:
        src = src.replace(old, new, 1)
        print('  ✓ 主界面按钮 → 模组管理')

    # 2) 安装 JAR：不再要求微软正版账号（离线账号也能装）
    old = (
        '        if (hasOnlineProfile()) {\n'
        '            mInstallJarButton.setOnClickListener(v -> runInstallerWithConfirmation(false));\n'
        '            mInstallJarButton.setOnLongClickListener(v -> {\n'
        '                runInstallerWithConfirmation(true);\n'
        '                return true;\n'
        '            });\n'
        '        } else mInstallJarButton.setOnClickListener(v -> hasNoOnlineProfileDialog(requireActivity()));'
    )
    new = (
        '        mInstallJarButton.setOnClickListener(v -> runInstallerWithConfirmation(false));\n'
        '        mInstallJarButton.setOnLongClickListener(v -> {\n'
        '            runInstallerWithConfirmation(true);\n'
        '            return true;\n'
        '        });'
    )
    if old not in src:
        failures.append('MainMenuFragment: 安装 JAR 账号限制锚点未找到（上游可能已改）')
        print('  ⚠️ 安装 JAR 账号限制锚点未找到')
    else:
        src = src.replace(old, new, 1)
        print('  ✓ 安装 JAR 不再要求微软账号')

    # 3) 打开游戏文件夹：不再要求微软正版账号
    old = (
        '            } else if (!hasOnlineProfile()) { // Otherwise display the generic pop-up to log in\n'
        '                hasNoOnlineProfileDialog(requireActivity());\n'
        '            } else openPath(v.getContext(), getCurrentProfileDirectory(), false);'
    )
    new = '            } else openPath(v.getContext(), getCurrentProfileDirectory(), false);'
    if old not in src:
        failures.append('MainMenuFragment: 打开文件夹账号限制锚点未找到（上游可能已改）')
        print('  ⚠️ 打开文件夹账号限制锚点未找到')
    else:
        src = src.replace(old, new, 1)
        print('  ✓ 打开游戏文件夹不再要求微软账号')

    write(path, src)


def verify():
    print('[7/7] 校验...')
    # 两个布局都需要的 id（MainMenuFragment 会 findViewById）
    common_ids = [
        'news_button', 'discord_button', 'custom_control_button', 'install_jar_button',
        'share_logs_button', 'open_files_button', 'mc_version_spinner',
        'edit_profile_button', 'play_button',
    ]
    # 竖屏布局额外需要（背景装饰 + 底部版本条相关）
    portrait_extra = ['_background_display_view']
    # 横屏布局额外需要（版本列表）
    landscape_extra = ['version_list']

    checks = [
        ('layout/fragment_launcher.xml', common_ids + portrait_extra),
        ('layout-land/fragment_launcher.xml', common_ids + landscape_extra),
    ]
    for rel, required_ids in checks:
        path = os.path.join(RES, rel)
        if not os.path.exists(path):
            failures.append(f'缺少布局 {rel}')
            print(f'  ⚠️ 缺少 {rel}')
            continue
        content = read(path)
        missing = [i for i in required_ids if f'@+id/{i}' not in content and f'@id/{i}' not in content]
        if missing:
            failures.append(f'{rel} 缺少 id: {missing}')
            print(f'  ⚠️ {rel} 缺少 id: {missing}')
        else:
            print(f'  ✓ {rel} 全部必要 id 保留')

    # 模组管理界面资源
    mod_layout = os.path.join(RES, 'layout/fragment_bigfish_mod_manager.xml')
    mod_item = os.path.join(RES, 'layout/item_bigfish_mod.xml')
    if not os.path.exists(mod_layout):
        failures.append('缺少 layout/fragment_bigfish_mod_manager.xml')
    elif not os.path.exists(mod_item):
        failures.append('缺少 layout/item_bigfish_mod.xml')
    else:
        layout_src = read(mod_layout)
        item_src = read(mod_item)
        need_layout = ['bf_mod_list', 'bf_mod_empty', 'bf_mod_back', 'bf_mod_refresh', 'bf_mod_open_dir']
        need_item = ['bf_mod_item_name', 'bf_mod_item_state']
        miss = [i for i in need_layout if f'@+id/{i}' not in layout_src]
        miss += [i for i in need_item if f'@+id/{i}' not in item_src]
        if miss:
            failures.append(f'模组管理布局缺少 id: {miss}')
            print(f'  ⚠️ 模组管理布局缺少 id: {miss}')
        else:
            print('  ✓ 模组管理布局 id 完整')

    # 模组管理 Fragment 源码
    mod_java = os.path.join(JAVA, 'fragments/BigFishModManagerFragment.java')
    if not os.path.exists(mod_java):
        failures.append('缺少 fragments/BigFishModManagerFragment.java')
    else:
        print('  ✓ fragments/BigFishModManagerFragment.java')

    # MainMenuFragment 改造点
    menu_src = read(os.path.join(JAVA, 'fragments/MainMenuFragment.java'))
    if 'BigFishModManagerFragment' not in menu_src:
        failures.append('MainMenuFragment 未接入模组管理入口')
    elif 'if (hasOnlineProfile()) {' in menu_src:
        failures.append('MainMenuFragment 仍有 hasOnlineProfile 限制（离线用户会被拦）')
    else:
        print('  ✓ MainMenuFragment 模组管理入口 + 离线解锁')


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
    patch_main_menu()
    patch_offline_and_mods()
    verify()

    if failures:
        print('\n❌ patch-ui 失败：')
        for f in failures:
            print('  -', f)
        sys.exit(1)
    print('\n✅ patch-ui 完成（BigFish 界面 + 离线登录 + 模组管理 + 横屏版本列表）')


if __name__ == '__main__':
    main()
