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

    /// ウィンドウを挿入する。重複は無視する。(BUG-07)
    /// - Empty なら Leaf に変換
    /// - Leaf なら Split に変換して既存ウィンドウと新ウィンドウを配置
    /// - Split なら少ない方のサブツリーに挿入（バランスド）
    pub fn insert(&mut self, hwnd: HWND, direction: Direction) -> &mut Self {
        // Prevent duplicate insertion (BUG-07)
        if self.find_window(hwnd) {
            return self;
        }
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
                let remainder = gap - half_gap * 2; // handle odd gap values
                let (first_area, second_area) = match direction {
                    Direction::Horizontal => {
                        let split_w = ((area.w as f32) * ratio) as i32;
                        let first_rect = Rect {
                            x: area.x,
                            y: area.y,
                            w: (split_w - half_gap).max(1),
                            h: area.h.max(1),
                        };
                        let second_rect = Rect {
                            x: area.x + split_w + half_gap + remainder,
                            y: area.y,
                            w: (area.w - split_w - half_gap - remainder).max(1),
                            h: area.h.max(1),
                        };
                        (first_rect, second_rect)
                    }
                    Direction::Vertical => {
                        let split_h = ((area.h as f32) * ratio) as i32;
                        let first_rect = Rect {
                            x: area.x,
                            y: area.y,
                            w: area.w.max(1),
                            h: (split_h - half_gap).max(1),
                        };
                        let second_rect = Rect {
                            x: area.x,
                            y: area.y + split_h + half_gap + remainder,
                            w: area.w.max(1),
                            h: (area.h - split_h - half_gap - remainder).max(1),
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
