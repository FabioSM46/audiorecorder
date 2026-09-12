use audiorecorder::app::AudioRecorderApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 480.0])
            .with_min_inner_size([460.0, 360.0])
            .with_title("audiorecorder"),
        ..Default::default()
    };

    eframe::run_native(
        "audiorecorder",
        options,
        Box::new(|_cc| Ok(Box::new(AudioRecorderApp::new()))),
    )
}
