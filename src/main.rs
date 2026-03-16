mod app;
mod engine;
mod platform;
mod worker;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 560.0])
            .with_min_inner_size([560.0, 420.0])
            .with_title("屏幕常醒"),
        ..Default::default()
    };

    eframe::run_native(
        "屏幕常醒",
        options,
        Box::new(|cc| {
            app::configure_theme(&cc.egui_ctx);
            Ok(Box::new(app::AwakeMouseApp::default()))
        }),
    )
}
