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
    let img_bytes = include_bytes!("../wiki/image.png");
    let img = image::load_from_memory(img_bytes).expect("Failed to load tray icon PNG");
    let resized = img.resize_exact(32, 32, image::imageops::FilterType::Lanczos3);
    let rgba = resized.to_rgba8().into_raw();
    Icon::from_rgba(rgba, 32, 32).expect("Failed to create tray icon")
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
