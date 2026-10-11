mod app;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Scottish house move calculator")
            .with_inner_size([1500.0, 950.0])
            .with_min_inner_size([1000.0, 650.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Scottish house move calculator",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
