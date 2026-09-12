use std::path::PathBuf;
use std::time::Duration;

use egui::{
    Align, Color32, CornerRadius, CursorIcon, FontId, Layout, Margin, RichText, Sense, Stroke,
    TextStyle, Vec2,
};

use crate::audio::devices::{list_sources, SourceDevice};
use crate::audio::meter::{to_db, SmoothedMeter};
use crate::audio::recorder::Recorder;

const METER_RELEASE_PER_FRAME: f32 = 0.03;

const BG: Color32 = Color32::from_rgb(0x0E, 0x10, 0x16);
const CARD: Color32 = Color32::from_rgb(0x16, 0x1A, 0x22);
const CARD_STROKE: Color32 = Color32::from_rgb(0x25, 0x2B, 0x37);
const TRACK: Color32 = Color32::from_rgb(0x1C, 0x21, 0x2A);
const TEXT: Color32 = Color32::from_rgb(0xE8, 0xEB, 0xF2);
const MUTED: Color32 = Color32::from_rgb(0x8B, 0x95, 0xA8);
const FAINT: Color32 = Color32::from_rgb(0x5A, 0x63, 0x75);
const ACCENT: Color32 = Color32::from_rgb(0xFF, 0x46, 0x5C);
const ACCENT_HOVER: Color32 = Color32::from_rgb(0xFF, 0x64, 0x77);
const AMBER: Color32 = Color32::from_rgb(0xF5, 0xA5, 0x24);
const AMBER_HOVER: Color32 = Color32::from_rgb(0xFF, 0xB8, 0x45);
const SLATE: Color32 = Color32::from_rgb(0x2A, 0x31, 0x3E);
const SLATE_HOVER: Color32 = Color32::from_rgb(0x37, 0x40, 0x50);
const METER_GREEN: Color32 = Color32::from_rgb(0x22, 0xC5, 0x5E);
const METER_YELLOW: Color32 = Color32::from_rgb(0xFA, 0xCC, 0x15);
const METER_RED: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44);

enum Message {
    Error(String),
    Info(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceFilter {
    All,
    Mic,
    System,
}

pub struct AudioRecorderApp {
    devices: Vec<SourceDevice>,
    selected: usize,
    filter: SourceFilter,
    recorder: Option<Recorder>,
    output_dir: PathBuf,
    meter: SmoothedMeter,
    message: Option<Message>,
    styled: bool,
}

impl AudioRecorderApp {
    pub fn new() -> Self {
        let mut app = Self {
            devices: Vec::new(),
            selected: 0,
            filter: SourceFilter::All,
            recorder: None,
            output_dir: PathBuf::from("recordings"),
            meter: SmoothedMeter::new(METER_RELEASE_PER_FRAME),
            message: None,
            styled: false,
        };
        app.refresh_devices();
        app
    }

    fn refresh_devices(&mut self) {
        match list_sources() {
            Ok(devices) if devices.is_empty() => {
                self.devices = devices;
                self.message = Some(Message::Error("Nessuna sorgente audio trovata".into()));
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
            self.message = Some(Message::Error("Seleziona prima una sorgente".into()));
            return;
        };

        if let Err(error) = std::fs::create_dir_all(&self.output_dir) {
            self.message = Some(Message::Error(format!(
                "Impossibile creare la cartella {}: {error}",
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
                self.message = Some(Message::Info(format!("Salvataggio in {}", path.display())));
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

    fn matches_filter(&self, device: &SourceDevice) -> bool {
        match self.filter {
            SourceFilter::All => true,
            SourceFilter::Mic => !device.is_monitor,
            SourceFilter::System => device.is_monitor,
        }
    }

    fn visible_devices(&self) -> Vec<usize> {
        self.devices
            .iter()
            .enumerate()
            .filter(|(_, d)| self.matches_filter(d))
            .map(|(i, _)| i)
            .collect()
    }

    fn apply_style(&mut self, ctx: &egui::Context) {
        if self.styled {
            return;
        }
        self.styled = true;
        ctx.set_theme(egui::ThemePreference::Dark);

        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = CARD;
        visuals.faint_bg_color = CARD;
        visuals.extreme_bg_color = Color32::from_rgb(0x0A, 0x0C, 0x11);
        visuals.override_text_color = Some(TEXT);
        visuals.hyperlink_color = ACCENT;
        visuals.window_corner_radius = CornerRadius::same(14);
        visuals.selection.bg_fill = ACCENT.gamma_multiply(0.85);
        visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);

        let widgets = &mut visuals.widgets;
        widgets.noninteractive.bg_fill = CARD;
        widgets.noninteractive.bg_stroke = Stroke::new(1.0, CARD_STROKE);
        widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        widgets.noninteractive.corner_radius = CornerRadius::same(10);
        for widget in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
            &mut widgets.open,
        ] {
            widget.bg_fill = SLATE;
            widget.weak_bg_fill = SLATE;
            widget.bg_stroke = Stroke::new(1.0, CARD_STROKE);
            widget.fg_stroke = Stroke::new(1.0, TEXT);
            widget.corner_radius = CornerRadius::same(10);
        }
        widgets.hovered.weak_bg_fill = SLATE_HOVER;
        widgets.hovered.bg_fill = SLATE_HOVER;
        widgets.hovered.bg_stroke = Stroke::new(1.0, FAINT);
        widgets.hovered.expansion = 1.0;
        widgets.open.weak_bg_fill = SLATE_HOVER;
        widgets.open.bg_fill = SLATE_HOVER;
        widgets.active.weak_bg_fill = ACCENT;
        widgets.active.expansion = 0.0;

        ctx.all_styles_mut(|style| {
            style.visuals = visuals.clone();
            style.spacing.item_spacing = Vec2::new(10.0, 10.0);
            style.spacing.button_padding = Vec2::new(12.0, 7.0);
            style.spacing.interact_size.y = 30.0;
            style.spacing.combo_width = 160.0;
            style
                .text_styles
                .insert(TextStyle::Heading, FontId::proportional(22.0));
            style
                .text_styles
                .insert(TextStyle::Body, FontId::proportional(14.0));
            style
                .text_styles
                .insert(TextStyle::Button, FontId::proportional(14.0));
            style
                .text_styles
                .insert(TextStyle::Small, FontId::proportional(11.0));
            style
                .text_styles
                .insert(TextStyle::Monospace, FontId::monospace(13.0));
        });
    }
}

impl Default for AudioRecorderApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for AudioRecorderApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.apply_style(ui.ctx());

        if self.recorder.is_some() {
            if let Some(error) = self.recorder.as_ref().and_then(|r| r.poll_error()) {
                self.recorder = None;
                self.message = Some(Message::Error(error));
            }
            ui.ctx().request_repaint();
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(Margin::same(20)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render(ui);
                    });
            });
    }
}

impl AudioRecorderApp {
    fn render(&mut self, ui: &mut egui::Ui) {
        let recording = self.recorder.is_some();
        let paused = self
            .recorder
            .as_ref()
            .map(Recorder::is_paused)
            .unwrap_or(false);

        self.header(ui, recording, paused);
        ui.add_space(6.0);
        self.source_card(ui, recording);
        self.transport_card(ui, recording, paused);
        self.meter_card(ui, recording, paused);
        self.message_banner(ui);
    }

    fn header(&mut self, ui: &mut egui::Ui, recording: bool, paused: bool) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("audiorecorder")
                        .size(24.0)
                        .strong()
                        .color(TEXT),
                );
                ui.label(
                    RichText::new("Registra microfono o audio di sistema")
                        .size(13.0)
                        .color(MUTED),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                status_badge(ui, recording, paused);
            });
        });
    }

    fn source_card(&mut self, ui: &mut egui::Ui, recording: bool) {
        let visible = self.visible_devices();

        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_label(ui, "SORGENTE");

            ui.horizontal(|ui| {
                if segment(ui, self.filter == SourceFilter::All, "Tutte").clicked() {
                    self.filter = SourceFilter::All;
                }
                if segment(ui, self.filter == SourceFilter::Mic, "Microfono").clicked() {
                    self.filter = SourceFilter::Mic;
                }
                if segment(ui, self.filter == SourceFilter::System, "Sistema").clicked() {
                    self.filter = SourceFilter::System;
                }
            });

            ui.add_space(6.0);

            if !visible.contains(&self.selected) {
                if let Some(first) = visible.first() {
                    self.selected = *first;
                }
            }

            let current = self
                .devices
                .get(self.selected)
                .map(SourceDevice::display_name)
                .unwrap_or_else(|| "—".to_string());

            ui.add_enabled_ui(!recording, |ui| {
                ui.horizontal(|ui| {
                    let combo_width = (ui.available_width() - 46.0).max(120.0);
                    egui::ComboBox::from_id_salt("source")
                        .selected_text(RichText::new(current).color(TEXT))
                        .width(combo_width)
                        .show_ui(ui, |ui| {
                            if visible.is_empty() {
                                ui.weak("Nessun dispositivo in questa categoria");
                            }
                            for index in &visible {
                                let device = &self.devices[*index];
                                let text = RichText::new(device.description.clone())
                                    .color(if device.is_monitor { MUTED } else { TEXT });
                                ui.selectable_value(&mut self.selected, *index, text);
                            }
                        });
                    if icon_button(ui, "↻", "Aggiorna elenco").clicked() {
                        self.refresh_devices();
                    }
                });
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("CARTELLA").size(11.0).strong().color(FAINT));
                ui.add_enabled_ui(!recording, |ui| {
                    if ui.button("Scegli…").clicked() {
                        self.choose_output_dir();
                    }
                });
            });
            ui.add_space(2.0);
            ui.label(
                RichText::new(self.output_dir.display().to_string())
                    .monospace()
                    .size(12.5)
                    .color(MUTED),
            );
        });
    }

    fn transport_card(&mut self, ui: &mut egui::Ui, recording: bool, paused: bool) {
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let full = ui.available_width();

            if !recording {
                let response = pill_button(
                    ui,
                    "Registra",
                    Pill {
                        icon: Some(Icon::Record),
                        fill: ACCENT,
                        fill_hover: ACCENT_HOVER,
                        text_color: Color32::WHITE,
                        height: 50.0,
                        min_width: full,
                    },
                );
                if response.clicked() {
                    self.start_recording();
                }
            } else {
                let half = (full - 10.0) / 2.0;
                let (pause_label, pause_icon) = if paused {
                    ("Riprendi", Icon::Play)
                } else {
                    ("Pausa", Icon::Pause)
                };
                let toggle = pill_button(
                    ui,
                    pause_label,
                    Pill {
                        icon: Some(pause_icon),
                        fill: AMBER,
                        fill_hover: AMBER_HOVER,
                        text_color: Color32::from_rgb(0x20, 0x18, 0x05),
                        height: 44.0,
                        min_width: half,
                    },
                );
                let stop = pill_button(
                    ui,
                    "Stop",
                    Pill {
                        icon: Some(Icon::Stop),
                        fill: SLATE,
                        fill_hover: SLATE_HOVER,
                        text_color: TEXT,
                        height: 44.0,
                        min_width: full - half - 10.0,
                    },
                );
                if toggle.clicked() {
                    if let Some(recorder) = self.recorder.as_ref() {
                        recorder.set_paused(!paused);
                    }
                }
                if stop.clicked() {
                    self.stop_recording();
                }
            }
        });
    }

    fn meter_card(&mut self, ui: &mut egui::Ui, recording: bool, paused: bool) {
        let level = match self.recorder.as_ref() {
            Some(recorder) => self.meter.update(recorder.peak()),
            None => 0.0,
        };
        let duration = self
            .recorder
            .as_ref()
            .map(Recorder::duration)
            .unwrap_or_default();

        card().show(ui, |ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format_duration(duration))
                        .monospace()
                        .size(34.0)
                        .color(if recording { TEXT } else { MUTED }),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (label, color) = if recording && !paused {
                        ("REC", ACCENT)
                    } else if paused {
                        ("IN PAUSA", AMBER)
                    } else {
                        ("PRONTO", MUTED)
                    };
                    ui.label(RichText::new(label).size(12.0).strong().color(color));
                });
            });

            ui.add_space(4.0);
            vu_meter(ui, level);
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{:.1} dBFS", to_db(level)))
                        .size(12.0)
                        .color(MUTED),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("Picco {:>5.1}%", level * 100.0))
                            .size(12.0)
                            .color(FAINT),
                    );
                });
            });
        });
    }

    fn message_banner(&mut self, ui: &mut egui::Ui) {
        let Some(message) = &self.message else {
            return;
        };
        let (color, background, text) = match message {
            Message::Error(text) => (
                Color32::from_rgb(0xFF, 0x9A, 0xA4),
                Color32::from_rgb(0x34, 0x14, 0x1A),
                text,
            ),
            Message::Info(text) => (
                Color32::from_rgb(0x8B, 0xE8, 0xBE),
                Color32::from_rgb(0x0E, 0x2A, 0x20),
                text,
            ),
        };
        ui.add_space(2.0);
        egui::Frame::new()
            .fill(background)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (bar, _) = ui.allocate_exact_size(Vec2::new(4.0, 18.0), Sense::hover());
                    ui.painter().rect_filled(bar, CornerRadius::same(2), color);
                    ui.add_space(2.0);
                    ui.label(RichText::new(text).size(13.0).color(color));
                });
            });
    }
}

fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, CARD_STROKE))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(16))
        .outer_margin(Margin::symmetric(0, 5))
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).strong().color(FAINT));
    ui.add_space(4.0);
}

fn segment(ui: &mut egui::Ui, selected: bool, text: &str) -> egui::Response {
    let (fill, text_color, stroke) = if selected {
        (
            ACCENT.gamma_multiply(0.18),
            ACCENT,
            ACCENT.gamma_multiply(0.55),
        )
    } else {
        (Color32::TRANSPARENT, MUTED, CARD_STROKE)
    };
    ui.add(
        egui::Button::new(RichText::new(text).size(13.0).color(text_color))
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(CornerRadius::same(8))
            .min_size(Vec2::new(0.0, 30.0)),
    )
}

const ICON_SIZE: f32 = 13.0;
const ICON_GAP: f32 = 10.0;

fn pill_button(ui: &mut egui::Ui, text: &str, pill: Pill) -> egui::Response {
    let font_id = FontId::proportional(15.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font_id, pill.text_color);
    let icon_space = if pill.icon.is_some() {
        ICON_SIZE + ICON_GAP
    } else {
        0.0
    };
    let content_width = galley.size().x + icon_space;
    let width = (content_width + 48.0).max(pill.min_width);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, pill.height), Sense::click());

    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    let background = if pressed {
        pill.fill.gamma_multiply(0.85)
    } else if hovered {
        pill.fill_hover
    } else {
        pill.fill
    };

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(12), background);

    let mut content_x = rect.center().x - content_width / 2.0;
    if let Some(icon) = pill.icon {
        let center = egui::pos2(content_x + ICON_SIZE / 2.0, rect.center().y);
        draw_icon(painter, icon, center, ICON_SIZE, pill.text_color);
        content_x += icon_space;
    }
    painter.galley(
        egui::pos2(content_x, rect.center().y - galley.size().y / 2.0),
        galley,
        pill.text_color,
    );

    response.on_hover_cursor(CursorIcon::PointingHand)
}

#[derive(Clone, Copy)]
enum Icon {
    Record,
    Pause,
    Play,
    Stop,
}

fn draw_icon(painter: &egui::Painter, icon: Icon, center: egui::Pos2, size: f32, color: Color32) {
    match icon {
        Icon::Record => {
            painter.circle_filled(center, size * 0.42, color);
        }
        Icon::Stop => {
            let rect = egui::Rect::from_center_size(center, Vec2::splat(size * 0.82));
            painter.rect_filled(rect, CornerRadius::same(2), color);
        }
        Icon::Pause => {
            let bar_width = size * 0.3;
            let gap = size * 0.22;
            let offset = gap / 2.0 + bar_width / 2.0;
            for dx in [-offset, offset] {
                let rect = egui::Rect::from_center_size(
                    egui::pos2(center.x + dx, center.y),
                    Vec2::new(bar_width, size),
                );
                painter.rect_filled(rect, CornerRadius::same(1), color);
            }
        }
        Icon::Play => {
            let points = vec![
                egui::pos2(center.x - size * 0.36, center.y - size * 0.5),
                egui::pos2(center.x + size * 0.5, center.y),
                egui::pos2(center.x - size * 0.36, center.y + size * 0.5),
            ];
            painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
        }
    }
}

struct Pill {
    icon: Option<Icon>,
    fill: Color32,
    fill_hover: Color32,
    text_color: Color32,
    height: f32,
    min_width: f32,
}

fn status_badge(ui: &mut egui::Ui, recording: bool, paused: bool) {
    let (dot, text) = if recording && !paused {
        (ACCENT, "REC")
    } else if paused {
        (AMBER, "IN PAUSA")
    } else {
        (MUTED, "PRONTO")
    };
    let pulse = if recording && !paused {
        let time = ui.input(|i| i.time) as f32;
        0.55 + 0.45 * (time * 3.0).sin().abs()
    } else {
        1.0
    };

    egui::Frame::new()
        .fill(Color32::from_rgb(0x12, 0x15, 0x1C))
        .stroke(Stroke::new(1.0, CARD_STROKE))
        .corner_radius(CornerRadius::same(20))
        .inner_margin(Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(9.0), Sense::hover());
                ui.painter()
                    .circle_filled(rect.center(), 4.5, dot.gamma_multiply(pulse));
                ui.label(RichText::new(text).size(11.5).strong().color(TEXT));
            });
        });
}

fn vu_meter(ui: &mut egui::Ui, level: f32) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), Sense::hover());
    let segments = 32usize;
    let gap = 3.0;
    let segment_width = (rect.width() - gap * (segments - 1) as f32) / segments as f32;
    let active = (level.clamp(0.0, 1.0) * segments as f32).round() as usize;
    let painter = ui.painter();

    for index in 0..segments {
        let x = rect.left() + index as f32 * (segment_width + gap);
        let segment = egui::Rect::from_min_size(
            egui::pos2(x, rect.top()),
            Vec2::new(segment_width, rect.height()),
        );
        let position = index as f32 / (segments - 1) as f32;
        let color = if index < active {
            meter_color(position)
        } else {
            TRACK
        };
        painter.rect_filled(segment, CornerRadius::same(3), color);
    }

    response
}

fn meter_color(position: f32) -> Color32 {
    let position = position.clamp(0.0, 1.0);
    if position < 0.75 {
        lerp_color(METER_GREEN, METER_YELLOW, position / 0.75)
    } else {
        lerp_color(METER_YELLOW, METER_RED, (position - 0.75) / 0.25)
    }
}

fn lerp_color(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgb(
        channel(from.r(), to.r()),
        channel(from.g(), to.g()),
        channel(from.b(), to.b()),
    )
}

fn icon_button(ui: &mut egui::Ui, icon: &str, tooltip: &str) -> egui::Response {
    let response = ui.add_sized(
        Vec2::new(36.0, 30.0),
        egui::Button::new(RichText::new(icon).size(15.0).color(MUTED)),
    );
    response.on_hover_text(tooltip)
}

fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    format!("{:02}:{:02}", total / 60, total % 60)
}
