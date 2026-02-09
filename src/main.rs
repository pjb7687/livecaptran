#![windows_subsystem = "windows"]

mod app;
mod audio;
mod settings;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([3840.0, 500.0])
            .with_position([0.0, 0.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(true)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "LiveCapTran",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
