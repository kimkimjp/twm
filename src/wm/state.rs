use crate::layout::bsp::{BspNode, Direction, Rect};
use crate::windows_api::window;
use windows::Win32::Foundation::HWND;

pub struct Workspace {
    pub tree: BspNode,
    pub next_direction: Direction,
}

impl Workspace {
    fn new() -> Self {
        Self {
            tree: BspNode::new(),
            next_direction: Direction::Horizontal,
        }
    }
}

pub struct Monitor {
    pub id: isize,
    pub work_area: Rect,
    pub workspaces: Vec<Workspace>,
    pub active_workspace: usize,
    pub fullscreen_hwnd: Option<HWND>,
}

impl Monitor {
    fn new(id: isize, work_area: Rect) -> Self {
        let workspaces = (0..9).map(|_| Workspace::new()).collect();
        Self {
            id,
            work_area,
            workspaces,
            active_workspace: 0,
            fullscreen_hwnd: None,
        }
    }

    fn current_workspace(&self) -> &Workspace {
        &self.workspaces[self.active_workspace]
    }

    fn current_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace]
    }

    fn find_window(&self, hwnd: HWND) -> bool {
        self.workspaces.iter().any(|ws| ws.tree.find_window(hwnd))
    }
}

pub struct WmState {
    pub monitors: Vec<Monitor>,
    pub active_monitor: usize,
    pub inner_gap: i32,
    pub outer_gap: i32,
    pub focused_window: Option<HWND>,
}

impl WmState {
    /// monitors は (id, work_area) のタプルリスト。空の場合は 1920x1080 のダミーモニターを作る。
    pub fn new(monitors: Vec<(isize, Rect)>, inner_gap: i32, outer_gap: i32) -> Self {
        let monitors = if monitors.is_empty() {
            let fallback_area = Rect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080,
            };
            vec![Monitor::new(0, fallback_area)]
        } else {
            monitors
                .into_iter()
                .map(|(id, area)| Monitor::new(id, area))
                .collect()
        };

        Self {
            monitors,
            active_monitor: 0,
            inner_gap,
            outer_gap,
            focused_window: None,
        }
    }

    pub fn current_monitor(&self) -> &Monitor {
        &self.monitors[self.active_monitor]
    }

    pub fn current_monitor_mut(&mut self) -> &mut Monitor {
        &mut self.monitors[self.active_monitor]
    }

    pub fn current_workspace(&self) -> &Workspace {
        self.current_monitor().current_workspace()
    }

    pub fn current_workspace_mut(&mut self) -> &mut Workspace {
        self.current_monitor_mut().current_workspace_mut()
    }

    /// 現在のモニターの現在のワークスペースにウィンドウを追加
    pub fn add_window(&mut self, hwnd: HWND) {
        let mon = self.current_monitor_mut();
        let dir = mon.current_workspace().next_direction;
        mon.current_workspace_mut().tree.insert(hwnd, dir);
        self.focused_window = Some(hwnd);
    }

    /// 指定モニターのアクティブワークスペースにウィンドウを追加。
    /// monitor_id が見つからなければ active_monitor に追加。
    pub fn add_window_to_monitor(&mut self, hwnd: HWND, monitor_id: isize) {
        let mon_idx = self
            .find_monitor_by_id(monitor_id)
            .unwrap_or(self.active_monitor);

        let mon = &mut self.monitors[mon_idx];
        let dir = mon.current_workspace().next_direction;
        mon.current_workspace_mut().tree.insert(hwnd, dir);
        self.focused_window = Some(hwnd);
    }

    /// 全モニター全ワークスペースからウィンドウを削除
    pub fn remove_window(&mut self, hwnd: HWND) {
        for mon in &mut self.monitors {
            for ws in &mut mon.workspaces {
                if ws.tree.remove(hwnd) {
                    // Clear fullscreen if the fullscreen window was removed
                    if let Some(fs) = mon.fullscreen_hwnd {
                        if fs.0 == hwnd.0 {
                            mon.fullscreen_hwnd = None;
                        }
                    }
                    // Update focused window
                    if let Some(focused) = self.focused_window {
                        if focused.0 == hwnd.0 {
                            let windows = self
                                .monitors[self.active_monitor]
                                .current_workspace()
                                .tree
                                .get_windows();
                            self.focused_window = windows.first().copied();
                        }
                    }
                    return;
                }
            }
        }
    }

    /// 現在のモニターの現在のワークスペースのレイアウトのみ適用
    pub fn apply_layout(&self) {
        self.apply_monitor_layout(self.active_monitor);
    }

    /// 全モニターの現在のワークスペースのレイアウトを適用
    pub fn apply_all_layouts(&self) {
        for i in 0..self.monitors.len() {
            self.apply_monitor_layout(i);
        }
    }

    /// 指定モニターの現在のワークスペースのレイアウトを適用
    fn apply_monitor_layout(&self, monitor_idx: usize) {
        let mon = &self.monitors[monitor_idx];
        let ws = mon.current_workspace();

        // If a window is in fullscreen mode, only show that window
        if let Some(fs_hwnd) = mon.fullscreen_hwnd {
            if ws.tree.find_window(fs_hwnd) {
                window::set_window_pos(
                    fs_hwnd,
                    mon.work_area.x,
                    mon.work_area.y,
                    mon.work_area.w,
                    mon.work_area.h,
                );
                for w in ws.tree.get_windows() {
                    if w.0 != fs_hwnd.0 {
                        window::show_window(w, false);
                    }
                }
                window::show_window(fs_hwnd, true);
                return;
            }
        }

        // Calculate effective area with outer gaps
        let effective_area = Rect {
            x: mon.work_area.x + self.outer_gap,
            y: mon.work_area.y + self.outer_gap,
            w: mon.work_area.w - self.outer_gap * 2,
            h: mon.work_area.h - self.outer_gap * 2,
        };

        let placements = ws.tree.calculate_layout(effective_area, self.inner_gap);

        for placement in &placements {
            window::set_window_pos(
                placement.hwnd,
                placement.rect.x,
                placement.rect.y,
                placement.rect.w,
                placement.rect.h,
            );
            window::show_window(placement.hwnd, true);
        }
    }

    /// 現在のモニターでワークスペース切替
    pub fn switch_workspace(&mut self, idx: usize) {
        let mon = self.current_monitor_mut();
        if idx >= mon.workspaces.len() || idx == mon.active_workspace {
            return;
        }

        // Hide windows in current workspace
        let current_windows = mon.current_workspace().tree.get_windows();
        for w in &current_windows {
            window::show_window(*w, false);
        }

        // Switch active workspace
        mon.active_workspace = idx;

        // Show windows in new workspace
        let new_windows = mon.current_workspace().tree.get_windows();
        for w in &new_windows {
            window::show_window(*w, true);
        }

        // Update focused window
        self.focused_window = new_windows.first().copied();
        if let Some(focused) = self.focused_window {
            window::set_foreground(focused);
        }

        // Clear fullscreen state when switching workspaces
        self.current_monitor_mut().fullscreen_hwnd = None;

        self.apply_layout();
    }

    /// 現在のモニターの指定ワークスペースにウィンドウを移動
    pub fn move_window_to_workspace(&mut self, hwnd: HWND, idx: usize) {
        let mon = self.current_monitor_mut();
        if idx >= mon.workspaces.len() || idx == mon.active_workspace {
            return;
        }

        let source_ws = mon.active_workspace;
        if !mon.workspaces[source_ws].tree.remove(hwnd) {
            return;
        }

        // Add to target workspace
        let dir = mon.workspaces[idx].next_direction;
        mon.workspaces[idx].tree.insert(hwnd, dir);

        // Hide the moved window (it's now on a different workspace)
        window::show_window(hwnd, false);

        // Update focused window in current workspace
        if let Some(focused) = self.focused_window {
            if focused.0 == hwnd.0 {
                let windows = self.current_workspace().tree.get_windows();
                self.focused_window = windows.first().copied();
                if let Some(new_focused) = self.focused_window {
                    window::set_foreground(new_focused);
                }
            }
        }

        self.apply_layout();
    }

    /// 指定方向の隣接ウィンドウにフォーカス
    pub fn focus_direction(&mut self, dir: Direction, forward: bool) {
        let focused = match self.focused_window {
            Some(hwnd) => hwnd,
            None => return,
        };

        if let Some(adjacent) = self.current_workspace().tree.get_adjacent(focused, dir, forward) {
            self.focused_window = Some(adjacent);
            window::set_foreground(adjacent);
        }
    }

    /// フォーカス中のウィンドウを指定方向に移動（swap）
    pub fn move_direction(&mut self, dir: Direction, forward: bool) {
        let focused = match self.focused_window {
            Some(hwnd) => hwnd,
            None => return,
        };

        let adjacent = self.current_workspace().tree.get_adjacent(focused, dir, forward);
        if let Some(target) = adjacent {
            self.current_workspace_mut()
                .tree
                .swap_windows(focused, target);
            self.apply_layout();
        }
    }

    /// フォーカス中のウィンドウのフルスクリーン切替（現在のモニター）
    pub fn toggle_fullscreen(&mut self) {
        let focused = match self.focused_window {
            Some(hwnd) => hwnd,
            None => return,
        };

        let mon = self.current_monitor_mut();
        if let Some(fs) = mon.fullscreen_hwnd {
            if fs.0 == focused.0 {
                // Exit fullscreen
                mon.fullscreen_hwnd = None;
                // Show all windows again
                for w in mon.current_workspace().tree.get_windows() {
                    window::show_window(w, true);
                }
                self.apply_layout();
                return;
            }
        }

        // Enter fullscreen
        self.current_monitor_mut().fullscreen_hwnd = Some(focused);
        self.apply_layout();
    }

    /// ウィンドウがどのモニター・ワークスペースにあるかを返す: (monitor_index, workspace_index)
    pub fn find_workspace_for_window(&self, hwnd: HWND) -> Option<(usize, usize)> {
        for (mi, mon) in self.monitors.iter().enumerate() {
            for (wi, ws) in mon.workspaces.iter().enumerate() {
                if ws.tree.find_window(hwnd) {
                    return Some((mi, wi));
                }
            }
        }
        None
    }

    /// 次のモニターにフォーカス移動（ラウンドロビン）
    pub fn focus_monitor_next(&mut self) {
        if self.monitors.len() <= 1 {
            return;
        }
        self.active_monitor = (self.active_monitor + 1) % self.monitors.len();

        // Focus the first window on the new monitor's active workspace
        let windows = self.current_workspace().tree.get_windows();
        self.focused_window = windows.first().copied();
        if let Some(focused) = self.focused_window {
            window::set_foreground(focused);
        }
    }

    /// 前のモニターにフォーカス移動（ラウンドロビン）
    pub fn focus_monitor_prev(&mut self) {
        if self.monitors.len() <= 1 {
            return;
        }
        self.active_monitor = if self.active_monitor == 0 {
            self.monitors.len() - 1
        } else {
            self.active_monitor - 1
        };

        // Focus the first window on the new monitor's active workspace
        let windows = self.current_workspace().tree.get_windows();
        self.focused_window = windows.first().copied();
        if let Some(focused) = self.focused_window {
            window::set_foreground(focused);
        }
    }

    /// フォーカス中ウィンドウを次のモニターに移動
    pub fn move_to_monitor_next(&mut self) {
        self.move_to_monitor_offset(1);
    }

    /// フォーカス中ウィンドウを前のモニターに移動
    pub fn move_to_monitor_prev(&mut self) {
        self.move_to_monitor_offset(-1);
    }

    /// フォーカス中ウィンドウを offset 分ずれたモニターに移動
    fn move_to_monitor_offset(&mut self, offset: isize) {
        if self.monitors.len() <= 1 {
            return;
        }

        let focused = match self.focused_window {
            Some(hwnd) => hwnd,
            None => return,
        };

        let src_mon = self.active_monitor;
        let dst_mon = {
            let len = self.monitors.len() as isize;
            ((src_mon as isize + offset).rem_euclid(len)) as usize
        };

        if src_mon == dst_mon {
            return;
        }

        // Remove from source monitor's active workspace
        let src_ws = self.monitors[src_mon].active_workspace;
        if !self.monitors[src_mon].workspaces[src_ws].tree.remove(focused) {
            return;
        }

        // Insert into destination monitor's active workspace
        let dst_ws = self.monitors[dst_mon].active_workspace;
        let dir = self.monitors[dst_mon].workspaces[dst_ws].next_direction;
        self.monitors[dst_mon].workspaces[dst_ws]
            .tree
            .insert(focused, dir);

        // Move active monitor to destination
        self.active_monitor = dst_mon;

        // Apply layout on both monitors
        self.apply_monitor_layout(src_mon);
        self.apply_monitor_layout(dst_mon);

        // Focus the moved window
        self.focused_window = Some(focused);
        window::set_foreground(focused);
    }

    /// monitor_id からモニターインデックスを検索
    pub fn find_monitor_by_id(&self, id: isize) -> Option<usize> {
        self.monitors.iter().position(|m| m.id == id)
    }
}
