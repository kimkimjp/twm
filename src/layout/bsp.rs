use windows::Win32::Foundation::HWND;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Debug)]
pub enum BspNode {
    Split {
        direction: Direction,
        ratio: f32,
        first: Box<BspNode>,
        second: Box<BspNode>,
    },
    Leaf {
        hwnd: HWND,
    },
    Empty,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// ウィンドウの配置結果
#[derive(Debug)]
pub struct WindowPlacement {
    pub hwnd: HWND,
    pub rect: Rect,
}

impl BspNode {
    /// 空のノードを返す
    pub fn new() -> Self {
        BspNode::Empty
    }

    /// ウィンドウを挿入する。
    /// - Empty なら Leaf に変換
    /// - Leaf なら Split に変換して既存ウィンドウと新ウィンドウを配置
    /// - Split なら少ない方のサブツリーに挿入（バランスド）
    pub fn insert(&mut self, hwnd: HWND, direction: Direction) -> &mut Self {
        match self {
            BspNode::Empty => {
                *self = BspNode::Leaf { hwnd };
            }
            BspNode::Leaf { hwnd: existing } => {
                let existing_hwnd = *existing;
                *self = BspNode::Split {
                    direction,
                    ratio: 0.5,
                    first: Box::new(BspNode::Leaf {
                        hwnd: existing_hwnd,
                    }),
                    second: Box::new(BspNode::Leaf { hwnd }),
                };
            }
            BspNode::Split {
                first,
                second,
                direction: split_dir,
                ..
            } => {
                // Insert into the subtree with fewer windows for balanced layout.
                // This ensures 3 windows get ~33% each instead of 50/25/25.
                // Alternate direction at each level for grid-like layout.
                let next_dir = match *split_dir {
                    Direction::Horizontal => Direction::Vertical,
                    Direction::Vertical => Direction::Horizontal,
                };
                if first.window_count() <= second.window_count() {
                    first.insert(hwnd, next_dir);
                } else {
                    second.insert(hwnd, next_dir);
                }
            }
        }
        self
    }

    /// ウィンドウを削除する。
    /// Leaf が削除されたら、兄弟ノードが親の位置に昇格する。
    /// 削除に成功したら true を返す。
    pub fn remove(&mut self, hwnd: HWND) -> bool {
        match self {
            BspNode::Empty => false,
            BspNode::Leaf { hwnd: leaf_hwnd } => {
                if leaf_hwnd.0 == hwnd.0 {
                    *self = BspNode::Empty;
                    true
                } else {
                    false
                }
            }
            BspNode::Split { first, second, .. } => {
                // Try removing from first child
                if first.remove(hwnd) {
                    // first became Empty, promote second
                    if matches!(**first, BspNode::Empty) {
                        let promoted = std::mem::replace(second.as_mut(), BspNode::Empty);
                        *self = promoted;
                    }
                    return true;
                }
                // Try removing from second child
                if second.remove(hwnd) {
                    // second became Empty, promote first
                    if matches!(**second, BspNode::Empty) {
                        let promoted = std::mem::replace(first.as_mut(), BspNode::Empty);
                        *self = promoted;
                    }
                    return true;
                }
                false
            }
        }
    }

    /// 再帰的に矩形を分割して各ウィンドウの位置を計算する。
    /// gap はウィンドウ間の隙間（inner gap）。
    pub fn calculate_layout(&self, area: Rect, gap: i32) -> Vec<WindowPlacement> {
        match self {
            BspNode::Empty => Vec::new(),
            BspNode::Leaf { hwnd } => {
                vec![WindowPlacement {
                    hwnd: *hwnd,
                    rect: area,
                }]
            }
            BspNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let half_gap = gap / 2;
                let (first_area, second_area) = match direction {
                    Direction::Horizontal => {
                        let split_w = ((area.w as f32) * ratio) as i32;
                        let first_rect = Rect {
                            x: area.x,
                            y: area.y,
                            w: split_w - half_gap,
                            h: area.h,
                        };
                        let second_rect = Rect {
                            x: area.x + split_w + half_gap,
                            y: area.y,
                            w: area.w - split_w - half_gap,
                            h: area.h,
                        };
                        (first_rect, second_rect)
                    }
                    Direction::Vertical => {
                        let split_h = ((area.h as f32) * ratio) as i32;
                        let first_rect = Rect {
                            x: area.x,
                            y: area.y,
                            w: area.w,
                            h: split_h - half_gap,
                        };
                        let second_rect = Rect {
                            x: area.x,
                            y: area.y + split_h + half_gap,
                            w: area.w,
                            h: area.h - split_h - half_gap,
                        };
                        (first_rect, second_rect)
                    }
                };

                let mut placements = first.calculate_layout(first_area, gap);
                placements.extend(second.calculate_layout(second_area, gap));
                placements
            }
        }
    }

    /// ウィンドウがツリー内に存在するかを返す
    pub fn find_window(&self, hwnd: HWND) -> bool {
        match self {
            BspNode::Empty => false,
            BspNode::Leaf { hwnd: leaf_hwnd } => leaf_hwnd.0 == hwnd.0,
            BspNode::Split { first, second, .. } => {
                first.find_window(hwnd) || second.find_window(hwnd)
            }
        }
    }

    /// ツリー内の全ウィンドウを返す
    pub fn get_windows(&self) -> Vec<HWND> {
        match self {
            BspNode::Empty => Vec::new(),
            BspNode::Leaf { hwnd } => vec![*hwnd],
            BspNode::Split { first, second, .. } => {
                let mut windows = first.get_windows();
                windows.extend(second.get_windows());
                windows
            }
        }
    }

    /// 指定方向の隣接ウィンドウを返す（フォーカス移動用）。
    /// forward=true は右/下、false は左/上。
    pub fn get_adjacent(&self, hwnd: HWND, dir: Direction, forward: bool) -> Option<HWND> {
        let windows = self.get_windows();
        if windows.is_empty() {
            return None;
        }

        // Find the current window's index
        let _pos = windows.iter().position(|w| w.0 == hwnd.0)?;

        // For BSP trees, windows are laid out in order.
        // Horizontal splits: left-to-right in windows list
        // Vertical splits: top-to-bottom in windows list
        // Use the flat list for adjacency based on direction.
        // We need to use the layout to determine spatial adjacency.
        // For simplicity and correctness, compute a dummy layout and find spatial neighbors.
        let dummy_area = Rect {
            x: 0,
            y: 0,
            w: 10000,
            h: 10000,
        };
        let placements = self.calculate_layout(dummy_area, 0);

        // Find current window placement
        let current = placements.iter().find(|p| p.hwnd.0 == hwnd.0)?;
        let cx = current.rect.x + current.rect.w / 2;
        let cy = current.rect.y + current.rect.h / 2;

        // Find the nearest window in the specified direction
        let mut best: Option<(HWND, i32)> = None;

        for p in &placements {
            if p.hwnd.0 == hwnd.0 {
                continue;
            }
            let px = p.rect.x + p.rect.w / 2;
            let py = p.rect.y + p.rect.h / 2;

            let is_valid = match (dir, forward) {
                (Direction::Horizontal, true) => px > cx,  // right
                (Direction::Horizontal, false) => px < cx, // left
                (Direction::Vertical, true) => py > cy,    // down
                (Direction::Vertical, false) => py < cy,   // up
            };

            if !is_valid {
                continue;
            }

            let dist = (px - cx).abs() + (py - cy).abs();
            match best {
                None => best = Some((p.hwnd, dist)),
                Some((_, best_dist)) if dist < best_dist => {
                    best = Some((p.hwnd, dist));
                }
                _ => {}
            }
        }

        best.map(|(hwnd, _)| hwnd)
    }

    /// 2つのウィンドウの位置を交換（ウィンドウ移動用）
    pub fn swap_windows(&mut self, a: HWND, b: HWND) {
        // Collect mutable pointers to the HWND values we need to swap
        let mut found_a: Option<*mut HWND> = None;
        let mut found_b: Option<*mut HWND> = None;
        Self::find_hwnd_ptrs(self, a, b, &mut found_a, &mut found_b);

        if let (Some(pa), Some(pb)) = (found_a, found_b) {
            unsafe {
                std::ptr::swap(pa, pb);
            }
        }
    }

    /// Helper to find mutable pointers to two HWNDs in the tree
    fn find_hwnd_ptrs(
        node: &mut BspNode,
        a: HWND,
        b: HWND,
        found_a: &mut Option<*mut HWND>,
        found_b: &mut Option<*mut HWND>,
    ) {
        match node {
            BspNode::Leaf { hwnd } => {
                if hwnd.0 == a.0 {
                    *found_a = Some(hwnd as *mut HWND);
                } else if hwnd.0 == b.0 {
                    *found_b = Some(hwnd as *mut HWND);
                }
            }
            BspNode::Split { first, second, .. } => {
                Self::find_hwnd_ptrs(first, a, b, found_a, found_b);
                Self::find_hwnd_ptrs(second, a, b, found_a, found_b);
            }
            BspNode::Empty => {}
        }
    }

    /// ツリー内のウィンドウ数
    pub fn window_count(&self) -> usize {
        match self {
            BspNode::Empty => 0,
            BspNode::Leaf { .. } => 1,
            BspNode::Split { first, second, .. } => {
                first.window_count() + second.window_count()
            }
        }
    }
}

impl Default for BspNode {
    fn default() -> Self {
        Self::new()
    }
}
