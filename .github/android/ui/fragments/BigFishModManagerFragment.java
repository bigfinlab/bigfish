package net.kdt.pojavlaunch.fragments;

import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.BaseAdapter;
import android.widget.ListView;
import android.widget.TextView;

import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import androidx.appcompat.app.AlertDialog;
import androidx.fragment.app.Fragment;

import net.kdt.pojavlaunch.R;
import net.kdt.pojavlaunch.Tools;
import net.kdt.pojavlaunch.prefs.LauncherPreferences;
import net.kdt.pojavlaunch.value.launcherprofiles.LauncherProfiles;
import net.kdt.pojavlaunch.value.launcherprofiles.MinecraftProfile;

import java.io.File;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * BigFish 模组管理界面：列出当前版本 mods 文件夹里的内容，
 * 支持启用/停用（重命名 .jar <-> .jar.disabled）、删除、打开文件夹。
 */
public class BigFishModManagerFragment extends Fragment {
    public static final String TAG = "BigFishModManagerFragment";
    private static final String SUFFIX_DISABLED = ".disabled";

    private ListView mListView;
    private TextView mEmptyView;
    private final List<File> mMods = new ArrayList<>();
    private ModAdapter mAdapter;
    private File mModsDir;

    public BigFishModManagerFragment() {
        super(R.layout.fragment_bigfish_mod_manager);
    }

    @Override
    public void onViewCreated(@NonNull View view, @Nullable Bundle savedInstanceState) {
        mListView = view.findViewById(R.id.bf_mod_list);
        mEmptyView = view.findViewById(R.id.bf_mod_empty);
        View back = view.findViewById(R.id.bf_mod_back);
        View refresh = view.findViewById(R.id.bf_mod_refresh);
        View openDir = view.findViewById(R.id.bf_mod_open_dir);

        if (back != null) back.setOnClickListener(v -> Tools.backToMainMenu(requireActivity()));
        if (refresh != null) refresh.setOnClickListener(v -> reload());
        if (openDir != null) openDir.setOnClickListener(v -> {
            ensureModsDir();
            Tools.openPath(requireContext(), mModsDir, false);
        });

        mModsDir = new File(getSafeGameDir(), "mods");
        mAdapter = new ModAdapter();
        mListView.setAdapter(mAdapter);
        mListView.setOnItemClickListener((parent, v, position, id) -> {
            if (position >= 0 && position < mMods.size()) showActions(mMods.get(position));
        });
        reload();
    }

    @Override
    public void onResume() {
        super.onResume();
        if (mAdapter != null) reload();
    }

    /** 和 MainMenuFragment 一样安全地取当前版本的游戏目录（profile 缺失时退回默认目录） */
    private File getSafeGameDir() {
        try {
            String currentProfile = LauncherPreferences.DEFAULT_PREF.getString(
                    LauncherPreferences.PREF_KEY_CURRENT_PROFILE, null);
            if (currentProfile == null || currentProfile.isEmpty()) return new File(Tools.DIR_GAME_NEW);
            LauncherProfiles.load();
            if (LauncherProfiles.mainProfileJson == null) return new File(Tools.DIR_GAME_NEW);
            MinecraftProfile profile = LauncherProfiles.mainProfileJson.profiles.get(currentProfile);
            if (profile == null) return new File(Tools.DIR_GAME_NEW);
            return Tools.getGameDirPath(profile);
        } catch (Exception e) {
            return new File(Tools.DIR_GAME_NEW);
        }
    }

    private void ensureModsDir() {
        if (mModsDir != null && !mModsDir.exists()) mModsDir.mkdirs();
    }

    private void reload() {
        if (mModsDir == null) return;
        mMods.clear();
        File[] files = mModsDir.listFiles();
        if (files != null) {
            for (File file : files) {
                if (!file.isFile()) continue;
                String name = file.getName().toLowerCase();
                if (name.endsWith(".jar") || name.endsWith(".jar" + SUFFIX_DISABLED)) {
                    mMods.add(file);
                }
            }
        }
        Collections.sort(mMods, (a, b) -> a.getName().compareToIgnoreCase(b.getName()));
        if (mAdapter != null) mAdapter.notifyDataSetChanged();
        if (mEmptyView != null) mEmptyView.setVisibility(mMods.isEmpty() ? View.VISIBLE : View.GONE);
        if (mListView != null) mListView.setVisibility(mMods.isEmpty() ? View.GONE : View.VISIBLE);
    }

    private boolean isDisabled(File file) {
        return file.getName().endsWith(SUFFIX_DISABLED);
    }

    private String displayName(File file) {
        String name = file.getName();
        if (name.endsWith(SUFFIX_DISABLED)) {
            name = name.substring(0, name.length() - SUFFIX_DISABLED.length());
        }
        return name;
    }

    private void showActions(final File file) {
        final boolean disabled = isDisabled(file);
        new AlertDialog.Builder(requireContext())
                .setTitle(displayName(file))
                .setItems(new String[]{
                        disabled ? "启用此模组" : "停用此模组",
                        "删除此模组",
                        "打开模组文件夹"
                }, (dialog, which) -> {
                    if (which == 0) toggleMod(file, disabled);
                    else if (which == 1) confirmDelete(file);
                    else {
                        ensureModsDir();
                        Tools.openPath(requireContext(), mModsDir, false);
                    }
                })
                .setNegativeButton("取消", null)
                .show();
    }

    private void toggleMod(final File file, boolean disabled) {
        String name = file.getName();
        File target = disabled
                ? new File(file.getParentFile(), name.substring(0, name.length() - SUFFIX_DISABLED.length()))
                : new File(file.getParentFile(), name + SUFFIX_DISABLED);
        if (target.exists()) {
            Tools.dialogOnUiThread(requireActivity(), "操作失败", "目标文件已存在：" + target.getName());
            return;
        }
        if (file.renameTo(target)) {
            reload();
        } else {
            Tools.dialogOnUiThread(requireActivity(), "操作失败", "无法重命名文件，请检查存储权限");
        }
    }

    private void confirmDelete(final File file) {
        new AlertDialog.Builder(requireContext())
                .setTitle("删除模组")
                .setMessage("确定要删除 " + displayName(file) + " 吗？此操作不可撤销。")
                .setPositiveButton("删除", (dialog, which) -> {
                    if (file.delete()) {
                        reload();
                    } else {
                        Tools.dialogOnUiThread(requireActivity(), "删除失败", "文件可能被占用或没有权限");
                    }
                })
                .setNegativeButton("取消", null)
                .show();
    }

    private class ModAdapter extends BaseAdapter {
        @Override
        public int getCount() {
            return mMods.size();
        }

        @Override
        public Object getItem(int position) {
            return mMods.get(position);
        }

        @Override
        public long getItemId(int position) {
            return position;
        }

        @Override
        public View getView(int position, View convertView, ViewGroup parent) {
            View view = convertView;
            if (view == null) {
                view = LayoutInflater.from(parent.getContext()).inflate(R.layout.item_bigfish_mod, parent, false);
            }
            File file = mMods.get(position);
            TextView nameView = view.findViewById(R.id.bf_mod_item_name);
            TextView stateView = view.findViewById(R.id.bf_mod_item_state);
            nameView.setText(displayName(file));
            boolean disabled = isDisabled(file);
            stateView.setText(disabled ? "已停用" : "已启用");
            stateView.setTextColor(disabled ? 0xFF8CA3C4 : 0xFF10B981);
            return view;
        }
    }
}
