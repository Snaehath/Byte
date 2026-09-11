use serde::{Deserialize, Serialize};
use tauri::{PhysicalPosition, Position, WebviewWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Anchor {
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
    Taskbar,
}

impl Default for Anchor {
    fn default() -> Self {
        Self::BottomRight
    }
}

#[derive(Debug, Clone)]
pub struct MonitorGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

#[derive(Debug, Clone)]
pub struct WindowGeometry {
    pub width: u32,
    pub height: u32,
}

pub struct PlacementEngine;

impl PlacementEngine {
    /// Pure coordinate calculation function for deterministic positioning
    pub fn calculate_position(
        monitor: &MonitorGeometry,
        window: &WindowGeometry,
        anchor: Anchor,
        margin_x: i32,
        margin_y: i32,
    ) -> (i32, i32) {
        match anchor {
            Anchor::BottomRight | Anchor::Taskbar => {
                let x = monitor.x + (monitor.width as i32) - (window.width as i32) - margin_x;
                let y = monitor.y + (monitor.height as i32) - (window.height as i32) - margin_y;
                (x, y)
            }
            Anchor::BottomLeft => {
                let x = monitor.x + margin_x;
                let y = monitor.y + (monitor.height as i32) - (window.height as i32) - margin_y;
                (x, y)
            }
            Anchor::TopRight => {
                let x = monitor.x + (monitor.width as i32) - (window.width as i32) - margin_x;
                let y = monitor.y + margin_y;
                (x, y)
            }
            Anchor::TopLeft => {
                let x = monitor.x + margin_x;
                let y = monitor.y + margin_y;
                (x, y)
            }
        }
    }

    /// Position a Tauri WebviewWindow relative to the active or primary monitor
    pub fn position_window(window: &WebviewWindow, anchor: Anchor) -> Result<(), String> {
        let monitor = window
            .current_monitor()
            .map_err(|e| format!("Failed to get current monitor: {}", e))?
            .or_else(|| window.primary_monitor().ok().flatten())
            .ok_or_else(|| "No active monitor found".to_string())?;

        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        let scale = monitor.scale_factor();

        let win_size = window
            .outer_size()
            .map_err(|e| format!("Failed to get window size: {}", e))?;

        let monitor_geo = MonitorGeometry {
            x: mon_pos.x,
            y: mon_pos.y,
            width: mon_size.width,
            height: mon_size.height,
            scale_factor: scale,
        };

        let window_geo = WindowGeometry {
            width: win_size.width,
            height: win_size.height,
        };

        // 32px from right, 64px from bottom to clear standard Windows taskbar (48-60px)
        let margin_x = (28.0 * scale) as i32;
        let margin_y = (64.0 * scale) as i32;

        let (target_x, target_y) = Self::calculate_position(&monitor_geo, &window_geo, anchor, margin_x, margin_y);

        log::info!(
            "PlacementEngine: Positioning window to ({}, {}) on monitor [{}x{} at ({},{})]",
            target_x, target_y, mon_size.width, mon_size.height, mon_pos.x, mon_pos.y
        );

        window
            .set_position(Position::Physical(PhysicalPosition::new(target_x, target_y)))
            .map_err(|e| format!("Failed to set window position: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bottom_right_placement() {
        let monitor = MonitorGeometry {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
        };
        let window = WindowGeometry {
            width: 220,
            height: 220,
        };

        let (x, y) = PlacementEngine::calculate_position(&monitor, &window, Anchor::BottomRight, 24, 60);
        // 1920 - 220 - 24 = 1676
        // 1080 - 220 - 60 = 800
        assert_eq!(x, 1676);
        assert_eq!(y, 800);
    }

    #[test]
    fn test_multi_monitor_offset() {
        // Second monitor positioned to the right at x=1920
        let monitor = MonitorGeometry {
            x: 1920,
            y: 0,
            width: 2560,
            height: 1440,
            scale_factor: 1.0,
        };
        let window = WindowGeometry {
            width: 220,
            height: 220,
        };

        let (x, y) = PlacementEngine::calculate_position(&monitor, &window, Anchor::BottomRight, 30, 70);
        // 1920 + 2560 - 220 - 30 = 4230
        // 0 + 1440 - 220 - 70 = 1150
        assert_eq!(x, 4230);
        assert_eq!(y, 1150);
    }
}
