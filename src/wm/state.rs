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

pub struct WmState {
    pub workspaces: Vec<Workspace>,
    pub active_workspace: usize,
    pub work_area: Rect,
    pub inner_gap: i32,
    pub outer_gap: i32,
    pub focused_window: Option<HWND>,
    fullscreen_hwnd: Option<HWND>,
}

impl WmState {
    /// 9個のワークスペースを初期化
    pub fn new(work_area: Rect, inner_gap: i32, outer_gap: i32) -> Self {
        let workspaces = (0..9).map(|_| Workspace::new()).collect();
        Self {
            workspaces,
            active_workspace: 0,
            work_area,
            inner_gap,
            outer_gap,
            focused_window: None,
            fullscreen_hwnd: None,
        }
    }

    pub fn current_workspace(&self) -> &Workspace {
        &self.workspaces[self.active_workspace]
    }

    pub fn current_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace]
    }

    /// 現在のワークスペースにウィンドウを追加
    pub fn add_window(&mut self, hwnd: HWND) {
        let dir = self.workspaces[self.active_workspace].next_direction;
        self.workspaces[self.active_workspace].tree.insert(hwnd, dir);
        self.focused_window = Some(hwnd);
    }

    /// 全ワークスペースからウィンドウを削除
    pub fn remove_window(&mut self, hwnd: HWND) {
        for ws in &mut self.workspaces {
            if ws.tree.remove(hwnd) {
                break;
            }
        }

        // If the removed window was focused, clear or update focus
        if let Some(focused) = self.focused_window {
            if focused.0 == hwnd.0 {
                // Try to focus another window in the current workspace
                let windows = self.current_workspace().tree.get_windows();
                self.focused_window = windows.first().copied();
            }
        }

        // Clear fullscreen if the fullscreen window was removed
        if let Some(fs) = self.fullscreen_hwnd {
            if fs.0 == hwnd.0 {
                self.fullscreen_hwnd = None;
            }
        }
    }

    /// 現在のワークスペースのレイアウトを計算し、各ウィンドウを配置
    pub fn apply_layout(&self) {
        // If a window is in fullscreen mode, only show that window
        if let Some(fs_hwnd) = self.fullscreen_hwnd {
            if self.current_workspace().tree.find_window(fs_hwnd) {
                window::set_window_pos(
                    fs_hwnd,
                    self.work_area.x,
                    self.work_area.y,
                    self.work_area.w,
                    self.work_area.h,
                );
                // Hide other windows in the workspace
                for w in self.current_workspace().tree.get_windows() {
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
            x: self.work_area.x + self.outer_gap,
            y: self.work_area.y + self.outer_gap,
            w: self.work_area.w - self.outer_gap * 2,
            h: self.work_area.h - self.outer_gap * 2,
        };

        let placements = self
            .current_workspace()
            .tree
            .calculate_layout(effective_area, self.inner_gap);

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

    /// ワークスペース切替。旧ワークスペースのウィンドウを非表示、新ワークスペースのウィンドウを表示。
    pub fn switch_workspace(&mut self, idx: usize) {
        if idx >= self.workspaces.len() || idx == self.active_workspace {
            return;
        }

        // Hide windows in current workspace
        let current_windows = self.workspaces[self.active_workspace].tree.get_windows();
        for w in &current_windows {
            window::show_window(*w, false);
        }

        // Switch active workspace
        self.active_workspace = idx;

        // Show windows in new workspace
        let new_windows = self.workspaces[self.active_workspace].tree.get_windows();
        for w in &new_windows {
            window::show_window(*w, true);
        }

        // Update focused window
        self.focused_window = new_windows.first().copied();
        if let Some(focused) = self.focused_window {
            window::set_foreground(focused);
        }

        // Clear fullscreen state when switching workspaces
        self.fullscreen_hwnd = None;

        self.apply_layout();
    }

    /// ウィンドウを別ワークスペースに移動
    pub fn move_window_to_workspace(&mut self, hwnd: HWND, idx: usize) {
        if idx >= self.workspaces.len() || idx == self.active_workspace {
            return;
        }

        // Remove from current workspace
        let source_ws = self.active_workspace;
        if !self.workspaces[source_ws].tree.remove(hwnd) {
            return;
        }

        // Add to target workspace
        let dir = self.workspaces[idx].next_direction;
        self.workspaces[idx].tree.insert(hwnd, dir);

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

    /// フォーカス中のウィンドウのフルスクリーン切替
    pub fn toggle_fullscreen(&mut self) {
        let focused = match self.focused_window {
            Some(hwnd) => hwnd,
            None => return,
        };

        if let Some(fs) = self.fullscreen_hwnd {
            if fs.0 == focused.0 {
                // Exit fullscreen
                self.fullscreen_hwnd = None;
                // Show all windows again
                for w in self.current_workspace().tree.get_windows() {
                    window::show_window(w, true);
                }
                self.apply_layout();
                return;
            }
        }

        // Enter fullscreen
        self.fullscreen_hwnd = Some(focused);
        self.apply_layout();
    }

    /// ウィンドウがどのワークスペースにあるかを返す
    pub fn find_workspace_for_window(&self, hwnd: HWND) -> Option<usize> {
        for (i, ws) in self.workspaces.iter().enumerate() {
            if ws.tree.find_window(hwnd) {
                return Some(i);
            }
        }
        None
    }
}
