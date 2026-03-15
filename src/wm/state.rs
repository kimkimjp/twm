use crate::layout::bsp::{BspNode, Direction, Rect};
use crate::windows_api::window;
use windows::Win32::Foundation::HWND;

/// A monitor with a single BSP tree of tiled windows.
pub struct Monitor {
    pub id: isize,
    pub work_area: Rect,
    pub tree: BspNode,
}

impl Monitor {
    fn new(id: isize, work_area: Rect) -> Self {
        Self {
            id,
            work_area,
            tree: BspNode::new(),
        }
    }
}

/// Auto-tiling window manager state. No workspaces, no shortcuts.
/// Just: window appears -> tile it; window disappears -> re-tile.
pub struct WmState {
    pub monitors: Vec<Monitor>,
    pub inner_gap: i32,
    pub outer_gap: i32,
}

impl WmState {
    pub fn new(monitors: Vec<(isize, Rect)>, inner_gap: i32, outer_gap: i32) -> Self {
        let monitors = if monitors.is_empty() {
            vec![Monitor::new(0, Rect { x: 0, y: 0, w: 1920, h: 1080 })]
        } else {
            monitors.into_iter().map(|(id, area)| Monitor::new(id, area)).collect()
        };

        Self { monitors, inner_gap, outer_gap }
    }

    /// Add a window to the monitor identified by monitor_id.
    /// Falls back to first monitor if not found.
    pub fn add_window(&mut self, hwnd: HWND, monitor_id: isize) {
        let mon_idx = self.find_monitor_by_id(monitor_id).unwrap_or(0);
        // Always pass Horizontal as root direction. insert() automatically
        // alternates H/V at each tree level for grid-like layout. (BUG-05)
        self.monitors[mon_idx].tree.insert(hwnd, Direction::Horizontal);
    }

    /// Remove a window from all monitors.
    pub fn remove_window(&mut self, hwnd: HWND) {
        for mon in &mut self.monitors {
            if mon.tree.remove(hwnd) {
                return;
            }
        }
    }

    /// Check if a window is tracked.
    pub fn has_window(&self, hwnd: HWND) -> bool {
        self.monitors.iter().any(|m| m.tree.find_window(hwnd))
    }

    /// Remove invalid/invisible (zombie) HWNDs from all monitor BSP trees.
    /// This prevents stale handles from being sent to SetWindowPos.
    pub fn gc_invalid_windows(&mut self) {
        for mon in &mut self.monitors {
            let windows = mon.tree.get_windows();
            for hwnd in windows {
                if !window::is_window_valid(hwnd) {
                    mon.tree.remove(hwnd);
                }
            }
        }
    }

    /// Apply layout on all monitors.
    /// 1. First purges zombie HWNDs (BUG-01/BUG-08)
    /// 2. Skips minimized windows (BUG-03)
    /// 3. Does NOT call show_window (BUG-16)
    pub fn apply_all_layouts(&mut self) {
        // Purge zombie HWNDs before computing layout
        self.gc_invalid_windows();

        for mon in &self.monitors {
            let effective_area = Rect {
                x: mon.work_area.x + self.outer_gap,
                y: mon.work_area.y + self.outer_gap,
                w: mon.work_area.w - self.outer_gap * 2,
                h: mon.work_area.h - self.outer_gap * 2,
            };

            let placements = mon.tree.calculate_layout(effective_area, self.inner_gap);

            for placement in &placements {
                // Skip minimized windows — don't force-restore them
                if window::is_window_minimized(placement.hwnd) {
                    continue;
                }
                window::set_window_pos(
                    placement.hwnd,
                    placement.rect.x,
                    placement.rect.y,
                    placement.rect.w,
                    placement.rect.h,
                );
            }
        }
    }

    /// Re-enumerate monitors and reassign windows to correct monitors.
    pub fn refresh_monitors(&mut self, new_monitors: Vec<(isize, Rect)>) {
        // Collect all tracked windows
        let all_windows: Vec<HWND> = self.monitors.iter()
            .flat_map(|m| m.tree.get_windows())
            .collect();

        // Rebuild monitor list
        self.monitors = if new_monitors.is_empty() {
            vec![Monitor::new(0, Rect { x: 0, y: 0, w: 1920, h: 1080 })]
        } else {
            new_monitors.into_iter().map(|(id, area)| Monitor::new(id, area)).collect()
        };

        // Re-assign windows to their correct monitors
        for hwnd in all_windows {
            if window::is_window_valid(hwnd) {
                let mon_id = crate::windows_api::monitor::monitor_from_window(hwnd);
                self.add_window(hwnd, mon_id);
            }
        }
    }

    /// Move a window to its correct monitor after drag-move (BUG-13).
    pub fn move_window_to_correct_monitor(&mut self, hwnd: HWND) {
        let new_mon_id = crate::windows_api::monitor::monitor_from_window(hwnd);
        let new_mon_idx = self.find_monitor_by_id(new_mon_id).unwrap_or(0);

        // Find which monitor currently tracks this window
        let current_mon_idx = self.monitors.iter().position(|m| m.tree.find_window(hwnd));

        if let Some(current) = current_mon_idx {
            if current != new_mon_idx {
                self.monitors[current].tree.remove(hwnd);
                self.monitors[new_mon_idx].tree.insert(hwnd, Direction::Horizontal);
            }
        }
    }

    fn find_monitor_by_id(&self, id: isize) -> Option<usize> {
        self.monitors.iter().position(|m| m.id == id)
    }
}
