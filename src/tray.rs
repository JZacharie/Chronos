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
    let svg_data = include_bytes!("../resources/chronos.svg");
    let tree = resvg::usvg::Tree::from_data(svg_data, &resvg::usvg::Options::default())
        .expect("Failed to parse SVG icon");

    let size = 32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)
        .expect("Failed to create pixmap for tray icon");

    let scale = size as f32 / 64.0;
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let premul = pixmap.data();
    let mut rgba = Vec::with_capacity(premul.len());

    for chunk in premul.chunks_exact(4) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let a = chunk[3];
        if a == 0 {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else if a == 255 {
            rgba.extend_from_slice(&[r, g, b, a]);
        } else {
            rgba.extend_from_slice(&[
                ((r as u16 * 255) / a as u16) as u8,
                ((g as u16 * 255) / a as u16) as u8,
                ((b as u16 * 255) / a as u16) as u8,
                a,
            ]);
        }
    }

    Icon::from_rgba(rgba, size, size).expect("Failed to create tray icon from SVG")
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
