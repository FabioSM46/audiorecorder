use std::path::PathBuf;
use std::time::Duration;

use crate::audio::devices::{list_sources, SourceDevice};
use crate::audio::meter::{to_db, SmoothedMeter};
use crate::audio::recorder::Recorder;

const METER_RELEASE_PER_FRAME: f32 = 0.03;

enum Message {
    Error(String),
    Info(String),
}

pub struct AudioRecorderApp {
    devices: Vec<SourceDevice>,
    selected: usize,
    recorder: Option<Recorder>,
    output_dir: PathBuf,
    meter: SmoothedMeter,
    message: Option<Message>,
}

impl AudioRecorderApp {
    pub fn new() -> Self {
        let mut app = Self {
            devices: Vec::new(),
            selected: 0,
            recorder: None,
            output_dir: PathBuf::from("recordings"),
            meter: SmoothedMeter::new(METER_RELEASE_PER_FRAME),
            message: None,
        };
        app.refresh_devices();
        app
    }

    fn refresh_devices(&mut self) {
        match list_sources() {
            Ok(devices) if devices.is_empty() => {
                self.devices = devices;
                self.message = Some(Message::Error("nessuna sorgente audio trovata".into()));
            }
            Ok(devices) => {
                self.selected = devices.iter().position(|d| d.is_default).unwrap_or(0);
                self.devices = devices;
                self.message = None;
            }
            Err(error) => {
                self.devices.clear();
                self.message = Some(Message::Error(format!("{error:#}")));
            }
        }
    }

    fn start_recording(&mut self) {
        let Some(device) = self.devices.get(self.selected) else {
            self.message = Some(Message::Error("seleziona prima una sorgente".into()));
            return;
        };

        if let Err(error) = std::fs::create_dir_all(&self.output_dir) {
            self.message = Some(Message::Error(format!(
                "impossibile creare la cartella {}: {error}",
                self.output_dir.display()
            )));
            return;
        }

        let filename = format!(
            "rec-{}.wav",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
        );
        let path = self.output_dir.join(filename);

        match Recorder::start(&device.name, &path, device.spec) {
            Ok(recorder) => {
                self.recorder = Some(recorder);
                self.meter = SmoothedMeter::new(METER_RELEASE_PER_FRAME);
                self.message = Some(Message::Info(format!(
                    "Registrazione in corso: {}",
                    path.display()
                )));
            }
            Err(error) => self.message = Some(Message::Error(format!("{error:#}"))),
        }
    }

    fn stop_recording(&mut self) {
        if let Some(recorder) = self.recorder.take() {
            match recorder.stop() {
                Ok(()) => self.message = Some(Message::Info("Registrazione salvata".into())),
                Err(error) => self.message = Some(Message::Error(format!("{error:#}"))),
            }
            self.meter = SmoothedMeter::new(METER_RELEASE_PER_FRAME);
        }
    }

    fn choose_output_dir(&mut self) {
        if let Some(dir) = rfd::FileDialog::new()
            .set_directory(&self.output_dir)
            .pick_folder()
        {
            self.output_dir = dir;
        }
    }
}

impl Default for AudioRecorderApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for AudioRecorderApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.recorder.is_some() {
            let error = self.recorder.as_ref().and_then(|r| r.poll_error());
            if let Some(error) = error {
                self.recorder = None;
                self.message = Some(Message::Error(error));
            }
            ui.ctx().request_repaint();
        }

        egui::CentralPanel::default().show(ui, |ui| {
            let recording = self.recorder.is_some();
            let paused = self
                .recorder
                .as_ref()
                .map(Recorder::is_paused)
                .unwrap_or(false);

            ui.heading("audiorecorder");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Sorgente:");
                ui.add_enabled_ui(!recording, |ui| {
                    let current = self
                        .devices
                        .get(self.selected)
                        .map(SourceDevice::display_name)
                        .unwrap_or_else(|| "—".to_string());
                    egui::ComboBox::from_id_salt("source")
                        .selected_text(current)
                        .width(300.0)
                        .show_ui(ui, |ui| {
                            for (index, device) in self.devices.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.selected,
                                    index,
                                    device.display_name(),
                                );
                            }
                        });
                });
                if ui
                    .add_enabled(!recording, egui::Button::new("Aggiorna"))
                    .clicked()
                {
                    self.refresh_devices();
                }
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Cartella:");
                if ui
                    .add_enabled(!recording, egui::Button::new("Scegli…"))
                    .clicked()
                {
                    self.choose_output_dir();
                }
                ui.monospace(self.output_dir.display().to_string());
            });

            ui.add_space(10.0);

            let mut do_start = false;
            let mut do_toggle_pause = false;
            let mut do_stop = false;

            ui.horizontal(|ui| {
                if !recording {
                    if ui.button("● Registra").clicked() {
                        do_start = true;
                    }
                } else {
                    let label = if paused { "▶ Riprendi" } else { "⏸ Pausa" };
                    if ui.button(label).clicked() {
                        do_toggle_pause = true;
                    }
                    if ui.button("■ Stop").clicked() {
                        do_stop = true;
                    }
                }
            });

            if do_start {
                self.start_recording();
            }
            if do_toggle_pause {
                if let Some(recorder) = self.recorder.as_ref() {
                    recorder.set_paused(!paused);
                }
            }
            if do_stop {
                self.stop_recording();
            }

            ui.add_space(10.0);

            match self.recorder.as_ref() {
                Some(recorder) => {
                    let level = self.meter.update(recorder.peak());
                    let duration = recorder.duration();
                    let state = if recorder.is_paused() {
                        "in pausa"
                    } else {
                        "in registrazione"
                    };
                    ui.label(format!("Durata: {}  ·  {state}", format_duration(duration)));
                    ui.add(
                        egui::ProgressBar::new(level.clamp(0.0, 1.0))
                            .text(format!("{:.1} dBFS", to_db(level))),
                    );
                }
                None => {
                    ui.label("Pronto.");
                    ui.add(egui::ProgressBar::new(0.0));
                }
            }

            if let Some(message) = &self.message {
                ui.add_space(8.0);
                let (color, text) = match message {
                    Message::Error(text) => (egui::Color32::LIGHT_RED, text),
                    Message::Info(text) => (egui::Color32::LIGHT_GREEN, text),
                };
                ui.colored_label(color, text);
            }
        });
    }
}

fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    format!("{:02}:{:02}", total / 60, total % 60)
}
