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
/// Just: window appears → tile it; window disappears → re-tile.
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
        // Alternate split direction based on window count for balanced layout
        let count = self.monitors[mon_idx].tree.window_count();
        let dir = if count % 2 == 0 { Direction::Horizontal } else { Direction::Vertical };
        self.monitors[mon_idx].tree.insert(hwnd, dir);
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

    /// Apply layout on all monitors.
    pub fn apply_all_layouts(&self) {
        for mon in &self.monitors {
            let effective_area = Rect {
                x: mon.work_area.x + self.outer_gap,
                y: mon.work_area.y + self.outer_gap,
                w: mon.work_area.w - self.outer_gap * 2,
                h: mon.work_area.h - self.outer_gap * 2,
            };

            let placements = mon.tree.calculate_layout(effective_area, self.inner_gap);

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
    }

    fn find_monitor_by_id(&self, id: isize) -> Option<usize> {
        self.monitors.iter().position(|m| m.id == id)
    }
}
