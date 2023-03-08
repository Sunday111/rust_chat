#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use eframe::egui;

mod application;
mod client;

pub fn run_app() -> Result<(), eframe::Error> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(600., 600.)),
        ..Default::default()
    };
    eframe::run_native(
        "Rust Chat",
        options,
        Box::new(|_cc| Box::new(application::Application::default())),
    )
}
