use audiorecorder::app::AudioRecorderApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([620.0, 720.0])
            .with_min_inner_size([480.0, 520.0])
            .with_title("Audiorecorder"),
        ..Default::default()
    };

    eframe::run_native(
        "audiorecorder",
        options,
        Box::new(|_cc| Ok(Box::new(AudioRecorderApp::new()))),
    )
}
