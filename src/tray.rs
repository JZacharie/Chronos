use tray_icon::menu::{Menu, MenuEvent, MenuEventReceiver, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent, TrayIconEventReceiver};

pub const MENU_ID_START: &str = "start";
pub const MENU_ID_STOP: &str = "stop";
pub const MENU_ID_PAUSE: &str = "pause";
pub const MENU_ID_RESUME: &str = "resume";
pub const MENU_ID_TOGGLE: &str = "toggle";
pub const MENU_ID_QUIT: &str = "quit";

pub struct TrayContext {
    pub tray: TrayIcon,
    pub menu_rx: MenuEventReceiver,
    pub _tray_rx: TrayIconEventReceiver,
    pub items: TrayMenuItems,
}

pub struct TrayMenuItems {
    pub start: MenuItem,
    pub stop: MenuItem,
    pub pause: MenuItem,
    pub resume: MenuItem,
    pub toggle: MenuItem,
    pub quit: MenuItem,
}

pub fn create_icon() -> Icon {
    let size = 32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        for x in 0..size {
            let cx = size as f32 / 2.0;
            let cy = size as f32 / 2.0;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let radius = size as f32 / 2.0 - 1.0;

            if dist > radius {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }

            let angle = dy.atan2(dx);
            let hour_angle = -std::f32::consts::FRAC_PI_2;
            let minute_angle = angle;

            let hour_len = radius * 0.5;
            let minute_len = radius * 0.8;

            let is_hour = {
                let hx = cx + hour_angle.cos() * hour_len;
                let hy = cy + hour_angle.sin() * hour_len;
                let lx = x as f32 - cx;
                let ly = y as f32 - cy;
                let dot = lx * (hx - cx) + ly * (hy - cy);
                let len_sq = (hx - cx) * (hx - cx) + (hy - cy) * (hy - cy);
                if len_sq == 0.0 {
                    false
                } else {
                    let t = (dot / len_sq).clamp(0.0, 1.0);
                    let px = cx + (hx - cx) * t;
                    let py = cy + (hy - cy) * t;
                    let d = ((x as f32 - px).powi(2) + (y as f32 - py).powi(2)).sqrt();
                    d < 2.5
                }
            };

            let is_minute = {
                let mx = cx + minute_angle.cos() * minute_len;
                let my = cy + minute_angle.sin() * minute_len;
                let lx = x as f32 - cx;
                let ly = y as f32 - cy;
                let dot = lx * (mx - cx) + ly * (my - cy);
                let len_sq = (mx - cx) * (mx - cx) + (my - cy) * (my - cy);
                if len_sq == 0.0 {
                    false
                } else {
                    let t = (dot / len_sq).clamp(0.0, 1.0);
                    let px = cx + (mx - cx) * t;
                    let py = cy + (my - cy) * t;
                    let d = ((x as f32 - px).powi(2) + (y as f32 - py).powi(2)).sqrt();
                    d < 2.0
                }
            };

            let is_center = dist < 3.0;
            let is_circle_edge = dist > radius - 1.5;

            if is_center || is_hour || is_minute {
                rgba.extend_from_slice(&[200, 200, 220, 255]);
            } else if is_circle_edge {
                rgba.extend_from_slice(&[180, 180, 200, 255]);
            } else if dist < radius - 1.0 {
                rgba.extend_from_slice(&[60, 60, 80, 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}

pub fn setup_tray() -> TrayContext {
    let start = MenuItem::with_id(MENU_ID_START, "Start Tracking", true, None);
    let stop = MenuItem::with_id(MENU_ID_STOP, "Stop Tracking", true, None);
    let pause = MenuItem::with_id(MENU_ID_PAUSE, "Pause", true, None);
    let resume = MenuItem::with_id(MENU_ID_RESUME, "Resume", true, None);
    let toggle = MenuItem::with_id(MENU_ID_TOGGLE, "Show/Hide Window", true, None);
    let quit = MenuItem::with_id(MENU_ID_QUIT, "Quit", true, None);

    let menu = Menu::new();
    menu.append_items(&[
        &start,
        &stop,
        &pause,
        &resume,
        &PredefinedMenuItem::separator(),
        &toggle,
        &PredefinedMenuItem::separator(),
        &quit,
    ])
    .expect("Failed to build tray menu");

    let icon = create_icon();

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Chronos - Time Tracker")
        .with_icon(icon)
        .build()
        .expect("Failed to create tray icon");

    let menu_rx = MenuEvent::receiver();
    let tray_rx = TrayIconEvent::receiver();

    TrayContext {
        tray,
        menu_rx: menu_rx.clone(),
        _tray_rx: tray_rx.clone(),
        items: TrayMenuItems {
            start,
            stop,
            pause,
            resume,
            toggle,
            quit,
        },
    }
}
