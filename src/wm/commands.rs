#[derive(Debug, Clone)]
pub enum WmCommand {
    FocusLeft,
    FocusDown,
    FocusUp,
    FocusRight,
    MoveLeft,
    MoveDown,
    MoveUp,
    MoveRight,
    Workspace(usize),
    MoveToWorkspace(usize),
    Close,
    ToggleFullscreen,
    ToggleSplitDirection,
    FocusMonitorNext,
    FocusMonitorPrev,
    MoveToMonitorNext,
    MoveToMonitorPrev,
    Exec(String),
    ReloadConfig,
    Exit,
}

pub fn parse_command(s: &str) -> Option<WmCommand> {
    let s = s.trim();

    match s {
        "focus left" => Some(WmCommand::FocusLeft),
        "focus right" => Some(WmCommand::FocusRight),
        "focus up" => Some(WmCommand::FocusUp),
        "focus down" => Some(WmCommand::FocusDown),
        "move left" => Some(WmCommand::MoveLeft),
        "move right" => Some(WmCommand::MoveRight),
        "move up" => Some(WmCommand::MoveUp),
        "move down" => Some(WmCommand::MoveDown),
        "close" => Some(WmCommand::Close),
        "fullscreen" => Some(WmCommand::ToggleFullscreen),
        "split_toggle" => Some(WmCommand::ToggleSplitDirection),
        "reload" => Some(WmCommand::ReloadConfig),
        "exit" => Some(WmCommand::Exit),
        "focus_monitor next" => Some(WmCommand::FocusMonitorNext),
        "focus_monitor prev" => Some(WmCommand::FocusMonitorPrev),
        "move_to_monitor next" => Some(WmCommand::MoveToMonitorNext),
        "move_to_monitor prev" => Some(WmCommand::MoveToMonitorPrev),
        _ => {
            // workspace 1~9
            if let Some(num_str) = s.strip_prefix("workspace ") {
                if let Ok(num) = num_str.trim().parse::<usize>() {
                    if (1..=9).contains(&num) {
                        return Some(WmCommand::Workspace(num - 1));
                    }
                }
                return None;
            }

            // move_to_workspace 1~9
            if let Some(num_str) = s.strip_prefix("move_to_workspace ") {
                if let Ok(num) = num_str.trim().parse::<usize>() {
                    if (1..=9).contains(&num) {
                        return Some(WmCommand::MoveToWorkspace(num - 1));
                    }
                }
                return None;
            }

            // exec ...
            if let Some(cmd) = s.strip_prefix("exec ") {
                let cmd = cmd.trim();
                if !cmd.is_empty() {
                    return Some(WmCommand::Exec(cmd.to_string()));
                }
                return None;
            }

            None
        }
    }
}
