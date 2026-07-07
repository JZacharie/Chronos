#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use anyhow::Result;
use directories::ProjectDirs;
use tracing_subscriber::EnvFilter;

use chronos::app::AppState;
use chronos::log_buffer::LogWriter;
use chronos::tray;
use chronos::ui::ChronosApp;

fn db_path() -> Result<PathBuf> {
    let proj = ProjectDirs::from("", "", "Chronos")
        .ok_or_else(|| anyhow::anyhow!("Cannot determine project directories"))?;
    let dir = proj.data_dir();
    std::fs::create_dir_all(dir)?;
    Ok(dir.join("chronos.db"))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(|| LogWriter)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Initializing Chronos...");

    let path = db_path()?;
    tracing::info!("Database path: {}", path.display());

    let state = AppState::new(path)?;

    let tray_ctx = tray::setup_tray();

    let app = ChronosApp::new(state, tray_ctx);

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Chronos — Time Tracker"),
        ..Default::default()
    };

    if let Err(e) = eframe::run_native("Chronos", options, Box::new(|_cc| Ok(Box::new(app)))) {
        tracing::error!("Application error: {e}");
    }

    Ok(())
}
