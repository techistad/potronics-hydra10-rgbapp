#![windows_subsystem = "windows"]

mod device;
mod effects;
mod layout;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use device_query::{DeviceQuery, DeviceState, Keycode};
use eframe::egui::{self, Color32, RichText, Sense, Vec2};
use hidapi::{HidApi, HidDevice};
use serde::{Deserialize, Serialize};

use crate::device::{write_leds, Rgb};
use crate::effects::{palette_from, CustomPattern, EffectEngine, Mode, Profile};
use crate::layout::KEYS;

const MARK: Color32 = Color32::from_rgb(0xea, 0x0b, 0x2a);
const INK: Color32 = Color32::from_rgb(0x0c, 0x0f, 0x0c);
const PANEL: Color32 = Color32::from_rgb(0x14, 0x18, 0x13);
const PAPER: Color32 = Color32::from_rgb(0xf3, 0xee, 0xe4);
const MUTED: Color32 = Color32::from_rgb(0xa8, 0xb0, 0xa4);
const TEAL: Color32 = Color32::from_rgb(0xb6, 0xe0, 0x6a);
const LINE: Color32 = Color32::from_rgb(0x2c, 0x36, 0x2a);
const FIELD: Color32 = Color32::from_rgb(0x0e, 0x12, 0x0d);

enum Cmd {
    Connect,
    Release,
    Stop,
}

struct Shared {
    engine: EffectEngine,
    streaming: bool,
    message: String,
    preview: HashMap<String, Rgb>,
    product: String,
}

struct View {
    mode: Mode,
    streaming: bool,
    message: String,
    preview: HashMap<String, Rgb>,
    primary: Rgb,
    secondary: Rgb,
    brightness: f32,
    speed: f32,
    base: f32,
    custom_on: bool,
    palette: [Rgb; 5],
    pattern: CustomPattern,
    paint: HashMap<String, Rgb>,
}

struct HydraWindow {
    shared: Arc<Mutex<Shared>>,
    cmd: Sender<Cmd>,
    worker: Option<JoinHandle<()>>,
    profile_name: String,
    selected_profile: String,
    profiles: Vec<String>,
    customs: [Option<SavedEffect>; 2],
    active_custom: Option<usize>,
    resume_link: bool,
    quit_requested: bool,
    run_at_startup: bool,
    startup_profile: String,
    in_tray: bool,
    tray: Option<tray_icon::TrayIcon>,
    view: View,
}

#[derive(Clone, Serialize, Deserialize)]
struct SavedEffect {
    name: String,
    profile: Profile,
    #[serde(default)]
    colors: Vec<Rgb>,
    #[serde(default)]
    pattern: CustomPattern,
}

#[derive(Clone, Serialize, Deserialize)]
struct Session {
    profile: Profile,
    custom_on: bool,
    palette: Vec<Rgb>,
    pattern: CustomPattern,
    active_custom: Option<usize>,
    linked: bool,
    #[serde(default)]
    run_at_startup: bool,
    #[serde(default)]
    startup_profile: String,
}

impl Default for Session {
    fn default() -> Self {
        let engine = EffectEngine::new();
        Self {
            profile: engine.snapshot(),
            custom_on: false,
            palette: engine.palette.to_vec(),
            pattern: engine.pattern,
            active_custom: None,
            linked: true,
            run_at_startup: false,
            startup_profile: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ProfileStore {
    #[serde(default)]
    current: String,
    #[serde(default)]
    items: HashMap<String, Profile>,
    #[serde(default)]
    customs: [Option<SavedEffect>; 2],
    #[serde(default)]
    session: Session,
}

impl Default for ProfileStore {
    fn default() -> Self {
        Self {
            current: String::new(),
            items: HashMap::new(),
            customs: [None, None],
            session: Session::default(),
        }
    }
}

fn profiles_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("profiles.json")
}

fn read_store() -> ProfileStore {
    let Ok(text) = fs::read_to_string(profiles_path()) else {
        return ProfileStore::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn write_store(store: &ProfileStore) {
    if let Ok(text) = serde_json::to_string_pretty(store) {
        let _ = fs::write(profiles_path(), text);
    }
}

fn map_keycode(key: &Keycode) -> Option<&'static str> {
    Some(match key {
        Keycode::Escape => "esc",
        Keycode::Tab => "tab",
        Keycode::CapsLock => "caps",
        Keycode::LShift => "lshift",
        Keycode::RShift => "rshift",
        Keycode::LControl => "lctrl",
        Keycode::RControl => "rctrl",
        Keycode::LAlt => "lalt",
        Keycode::RAlt => "ralt",
        Keycode::LMeta | Keycode::RMeta => "lwin",
        Keycode::Space => "space",
        Keycode::Enter => "enter",
        Keycode::Backspace => "bksp",
        Keycode::Up => "up",
        Keycode::Down => "down",
        Keycode::Left => "left",
        Keycode::Right => "right",
        Keycode::Home => "home",
        Keycode::Delete => "del",
        Keycode::PageUp => "pgup",
        Keycode::PageDown => "pgdn",
        Keycode::Minus => "minus",
        Keycode::Equal => "equal",
        Keycode::LeftBracket => "lb",
        Keycode::RightBracket => "rb",
        Keycode::BackSlash => "bslash",
        Keycode::Semicolon => "semi",
        Keycode::Apostrophe => "quote",
        Keycode::Comma => "comma",
        Keycode::Dot => "dot",
        Keycode::Slash => "slash",
        Keycode::Key0 => "0",
        Keycode::Key1 => "1",
        Keycode::Key2 => "2",
        Keycode::Key3 => "3",
        Keycode::Key4 => "4",
        Keycode::Key5 => "5",
        Keycode::Key6 => "6",
        Keycode::Key7 => "7",
        Keycode::Key8 => "8",
        Keycode::Key9 => "9",
        Keycode::A => "a",
        Keycode::B => "b",
        Keycode::C => "c",
        Keycode::D => "d",
        Keycode::E => "e",
        Keycode::F => "f",
        Keycode::G => "g",
        Keycode::H => "h",
        Keycode::I => "i",
        Keycode::J => "j",
        Keycode::K => "k",
        Keycode::L => "l",
        Keycode::M => "m",
        Keycode::N => "n",
        Keycode::O => "o",
        Keycode::P => "p",
        Keycode::Q => "q",
        Keycode::R => "r",
        Keycode::S => "s",
        Keycode::T => "t",
        Keycode::U => "u",
        Keycode::V => "v",
        Keycode::W => "w",
        Keycode::X => "x",
        Keycode::Y => "y",
        Keycode::Z => "z",
        _ => return None,
    })
}

fn worker(shared: Arc<Mutex<Shared>>, rx: Receiver<Cmd>) {
    let mut api = HidApi::new().ok();
    let mut device: Option<HidDevice> = None;
    let keys = DeviceState::new();
    let mut held: HashSet<String> = HashSet::new();
    loop {
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                Cmd::Stop => {
                    if let Ok(mut state) = shared.lock() {
                        state.streaming = false;
                    }
                    return;
                }
                Cmd::Release => {
                    device = None;
                    if let Ok(mut state) = shared.lock() {
                        state.streaming = false;
                        state.product.clear();
                        state.message = "Reset. The keyboard is back on its original effects. Press Link keyboard when you want the saved look again.".into();
                    }
                }
                Cmd::Connect => match api.as_mut() {
                    None => {
                        if let Ok(mut state) = shared.lock() {
                            state.streaming = false;
                            state.message = "USB access failed to start.".into();
                        }
                    }
                    Some(api) => match device::open_keyboard(api) {
                        Ok(opened) => {
                            device = Some(opened.device);
                            if let Ok(mut state) = shared.lock() {
                                state.streaming = true;
                                state.product = opened.product.clone();
                                state.message = format!(
                                    "Linked to {} (PID {:04X}). Live colors only — onboard presets are untouched.",
                                    opened.product, opened.product_id
                                );
                            }
                        }
                        Err(err) => {
                            device = None;
                            if let Ok(mut state) = shared.lock() {
                                state.streaming = false;
                                state.product.clear();
                                state.message = err;
                            }
                        }
                    },
                },
            }
        }

        for key in keys.get_keys() {
            if let Some(id) = map_keycode(&key) {
                if held.insert(id.to_string()) {
                    if let Ok(mut state) = shared.lock() {
                        state.engine.note_press(id);
                    }
                }
            }
        }
        held.retain(|id| {
            keys.get_keys()
                .iter()
                .filter_map(|key| map_keycode(key))
                .any(|name| name == id)
        });

        let (streaming, leds) = {
            let Ok(mut state) = shared.try_lock() else {
                thread::sleep(Duration::from_millis(8));
                continue;
            };
            let (leds, preview) = state.engine.frame();
            state.preview = preview;
            (state.streaming && device.is_some(), leds)
        };
        if streaming {
            if let Some(dev) = device.as_ref() {
                if let Err(err) = write_leds(dev, &leds) {
                    device = None;
                    if let Ok(mut state) = shared.lock() {
                        state.streaming = false;
                        state.message = err;
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(30));
    }
}

impl HydraWindow {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_theme(&cc.egui_ctx);

        let mut engine = EffectEngine::new();
        let store = read_store();
        let session = store.session.clone();
        engine.apply(session.profile.clone());
        if session.custom_on {
            let palette = if session.palette.len() == 5 {
                [
                    session.palette[0],
                    session.palette[1],
                    session.palette[2],
                    session.palette[3],
                    session.palette[4],
                ]
            } else {
                palette_from(session.profile.primary, session.profile.secondary)
            };
            engine.begin_custom(palette, session.pattern);
            engine.brightness = session.profile.brightness;
            engine.speed = session.profile.speed;
            engine.base_level = session.profile.base_level;
        }
        if std::env::args().any(|arg| arg == "--startup") {
            if let Some(profile) = store.items.get(&session.startup_profile).cloned() {
                engine.apply(profile);
            }
        }
        if session.run_at_startup {
            set_run_at_startup(true);
        }
        let startup = if session.custom_on {
            let name = session
                .active_custom
                .and_then(|slot| store.customs.get(slot).and_then(|saved| saved.as_ref()))
                .map(|saved| saved.name.clone())
                .unwrap_or_else(|| "your custom effect".into());
            format!("Opened your saved look: {name}.")
        } else if session.linked {
            format!("Opened your saved look: {}.", engine.mode.name())
        } else {
            format!(
                "Saved look is {}. The keyboard stays on its original effects until you press Link keyboard.",
                engine.mode.name()
            )
        };
        let view = View {
            mode: engine.mode,
            streaming: false,
            message: startup.clone(),
            preview: HashMap::new(),
            primary: engine.primary,
            secondary: engine.secondary,
            brightness: engine.brightness,
            speed: engine.speed,
            base: engine.base_level,
            custom_on: engine.custom_on,
            palette: engine.palette,
            pattern: engine.pattern,
            paint: engine.paint.clone(),
        };
        let shared = Arc::new(Mutex::new(Shared {
            engine,
            streaming: false,
            message: startup,
            preview: HashMap::new(),
            product: String::new(),
        }));
        let (tx, rx) = mpsc::channel();
        let worker_shared = Arc::clone(&shared);
        let worker = thread::spawn(move || worker(worker_shared, rx));
        if session.linked {
            let _ = tx.send(Cmd::Connect);
        }
        let profiles = store.items.keys().cloned().collect();
        let customs = store.customs.clone();
        let active_custom = session.active_custom.filter(|slot| customs.get(*slot).and_then(|saved| saved.as_ref()).is_some());
        Self {
            shared,
            cmd: tx,
            worker: Some(worker),
            profile_name: String::new(),
            selected_profile: store.current,
            profiles,
            customs,
            active_custom,
            resume_link: session.linked,
            quit_requested: false,
            run_at_startup: session.run_at_startup,
            startup_profile: session.startup_profile.clone(),
            in_tray: false,
            tray: None,
            view,
        }
    }

    fn hint(mode: Mode) -> &'static str {
        match mode {
            Mode::Rainbow => "Brightness and speed are the controls. The spectrum on the keys is Rainbow's own.",
            Mode::Paint => "Click a key on the keyboard above to make it glow in the brush color. Click that same key again to turn it off. The keys you type on the real keyboard are not how you pick them.",
            Mode::Afterglow => "Type anywhere. Each key flares, spills onto its neighbors, then goes dark.",
            Mode::Reactive | Mode::Ripple => {
                "Type anywhere and the board follows. Fn itself stays inside the keyboard, so that key will not flash."
            }
            _ => "Live effects need the USB-C cable. The 2.4 GHz dongle and Bluetooth only carry typing.",
        }
    }
}

impl eframe::App for HydraWindow {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        INK.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ensure_tray(ctx);
        if tray_quit_requested() {
            self.remember();
            let _ = self.cmd.send(Cmd::Stop);
            self.tray.take();
            std::process::exit(0);
        }
        if tray_show_requested() {
            self.in_tray = false;
            self.show_window(ctx);
        }
        if self.in_tray {
            ctx.request_repaint_after(Duration::from_millis(200));
        } else if ctx.input(|input| input.pointer.any_down()) {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        if ctx.input(|input| input.viewport().close_requested()) {
            self.remember();
            if !self.quit_requested {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                self.in_tray = true;
            }
        }
        let mut paint_changed = false;
        if let Ok(state) = self.shared.try_lock() {
            self.view.mode = state.engine.mode;
            self.view.streaming = state.streaming;
            self.view.message.clone_from(&state.message);
            self.view.preview.clone_from(&state.preview);
            self.view.primary = state.engine.primary;
            self.view.secondary = state.engine.secondary;
            self.view.brightness = state.engine.brightness;
            self.view.speed = state.engine.speed;
            self.view.base = state.engine.base_level;
            self.view.custom_on = state.engine.custom_on;
            self.view.palette = state.engine.palette;
            self.view.pattern = state.engine.pattern;
            if state.engine.paint != self.view.paint {
                self.view.paint.clone_from(&state.engine.paint);
                paint_changed = true;
            }
        }
        if paint_changed {
            self.remember();
        }
        let mode = self.view.mode;
        let streaming = self.view.streaming;
        let message = self.view.message.clone();
        let preview = self.view.preview.clone();
        let primary = self.view.primary;
        let secondary = self.view.secondary;
        let brightness = self.view.brightness;
        let speed = self.view.speed;
        let base = self.view.base;
        let custom_on = self.view.custom_on;
        let palette = self.view.palette;
        let pattern = self.view.pattern;

        title_bar(ctx);

        egui::SidePanel::right("controls")
            .exact_width(280.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(18).stroke(egui::Stroke::new(1.0_f32, LINE)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .drag_to_scroll(false)
                    .show(ui, |ui| {
                        self.side_controls(ui, mode, streaming, &message);
                    });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(INK).inner_margin(28))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                    ui.painter().circle_filled(
                        dot.center(),
                        4.0,
                        if streaming { TEAL } else { Color32::from_rgb(0x6a, 0x72, 0x66) },
                    );
                    ui.label(RichText::new(if streaming { "STREAMING" } else { "STANDBY" }).color(MUTED).size(11.0));
                });
                ui.add_space(10.0);
                brand_lockup(ui);
                ui.add_space(18.0);
                self.keyboard(ui, &preview, mode, primary, custom_on);
                ui.add_space(16.0);
                self.deck(ui, mode, primary, secondary, brightness, speed, base, custom_on, palette, pattern);
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.remember();
        let _ = self.cmd.send(Cmd::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl HydraWindow {
    fn keyboard(&mut self, ui: &mut egui::Ui, preview: &HashMap<String, Rgb>, mode: Mode, brush: Rgb, custom_on: bool) {
        ui.vertical(|ui| {
            egui::Frame::new()
                .fill(PANEL)
                .inner_margin(14.0)
                .corner_radius(16.0)
                .show(ui, |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(0x07, 0x09, 0x07))
                        .inner_margin(12.0)
                        .corner_radius(12.0)
                        .show(ui, |ui| {
                            let inner = ui.available_width();
                            let gap = 5.0;
                            let columns = 16.0_f32;
                            let unit = ((inner - (columns - 1.0) * gap) / columns).clamp(22.0, 58.0);
                            let key_h = (unit * 1.08).clamp(32.0, 62.0);
                            let pitch_x = unit + gap;
                            let pitch_y = key_h + gap;
                            let board_w = columns * unit + (columns - 1.0) * gap;
                            let board_h = 5.0 * key_h + 4.0 * gap;
                            let (board, _) = ui.allocate_exact_size(Vec2::new(board_w, board_h), Sense::hover());
                            for key in KEYS {
                                let (origin, span) = crate::layout::visual_span(key.id, key.width);
                                let x = board.min.x + origin * pitch_x;
                                let y = board.min.y + key.row as f32 * pitch_y;
                                let width = span * unit + (span - 1.0).max(0.0) * gap;
                                let rect = egui::Rect::from_min_size(egui::pos2(x, y), Vec2::new(width, key_h));
                                let response = ui.interact(rect, ui.id().with(key.id), Sense::click());
                                let painting = mode == Mode::Paint && !custom_on;
                                if painting && response.clicked() {
                                    if let Ok(mut state) = self.shared.lock() {
                                        state.engine.primary = brush;
                                        if state.engine.paint.contains_key(key.id) {
                                            state.engine.paint.remove(key.id);
                                        } else {
                                            state.engine.paint.insert(key.id.to_string(), brush);
                                        }
                                    }
                                    self.remember();
                                }
                                let color = preview.get(key.id).copied().unwrap_or([0, 0, 0]);
                                let fill = Color32::from_rgb(color[0], color[1], color[2]);
                                let shadow = rect.translate(Vec2::new(0.0, 3.0));
                                ui.painter().rect_filled(shadow, 8.0, Color32::from_black_alpha(90));
                                ui.painter().rect_filled(rect, 8.0, fill);
                                let lip = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.top() + 3.0));
                                ui.painter().rect_filled(lip, egui::CornerRadius::ZERO, Color32::from_white_alpha(28));
                                let label_color = ink(color);
                                if let Some(direction) = arrow_direction(key.id) {
                                    paint_arrow(ui.painter(), rect, direction, label_color);
                                } else if !key.label.is_empty() {
                                    ui.painter().text(
                                        rect.center() + Vec2::new(0.0, 1.0),
                                        egui::Align2::CENTER_CENTER,
                                        key.label,
                                        egui::FontId::proportional((unit * 0.32).clamp(12.0, 16.0)),
                                        label_color,
                                    );
                                }
                            }
                        });
                });
        });
    }

    fn side_controls(&mut self, ui: &mut egui::Ui, mode: Mode, streaming: bool, message: &str) {
        ui.spacing_mut().item_spacing.y = 8.0;
        if filled_button(ui, if streaming { "Linked" } else { "Link keyboard" }, TEAL, Color32::from_rgb(0x14, 0x20, 0x0c)).clicked() {
            self.resume_link = true;
            self.remember();
            let _ = self.cmd.send(Cmd::Connect);
        }
        if ghost_button(ui, "Reset to keyboard").clicked() {
            self.resume_link = false;
            self.remember();
            let _ = self.cmd.send(Cmd::Release);
        }
        ui.label(
            RichText::new("Close hides Hydra in the tray beside the clock. The USB effect keeps running. Right-click that icon to quit.")
                .color(MUTED)
                .size(12.0),
        );
        ui.add_space(14.0);
        section_label(ui, "Effect");
        for choice in Mode::ALL {
            let selected = self.active_custom.is_none() && choice == mode;
            if effect_row(ui, choice.name(), selected).clicked() && !selected {
                self.active_custom = None;
                if let Ok(mut state) = self.shared.lock() {
                    state.engine.mode = choice;
                    state.engine.custom_on = false;
                }
                self.resume_link = true;
                self.remember();
                let _ = self.cmd.send(Cmd::Connect);
            }
        }
        ui.add_space(12.0);
        section_label(ui, "Your effects");
        for slot in 0..2 {
            let label = self.customs[slot]
                .as_ref()
                .map(|saved| saved.name.as_str())
                .unwrap_or(if slot == 0 { "Custom 1" } else { "Custom 2" });
            let selected = self.active_custom == Some(slot);
            if effect_row(ui, label, selected).clicked() {
                self.apply_custom(slot);
            }
        }
        ui.add_space(16.0);
        section_label(ui, "Instructions");
        if let Some(slot) = self.active_custom {
            let pattern = self.view.pattern;
            ui.label(RichText::new(format!("Custom {}", slot + 1)).color(PAPER).size(13.0));
            ui.add_space(6.0);
            ui.label(RichText::new(pattern.blurb()).color(MUTED).size(12.0));
            ui.add_space(6.0);
            ui.label(
                RichText::new("Edit all five colors under the keyboard. Spread and Blocks stay still. Cycle and Chase use Speed.")
                    .color(MUTED)
                    .size(12.0),
            );
        } else {
            ui.label(RichText::new(mode.blurb()).color(PAPER).size(13.0));
            ui.add_space(6.0);
            ui.label(RichText::new(Self::hint(mode)).color(MUTED).size(12.0));
        }
        ui.add_space(8.0);
        ui.label(RichText::new(message).color(MUTED).size(12.0));
    }

    fn deck(
        &mut self,
        ui: &mut egui::Ui,
        mode: Mode,
        mut primary: Rgb,
        mut secondary: Rgb,
        mut brightness: f32,
        mut speed: f32,
        mut base: f32,
        custom_on: bool,
        mut palette: [Rgb; 5],
        mut pattern: CustomPattern,
    ) {
        egui::Frame::new()
            .fill(PANEL)
            .inner_margin(16.0)
            .corner_radius(16.0)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                if custom_on {
                    section_label(ui, "Colors");
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Five colors for this effect. Change any of them.")
                            .color(PAPER)
                            .size(12.0),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        for index in 0..5 {
                            color_swatch(ui, &format!("palette-{index}"), &format!("{}", index + 1), &mut palette[index]);
                        }
                    });
                    ui.add_space(10.0);
                    section_label(ui, "Pattern");
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        for choice in CustomPattern::ALL {
                            let on = choice == pattern;
                            let fill = if on { TEAL } else { FIELD };
                            let text = if on { Color32::from_rgb(0x14, 0x20, 0x0c) } else { PAPER };
                            if bar_button(ui, choice.name(), 92.0, fill, text, !on).clicked() {
                                pattern = choice;
                            }
                        }
                    });
                    ui.add_space(8.0);
                    ui.label(RichText::new(pattern.blurb()).color(MUTED).size(12.0));
                    ui.add_space(8.0);
                    let show_speed = pattern.uses_speed();
                    let columns = if show_speed { 2.0 } else { 1.0 };
                    let column = ((ui.available_width() - 18.0 * (columns - 1.0)) / columns).max(180.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 18.0;
                        meter(ui, "Brightness", &mut brightness, 0.0..=1.0, column);
                        if show_speed {
                            meter(ui, "Speed", &mut speed, 0.05..=1.0, column);
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 18.0;
                        ui.vertical(|ui| {
                            section_label(ui, "Color");
                            ui.add_space(8.0);
                            if mode.uses_primary() {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 14.0;
                                    let caption = if mode == Mode::Paint { "Brush" } else { "Main" };
                                    color_swatch(ui, "main", caption, &mut primary);
                                    if mode.uses_second_color() {
                                        color_swatch(ui, "second", "Second", &mut secondary);
                                    }
                                });
                            } else {
                                spectrum_note(ui);
                            }
                        });
                        let (rule, _) = ui.allocate_exact_size(Vec2::new(1.0, 58.0), Sense::hover());
                        ui.painter().rect_filled(rule, 0.0, LINE);
                        let show_speed = mode.uses_speed();
                        let show_glow = mode.uses_rest_glow();
                        let columns = 1.0
                            + if show_speed { 1.0 } else { 0.0 }
                            + if show_glow { 1.0 } else { 0.0 };
                        let gaps = 18.0 * (columns - 1.0);
                        let column = ((ui.available_width() - gaps) / columns).max(140.0);
                        meter(ui, "Brightness", &mut brightness, 0.0..=1.0, column);
                        if show_speed {
                            meter(ui, "Speed", &mut speed, 0.05..=1.0, column);
                        }
                        if show_glow {
                            meter(ui, "Resting glow", &mut base, 0.0..=0.8, column);
                        }
                    });
                }
                let mut changed = false;
                if primary != self.view.primary || secondary != self.view.secondary {
                    if let Ok(mut state) = self.shared.try_lock() {
                        state.engine.primary = primary;
                        state.engine.secondary = secondary;
                    }
                    self.view.primary = primary;
                    self.view.secondary = secondary;
                    changed = true;
                }
                if brightness != self.view.brightness || speed != self.view.speed || base != self.view.base {
                    if let Ok(mut state) = self.shared.try_lock() {
                        state.engine.brightness = brightness;
                        state.engine.speed = speed;
                        state.engine.base_level = base;
                    }
                    self.view.brightness = brightness;
                    self.view.speed = speed;
                    self.view.base = base;
                    changed = true;
                }
                if custom_on && (palette != self.view.palette || pattern != self.view.pattern) {
                    if let Ok(mut state) = self.shared.try_lock() {
                        state.engine.palette = palette;
                        state.engine.pattern = pattern;
                        state.engine.primary = palette[0];
                        state.engine.secondary = palette[2];
                        self.view.palette = palette;
                        self.view.pattern = pattern;
                    }
                    changed = true;
                }
                if changed {
                    self.remember();
                }
                if custom_on {
                    if let Some(slot) = self.active_custom {
                        let saved_differs = self.customs[slot].as_ref().is_none_or(|saved| {
                            saved.colors.as_slice() != palette.as_slice()
                                || saved.pattern != pattern
                                || (saved.profile.brightness - brightness).abs() > 0.001
                                || (saved.profile.speed - speed).abs() > 0.001
                        });
                        if saved_differs {
                            self.persist_custom(slot, palette, pattern, brightness, speed);
                        }
                    }
                }
                ui.add_space(12.0);
                section_label(ui, "Profiles");
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let (field, _) = ui.allocate_exact_size(Vec2::new(220.0, 38.0), Sense::hover());
                    ui.painter().rect_filled(field, 10.0, FIELD);
                    ui.painter().rect_stroke(
                        field,
                        egui::CornerRadius::same(10),
                        egui::Stroke::new(1.0_f32, LINE),
                        egui::StrokeKind::Inside,
                    );
                    let inner = field.shrink2(Vec2::new(12.0, 4.0));
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(inner)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.profile_name)
                                    .hint_text("Profile name")
                                    .frame(false)
                                    .desired_width(inner.width())
                                    .text_color(PAPER),
                            );
                        },
                    );
                    ui.add_space(14.0);
                    if bar_button(ui, "Save look", 108.0, TEAL, Color32::from_rgb(0x14, 0x20, 0x0c), false).clicked() {
                        self.save_profile();
                    }
                    if bar_button(ui, "Load", 84.0, TEAL, Color32::from_rgb(0x14, 0x20, 0x0c), false).clicked() {
                        self.load_profile();
                    }
                    if bar_button(ui, "Delete", 84.0, FIELD, PAPER, true).clicked() {
                        self.delete_profile();
                    }
                    ui.add_space(10.0);
                    if bar_button(ui, "Effect 1", 92.0, FIELD, PAPER, true).clicked() {
                        self.save_custom(0);
                    }
                    if bar_button(ui, "Effect 2", 92.0, FIELD, PAPER, true).clicked() {
                        self.save_custom(1);
                    }
                });
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Save look stores this keyboard, including the keys you turned on. Select a profile, then Delete to remove it.")
                        .color(MUTED)
                        .size(12.0),
                );
                if self.profiles.is_empty() {
                    ui.label(RichText::new("No saved profiles yet.").color(MUTED).size(12.0));
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                        for name in self.profiles.clone() {
                            if profile_chip(ui, &name, name == self.selected_profile).clicked() {
                                self.selected_profile = name;
                            }
                        }
                    });
                }
                ui.add_space(12.0);
                let startup_fill = if self.run_at_startup { TEAL } else { FIELD };
                let startup_text = if self.run_at_startup {
                    Color32::from_rgb(0x14, 0x20, 0x0c)
                } else {
                    PAPER
                };
                let startup_label = if self.run_at_startup { "Starts with Windows" } else { "Start with Windows" };
                if bar_button(ui, startup_label, 190.0, startup_fill, startup_text, !self.run_at_startup).clicked() {
                    self.run_at_startup = !self.run_at_startup;
                    set_run_at_startup(self.run_at_startup);
                    self.remember();
                }
                ui.add_space(6.0);
                ui.label(RichText::new("Profile used when Windows opens Hydra.").color(MUTED).size(12.0));
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    if profile_chip(ui, "Last look", self.startup_profile.is_empty()).clicked() {
                        self.startup_profile.clear();
                        self.remember();
                    }
                    for name in self.profiles.clone() {
                        if profile_chip(ui, &name, self.startup_profile == name).clicked() {
                            self.startup_profile = name;
                            self.remember();
                        }
                    }
                });
            });
    }

    fn save_custom(&mut self, slot: usize) {
        let (palette, pattern, profile) = {
            let Ok(state) = self.shared.lock() else {
                return;
            };
            let profile = state.engine.snapshot();
            let (palette, pattern) = if state.engine.custom_on {
                (state.engine.palette, state.engine.pattern)
            } else {
                (palette_from(state.engine.primary, state.engine.secondary), CustomPattern::Cycle)
            };
            (palette, pattern, profile)
        };
        let name = {
            let typed = self.profile_name.trim();
            if typed.is_empty() {
                self.customs[slot]
                    .as_ref()
                    .map(|saved| saved.name.clone())
                    .unwrap_or_else(|| format!("Custom {}", slot + 1))
            } else {
                typed.to_string()
            }
        };
        if let Ok(mut state) = self.shared.lock() {
            state.engine.begin_custom(palette, pattern);
            state.message = format!(
                "\"{name}\" is a five-color effect. Edit the colors and pattern under the keyboard."
            );
        }
        self.persist_custom(slot, palette, pattern, profile.brightness, profile.speed);
        if let Some(saved) = self.customs[slot].as_mut() {
            saved.name = name;
        }
        let mut store = read_store();
        store.customs = self.customs.clone();
        write_store(&store);
        self.active_custom = Some(slot);
        self.resume_link = true;
        self.remember();
        let _ = self.cmd.send(Cmd::Connect);
    }

    fn persist_custom(&mut self, slot: usize, palette: [Rgb; 5], pattern: CustomPattern, brightness: f32, speed: f32) {
        let existing = self.customs[slot].clone();
        let name = existing
            .as_ref()
            .map(|saved| saved.name.clone())
            .unwrap_or_else(|| format!("Custom {}", slot + 1));
        let mut profile = if let Some(saved) = existing {
            saved.profile
        } else if let Ok(state) = self.shared.lock() {
            state.engine.snapshot()
        } else {
            EffectEngine::new().snapshot()
        };
        profile.primary = palette[0];
        profile.secondary = palette[2];
        profile.brightness = brightness;
        profile.speed = speed;
        self.customs[slot] = Some(SavedEffect {
            name,
            profile,
            colors: palette.to_vec(),
            pattern,
        });
        let mut store = read_store();
        store.customs = self.customs.clone();
        write_store(&store);
    }

    fn apply_custom(&mut self, slot: usize) {
        let Some(saved) = self.customs[slot].clone() else {
            let created = {
                let Ok(mut state) = self.shared.lock() else {
                    return;
                };
                let palette = palette_from(state.engine.primary, state.engine.secondary);
                let brightness = state.engine.brightness;
                let speed = state.engine.speed;
                state.engine.begin_custom(palette, CustomPattern::Cycle);
                state.message = format!("Custom {} is ready. Set its five colors and a pattern below.", slot + 1);
                (palette, brightness, speed)
            };
            self.persist_custom(slot, created.0, CustomPattern::Cycle, created.1, created.2);
            self.active_custom = Some(slot);
            self.resume_link = true;
            self.remember();
            let _ = self.cmd.send(Cmd::Connect);
            return;
        };
        let palette = if saved.colors.len() == 5 {
            [saved.colors[0], saved.colors[1], saved.colors[2], saved.colors[3], saved.colors[4]]
        } else {
            palette_from(saved.profile.primary, saved.profile.secondary)
        };
        let pattern = if saved.colors.len() == 5 {
            saved.pattern
        } else {
            CustomPattern::Cycle
        };
        self.active_custom = Some(slot);
        if let Ok(mut state) = self.shared.lock() {
            state.engine.brightness = saved.profile.brightness;
            state.engine.speed = saved.profile.speed;
            state.engine.begin_custom(palette, pattern);
            state.message = format!("Playing \"{}\". Change any of its five colors below.", saved.name);
        }
        if saved.colors.len() != 5 {
            self.persist_custom(slot, palette, pattern, saved.profile.brightness, saved.profile.speed);
        }
        self.resume_link = true;
        self.remember();
        let _ = self.cmd.send(Cmd::Connect);
    }

    fn remember(&mut self) {
        let Ok(state) = self.shared.lock() else {
            return;
        };
        let session = Session {
            profile: state.engine.snapshot(),
            custom_on: state.engine.custom_on,
            palette: state.engine.palette.to_vec(),
            pattern: state.engine.pattern,
            active_custom: self.active_custom,
            linked: self.resume_link,
            run_at_startup: self.run_at_startup,
            startup_profile: self.startup_profile.clone(),
        };
        drop(state);
        let mut store = read_store();
        store.customs = self.customs.clone();
        store.session = session;
        write_store(&store);
    }

    fn save_profile(&mut self) {
        let typed = self.profile_name.trim().to_string();
        let (snapshot, custom_on) = {
            let Ok(state) = self.shared.lock() else {
                return;
            };
            (state.engine.snapshot(), state.engine.custom_on)
        };
        let name = if !typed.is_empty() {
            typed
        } else if custom_on {
            self.active_custom
                .and_then(|slot| self.customs[slot].as_ref().map(|saved| saved.name.clone()))
                .unwrap_or_else(|| "Custom".into())
        } else {
            snapshot.mode.name().to_string()
        };
        let glowing = if snapshot.mode == Mode::Paint { snapshot.paint.len() } else { 0 };
        let mut store = read_store();
        store.items.insert(name.clone(), snapshot);
        store.current = name.clone();
        write_store(&store);
        self.profile_name = name.clone();
        self.profiles = store.items.keys().cloned().collect();
        self.selected_profile = name.clone();
        self.remember();
        if let Ok(mut state) = self.shared.lock() {
            state.message = if glowing > 0 {
                format!("Saved \"{name}\" with {glowing} glowing keys. It is listed under Profiles.")
            } else {
                format!("Saved \"{name}\". It is listed under Profiles.")
            };
        }
    }

    fn load_profile(&mut self) {
        if self.selected_profile.is_empty() {
            return;
        }
        let store = read_store();
        let Some(profile) = store.items.get(&self.selected_profile).cloned() else {
            return;
        };
        let mut store = store;
        let name = self.selected_profile.clone();
        store.current = name.clone();
        write_store(&store);
        self.active_custom = None;
        if let Ok(mut state) = self.shared.lock() {
            state.engine.apply(profile);
            state.message = format!("Loaded profile \"{name}\".");
        }
        self.resume_link = true;
        self.remember();
        let _ = self.cmd.send(Cmd::Connect);
    }

    fn delete_profile(&mut self) {
        let name = self.selected_profile.trim().to_string();
        if name.is_empty() {
            if let Ok(mut state) = self.shared.lock() {
                state.message = "Select a saved profile, then press Delete.".into();
            }
            return;
        }
        let mut store = read_store();
        if store.items.remove(&name).is_none() {
            if let Ok(mut state) = self.shared.lock() {
                state.message = format!("\"{name}\" is not a saved profile.");
            }
            return;
        }
        if store.current == name {
            store.current.clear();
        }
        write_store(&store);
        self.profiles.retain(|saved| saved != &name);
        self.selected_profile.clear();
        if self.startup_profile == name {
            self.startup_profile.clear();
            self.remember();
        }
        if self.profile_name == name {
            self.profile_name.clear();
        }
        if let Ok(mut state) = self.shared.lock() {
            state.message = format!("Deleted profile \"{name}\".");
        }
    }
}

impl HydraWindow {
    fn ensure_tray(&mut self, ctx: &egui::Context) {
        install_tray_handlers(ctx.clone());
        if self.tray.is_some() {
            return;
        }
        let Some(icon) = tray_icon_image() else {
            return;
        };
        let open = tray_icon::menu::MenuItem::with_id("hydra-open", "Open", true, None);
        let quit = tray_icon::menu::MenuItem::with_id("hydra-quit", "Quit", true, None);
        let menu = tray_icon::menu::Menu::new();
        if menu.append_items(&[&open, &quit]).is_err() {
            return;
        }
        self.tray = tray_icon::TrayIconBuilder::new()
            .with_tooltip("Hydra RGB")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .ok();
    }

    fn show_window(&mut self, ctx: &egui::Context) {
        self.in_tray = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
}

static TRAY_CTX: std::sync::OnceLock<egui::Context> = std::sync::OnceLock::new();
static TRAY_SHOW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static TRAY_QUIT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static TRAY_HANDLERS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn install_tray_handlers(ctx: egui::Context) {
    let _ = TRAY_CTX.set(ctx);
    if TRAY_HANDLERS.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    tray_icon::menu::MenuEvent::set_event_handler(Some(|event: tray_icon::menu::MenuEvent| match event.id.0.as_str() {
        "hydra-quit" => request_tray_quit(),
        "hydra-open" => show_from_tray(),
        _ => {}
    }));
    tray_icon::TrayIconEvent::set_event_handler(Some(|event: tray_icon::TrayIconEvent| match event {
        tray_icon::TrayIconEvent::Click {
            button: tray_icon::MouseButton::Left,
            button_state: tray_icon::MouseButtonState::Up,
            ..
        }
        | tray_icon::TrayIconEvent::DoubleClick {
            button: tray_icon::MouseButton::Left,
            ..
        } => show_from_tray(),
        _ => {}
    }));
}

fn request_tray_quit() {
    if TRAY_QUIT.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    if let Some(ctx) = TRAY_CTX.get() {
        ctx.request_repaint();
    }
    // The click is handled on the tray window, which can be hidden from the
    // app window. If that window never paints again, still leave.
    thread::spawn(|| {
        thread::sleep(Duration::from_millis(400));
        std::process::exit(0);
    });
}

fn show_from_tray() {
    TRAY_SHOW.store(true, std::sync::atomic::Ordering::SeqCst);
    let Some(ctx) = TRAY_CTX.get() else {
        return;
    };
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.request_repaint();
}

fn tray_show_requested() -> bool {
    TRAY_SHOW.swap(false, std::sync::atomic::Ordering::SeqCst)
}

fn tray_quit_requested() -> bool {
    TRAY_QUIT.load(std::sync::atomic::Ordering::SeqCst)
}

fn set_run_at_startup(enabled: bool) {
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let mut command = std::process::Command::new("reg");
    if enabled {
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        let value = format!("\"{}\" --startup", exe.display());
        command.args(["add", key, "/v", "Hydra RGB", "/t", "REG_SZ", "/d", &value, "/f"]);
    } else {
        command.args(["delete", key, "/v", "Hydra RGB", "/f"]);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let _ = command.status();
}

fn tray_icon_image() -> Option<tray_icon::Icon> {
    let data = app_icon();
    tray_icon::Icon::from_rgba(data.rgba.clone(), data.width, data.height).ok()
}

fn install_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.button_padding = Vec2::new(12.0, 8.0);
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.interact_size = Vec2::new(36.0, 32.0);
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = INK;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = FIELD;
    style.visuals.faint_bg_color = FIELD;
    style.visuals.code_bg_color = FIELD;
    style.visuals.override_text_color = Some(PAPER);
    style.visuals.selection.bg_fill = Color32::from_rgb(0x2a, 0x38, 0x22);
    style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, TEAL);
    style.visuals.widgets.noninteractive = widget_look(PANEL, LINE, MUTED, 8);
    style.visuals.widgets.inactive = widget_look(FIELD, LINE, PAPER, 8);
    style.visuals.widgets.hovered = widget_look(Color32::from_rgb(0x24, 0x30, 0x1c), TEAL, PAPER, 8);
    style.visuals.widgets.active = widget_look(Color32::from_rgb(0x2e, 0x3c, 0x24), TEAL, PAPER, 8);
    style.visuals.widgets.open = style.visuals.widgets.active;
    ctx.set_style(style);
}

fn widget_look(fill: Color32, stroke: Color32, text: Color32, radius: u8) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: egui::Stroke::new(1.0_f32, stroke),
        corner_radius: egui::CornerRadius::same(radius),
        fg_stroke: egui::Stroke::new(1.0_f32, text),
        expansion: 0.0,
    }
}

fn arrow_direction(id: &str) -> Option<u8> {
    match id {
        "up" => Some(0),
        "right" => Some(1),
        "down" => Some(2),
        "left" => Some(3),
        _ => None,
    }
}

fn paint_arrow(painter: &egui::Painter, rect: egui::Rect, direction: u8, color: Color32) {
    let center = rect.center();
    let length = (rect.height() * 0.22).clamp(7.0, 12.0);
    let wing = length * 0.62;
    let stroke = egui::Stroke::new((rect.height() * 0.045).clamp(1.6, 2.4), color);
    let (tip, tail, left, right) = match direction {
        0 => {
            let tip = center + Vec2::new(0.0, -length);
            (tip, center + Vec2::new(0.0, length), tip + Vec2::new(-wing, wing), tip + Vec2::new(wing, wing))
        }
        1 => {
            let tip = center + Vec2::new(length, 0.0);
            (tip, center + Vec2::new(-length, 0.0), tip + Vec2::new(-wing, -wing), tip + Vec2::new(-wing, wing))
        }
        2 => {
            let tip = center + Vec2::new(0.0, length);
            (tip, center + Vec2::new(0.0, -length), tip + Vec2::new(-wing, -wing), tip + Vec2::new(wing, -wing))
        }
        _ => {
            let tip = center + Vec2::new(-length, 0.0);
            (tip, center + Vec2::new(length, 0.0), tip + Vec2::new(wing, -wing), tip + Vec2::new(wing, wing))
        }
    };
    painter.line_segment([tail, tip], stroke);
    painter.line_segment([left, tip], stroke);
    painter.line_segment([right, tip], stroke);
}

fn title_bar(ctx: &egui::Context) {
    egui::TopBottomPanel::top("title-bar")
        .exact_height(36.0)
        .frame(egui::Frame::new().fill(INK))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let button_w = 46.0;
            let close_rect = egui::Rect::from_min_max(egui::pos2(rect.right() - button_w, rect.top()), rect.right_bottom());
            let max_rect = close_rect.translate(Vec2::new(-button_w, 0.0));
            let min_rect = max_rect.translate(Vec2::new(-button_w, 0.0));
            let drag_rect = egui::Rect::from_min_max(rect.min, egui::pos2(min_rect.left(), rect.bottom()));
            let drag = ui.interact(drag_rect, ui.id().with("frame-drag"), Sense::click_and_drag());
            if drag.double_clicked() {
                let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            } else if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            let mark = egui::Rect::from_min_size(rect.min + Vec2::new(12.0, 9.0), Vec2::splat(18.0));
            paint_mark(ui.painter(), mark);
            ui.painter().text(
                egui::pos2(mark.right() + 8.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                "HYDRA",
                egui::FontId::proportional(13.0),
                PAPER,
            );

            let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
            if caption_button(ui, "min", min_rect, Color32::from_rgb(0x24, 0x2c, 0x22), |painter, center| {
                painter.line_segment(
                    [center + Vec2::new(-5.0, 0.0), center + Vec2::new(5.0, 0.0)],
                    egui::Stroke::new(1.4_f32, PAPER),
                );
            })
            .clicked()
            {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            if caption_button(ui, "max", max_rect, Color32::from_rgb(0x24, 0x2c, 0x22), |painter, center| {
                let box_rect = egui::Rect::from_center_size(center, Vec2::splat(9.0));
                painter.rect_stroke(box_rect, 0.0, egui::Stroke::new(1.3_f32, PAPER), egui::StrokeKind::Outside);
                if maximized {
                    let back = box_rect.translate(Vec2::new(-2.0, 2.0));
                    painter.rect_stroke(back, 0.0, egui::Stroke::new(1.3_f32, PAPER), egui::StrokeKind::Outside);
                }
            })
            .clicked()
            {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            if caption_button(ui, "close", close_rect, MARK, |painter, center| {
                let span = 4.5;
                let stroke = egui::Stroke::new(1.4_f32, PAPER);
                painter.line_segment([center + Vec2::new(-span, -span), center + Vec2::new(span, span)], stroke);
                painter.line_segment([center + Vec2::new(span, -span), center + Vec2::new(-span, span)], stroke);
            })
            .clicked()
            {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    resize_edges(ctx);
}

fn caption_button(
    ui: &mut egui::Ui,
    id: &str,
    rect: egui::Rect,
    hover: Color32,
    paint: impl FnOnce(&egui::Painter, egui::Pos2),
) -> egui::Response {
    let response = ui.interact(rect, ui.id().with(id), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, hover);
    }
    paint(ui.painter(), rect.center());
    response
}

fn resize_edges(ctx: &egui::Context) {
    if ctx.input(|input| input.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let screen = ctx.screen_rect();
    let thickness = 5.0;
    let edges = [
        (egui::ResizeDirection::West, egui::Rect::from_min_max(screen.min, egui::pos2(screen.left() + thickness, screen.bottom()))),
        (egui::ResizeDirection::East, egui::Rect::from_min_max(egui::pos2(screen.right() - thickness, screen.top()), screen.max)),
        (egui::ResizeDirection::South, egui::Rect::from_min_max(egui::pos2(screen.left(), screen.bottom() - thickness), screen.max)),
        (egui::ResizeDirection::SouthWest, egui::Rect::from_min_size(egui::pos2(screen.left(), screen.bottom() - thickness), Vec2::splat(thickness))),
        (egui::ResizeDirection::SouthEast, egui::Rect::from_min_size(screen.max - Vec2::splat(thickness), Vec2::splat(thickness))),
    ];
    egui::Area::new(egui::Id::new("resize-edges"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.set_clip_rect(screen);
            for (direction, rect) in edges {
                let response = ui.interact(rect, ui.id().with(format!("{direction:?}")), Sense::click_and_drag());
                if response.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                }
            }
        });
}

fn brand_lockup(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        let (tile, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
        paint_mark(ui.painter(), tile);
        ui.vertical(|ui| {
            ui.add_space(2.0);
            let mut job = egui::text::LayoutJob::default();
            job.append(
                "HYDRA",
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(30.0),
                    extra_letter_spacing: 2.0,
                    color: PAPER,
                    ..Default::default()
                },
            );
            let galley = ui.fonts(|fonts| fonts.layout_job(job));
            let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
            ui.painter().galley(rect.min + Vec2::new(0.6, 0.0), galley.clone(), PAPER);
            ui.painter().galley(rect.min, galley, PAPER);
            ui.label(RichText::new("CUSTOM LIGHTING").color(MUTED).size(11.0));
        });
    });
}

fn paint_mark(painter: &egui::Painter, tile: egui::Rect) {
    painter.rect_filled(tile, 10.0, MARK);
    let stem = tile.width() * 0.16;
    let pad = tile.width() * 0.24;
    let inner = tile.shrink(pad);
    let white = Color32::WHITE;
    painter.rect_filled(
        egui::Rect::from_min_size(inner.min, Vec2::new(stem, inner.height())),
        2.0,
        white,
    );
    painter.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(inner.right() - stem, inner.top()),
            Vec2::new(stem, inner.height()),
        ),
        2.0,
        white,
    );
    let bar = egui::Rect::from_center_size(inner.center(), Vec2::new(inner.width(), stem));
    painter.rect_filled(bar, 2.0, white);
}

fn app_icon() -> std::sync::Arc<egui::IconData> {
    let size = 64u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let radius = 14.0;
    for y in 0..size {
        for x in 0..size {
            let cover = round_rect_cover(x as f32, y as f32, size as f32, radius);
            if cover <= 0.0 {
                continue;
            }
            let on_letter = mark_letter(x as f32 + 0.5, y as f32 + 0.5, size as f32);
            let (r, g, b) = if on_letter { (255, 255, 255) } else { (0xea, 0x0b, 0x2a) };
            let i = ((y * size + x) * 4) as usize;
            let alpha = (cover * 255.0).round() as u8;
            rgba[i] = r;
            rgba[i + 1] = g;
            rgba[i + 2] = b;
            rgba[i + 3] = alpha;
        }
    }
    std::sync::Arc::new(egui::IconData {
        rgba,
        width: size,
        height: size,
    })
}

fn round_rect_cover(x: f32, y: f32, size: f32, radius: f32) -> f32 {
    let half = size * 0.5;
    let px = (x + 0.5 - half).abs();
    let py = (y + 0.5 - half).abs();
    let box_edge = half - radius;
    let dx = (px - box_edge).max(0.0);
    let dy = (py - box_edge).max(0.0);
    let outside = (dx * dx + dy * dy).sqrt() - radius;
    (0.75 - outside).clamp(0.0, 1.0)
}

fn mark_letter(x: f32, y: f32, size: f32) -> bool {
    let pad = size * 0.24;
    let stem = size * 0.16;
    let left = pad;
    let right = size - pad;
    let top = pad;
    let bottom = size - pad;
    let in_y = y >= top && y <= bottom;
    let left_stem = x >= left && x <= left + stem && in_y;
    let right_stem = x >= right - stem && x <= right && in_y;
    let mid = size * 0.5;
    let cross = y >= mid - stem * 0.5 && y <= mid + stem * 0.5 && x >= left && x <= right;
    left_stem || right_stem || cross
}

fn usable_width(ui: &egui::Ui) -> f32 {
    let width = ui.available_width();
    if width.is_finite() { width } else { 280.0 }
}

fn filled_button(ui: &mut egui::Ui, text: &str, fill: Color32, fg: Color32) -> egui::Response {
    bar_button(ui, text, usable_width(ui), fill, fg, false)
}

fn ghost_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let width = usable_width(ui).max(72.0);
    bar_button(ui, text, width, FIELD, PAPER, true)
}

fn bar_button(ui: &mut egui::Ui, text: &str, width: f32, fill: Color32, fg: Color32, stroke: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 38.0), Sense::click());
    let mut color = fill;
    if response.hovered() {
        color = Color32::from_rgb(
            color.r().saturating_add(12),
            color.g().saturating_add(12),
            color.b().saturating_add(12),
        );
    }
    ui.painter().rect_filled(rect, 10.0, color);
    if stroke {
        ui.painter().rect_stroke(rect, egui::CornerRadius::same(10), egui::Stroke::new(1.0_f32, LINE), egui::StrokeKind::Inside);
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(14.0),
        fg,
    );
    response
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(text.to_uppercase()).color(MUTED).size(11.0).strong());
}

fn effect_row(ui: &mut egui::Ui, name: &str, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(usable_width(ui), 30.0), Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(rect, 8.0, Color32::from_rgb(0x22, 0x2c, 0x1e));
    }
    if selected {
        let mark = egui::Rect::from_min_size(rect.min + Vec2::new(0.0, 6.0), Vec2::new(3.0, rect.height() - 12.0));
        ui.painter().rect_filled(mark, 2.0, TEAL);
    }
    ui.painter().text(
        rect.left_center() + Vec2::new(14.0, 0.0),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(14.0),
        if selected { PAPER } else { MUTED },
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn spectrum_note(ui: &mut egui::Ui) {
    let hues = [
        Color32::from_rgb(255, 48, 48),
        Color32::from_rgb(255, 150, 32),
        Color32::from_rgb(236, 220, 48),
        Color32::from_rgb(70, 210, 80),
        Color32::from_rgb(40, 214, 186),
        Color32::from_rgb(56, 120, 255),
        Color32::from_rgb(168, 72, 255),
        Color32::from_rgb(255, 56, 150),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for hue in hues {
            let (chip, _) = ui.allocate_exact_size(Vec2::new(22.0, 36.0), Sense::hover());
            ui.painter().rect_filled(chip, 8.0, hue);
        }
    });
    ui.add_space(8.0);
    ui.set_max_width(280.0);
    ui.label(
        RichText::new("Rainbow carries these colors itself. There is no picker. Brightness and speed are the controls.")
            .color(PAPER)
            .size(12.0),
    );
}

fn color_swatch(ui: &mut egui::Ui, id: &str, caption: &str, color: &mut Rgb) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(42.0), Sense::click());
        let fill = Color32::from_rgb(color[0], color[1], color[2]);
        ui.painter().rect_filled(rect, 12.0, fill);
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(12),
            egui::Stroke::new(1.0_f32, Color32::from_white_alpha(48)),
            egui::StrokeKind::Inside,
        );
        let popup_id = ui.make_persistent_id(("swatch", id));
        if response.clicked() {
            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
        }
        if ui.memory(|mem| mem.is_popup_open(popup_id)) {
            let area = egui::Area::new(popup_id)
                .kind(egui::UiKind::Popup)
                .order(egui::Order::Foreground)
                .pivot(egui::Align2::LEFT_BOTTOM)
                .fixed_pos(rect.left_top() + Vec2::new(0.0, -8.0))
                .show(ui.ctx(), |ui| {
                    egui::Frame::new()
                        .fill(PANEL)
                        .stroke(egui::Stroke::new(1.0_f32, LINE))
                        .corner_radius(12)
                        .inner_margin(12)
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 10],
                            blur: 24,
                            spread: 0,
                            color: Color32::from_black_alpha(140),
                        })
                        .show(ui, |ui| dark_picker(ui, popup_id, color));
                });
            if response.clicked_elsewhere() && area.response.clicked_elsewhere() {
                ui.memory_mut(|mem| mem.close_popup());
            }
        }
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.label(RichText::new(caption).color(MUTED).size(11.0));
            ui.label(
                RichText::new(format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]))
                    .color(PAPER)
                    .size(13.0),
            );
        });
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    });
}

fn dark_picker(ui: &mut egui::Ui, id: egui::Id, color: &mut Rgb) {
    let from_color = rgb_to_hsv(*color);
    let mut hsv = ui.ctx().data(|data| data.get_temp::<[f32; 3]>(id)).unwrap_or(from_color);
    let shown = hsv_to_rgb(hsv[0], hsv[1], hsv[2]);
    if shown != *color && from_color[1] > 0.02 {
        hsv = from_color;
    }

    let side = 168.0;
    let (square, square_drag) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
    paint_saturation(ui.painter(), square, hsv[0]);
    if let Some(pointer) = square_drag.interact_pointer_pos() {
        hsv[1] = ((pointer.x - square.left()) / square.width()).clamp(0.0, 1.0);
        hsv[2] = (1.0 - (pointer.y - square.top()) / square.height()).clamp(0.0, 1.0);
    }
    let handle = egui::pos2(
        square.left() + hsv[1] * square.width(),
        square.top() + (1.0 - hsv[2]) * square.height(),
    );
    let current = hsv_to_rgb(hsv[0], hsv[1], hsv[2]);
    ui.painter().circle_filled(handle, 6.0, Color32::from_rgb(current[0], current[1], current[2]));
    ui.painter().circle_stroke(handle, 7.0, egui::Stroke::new(2.0_f32, Color32::WHITE));

    ui.add_space(10.0);
    let (hue_bar, hue_drag) = ui.allocate_exact_size(Vec2::new(side, 12.0), Sense::click_and_drag());
    paint_hue(ui.painter(), hue_bar);
    if let Some(pointer) = hue_drag.interact_pointer_pos() {
        hsv[0] = ((pointer.x - hue_bar.left()) / hue_bar.width()).clamp(0.0, 0.999);
    }
    let marker = egui::Rect::from_center_size(
        egui::pos2(hue_bar.left() + hsv[0] * hue_bar.width(), hue_bar.center().y),
        Vec2::new(4.0, hue_bar.height() + 6.0),
    );
    ui.painter().rect_filled(marker, 2.0, PAPER);
    ui.painter().rect_stroke(marker, egui::CornerRadius::same(2), egui::Stroke::new(1.0_f32, Color32::BLACK), egui::StrokeKind::Outside);

    *color = hsv_to_rgb(hsv[0], hsv[1], hsv[2]);
    ui.add_space(8.0);
    ui.label(RichText::new(format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])).color(PAPER).size(13.0));
    ui.ctx().data_mut(|data| data.insert_temp(id, hsv));
}

fn paint_saturation(painter: &egui::Painter, rect: egui::Rect, hue: f32) {
    let steps = 16;
    let mut mesh = egui::Mesh::default();
    for y in 0..steps {
        for x in 0..steps {
            let mut index = [0u32; 4];
            for (slot, (cx, cy)) in [(x, y), (x + 1, y), (x + 1, y + 1), (x, y + 1)].into_iter().enumerate() {
                let saturation = cx as f32 / steps as f32;
                let value = 1.0 - cy as f32 / steps as f32;
                let rgb = hsv_to_rgb(hue, saturation, value);
                index[slot] = mesh.vertices.len() as u32;
                mesh.colored_vertex(
                    egui::pos2(rect.left() + saturation * rect.width(), rect.top() + (1.0 - value) * rect.height()),
                    Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                );
            }
            mesh.add_triangle(index[0], index[1], index[2]);
            mesh.add_triangle(index[0], index[2], index[3]);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn paint_hue(painter: &egui::Painter, rect: egui::Rect) {
    let steps = 24;
    let mut mesh = egui::Mesh::default();
    for x in 0..steps {
        let mut index = [0u32; 4];
        for (slot, cx) in [x, x + 1, x + 1, x].into_iter().enumerate() {
            let hue = cx as f32 / steps as f32;
            let y = if slot < 2 { 0.0 } else { 1.0 };
            let rgb = hsv_to_rgb(hue.min(0.999), 1.0, 1.0);
            index[slot] = mesh.vertices.len() as u32;
            mesh.colored_vertex(
                egui::pos2(rect.left() + hue.min(1.0) * rect.width(), rect.top() + y * rect.height()),
                Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
            );
        }
        mesh.add_triangle(index[0], index[1], index[2]);
        mesh.add_triangle(index[0], index[2], index[3]);
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn rgb_to_hsv(color: Rgb) -> [f32; 3] {
    let r = color[0] as f32 / 255.0;
    let g = color[1] as f32 / 255.0;
    let b = color[2] as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };
    let saturation = if max == 0.0 { 0.0 } else { delta / max };
    [hue, saturation, max]
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> Rgb {
    let hue = hue.rem_euclid(1.0);
    let sector = (hue * 6.0).floor();
    let f = hue * 6.0 - sector;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - f * saturation);
    let t = value * (1.0 - (1.0 - f) * saturation);
    let (r, g, b) = match sector as i32 % 6 {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

fn meter(ui: &mut egui::Ui, caption: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, width: f32) {
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.horizontal(|ui| {
            ui.label(RichText::new(caption).color(MUTED).size(12.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("{:.0}", *value * 100.0)).color(PAPER).size(12.0));
            });
        });
        ui.add_space(8.0);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 22.0), Sense::click_and_drag());
        if let Some(pointer) = response.interact_pointer_pos() {
            let t = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            *value = range.start() + t * (range.end() - range.start());
        }
        let span = (range.end() - range.start()).max(0.0001);
        let t = ((*value - range.start()) / span).clamp(0.0, 1.0);
        let track = egui::Rect::from_center_size(rect.center(), Vec2::new(rect.width(), 4.0));
        ui.painter().rect_filled(track, 2.0, Color32::from_rgb(0x2a, 0x32, 0x28));
        let filled = egui::Rect::from_min_size(track.min, Vec2::new((track.width() * t).max(4.0), track.height()));
        ui.painter().rect_filled(filled, 2.0, TEAL);
        let thumb = egui::pos2(track.left() + track.width() * t, track.center().y);
        ui.painter().circle_filled(thumb, 7.0, PAPER);
        ui.painter().circle_stroke(thumb, 7.0, egui::Stroke::new(2.0_f32, TEAL));
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
    });
}

fn profile_chip(ui: &mut egui::Ui, name: &str, selected: bool) -> egui::Response {
    let width = (name.len() as f32 * 7.4 + 28.0).clamp(72.0, 200.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
    let fill = if selected { Color32::from_rgb(0x22, 0x2c, 0x1e) } else { FIELD };
    ui.painter().rect_filled(rect, 8.0, fill);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(8),
        egui::Stroke::new(1.0_f32, if selected { TEAL } else { LINE }),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        name,
        egui::FontId::proportional(13.0),
        if selected { PAPER } else { MUTED },
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn ink(color: Rgb) -> Color32 {
    let luma = 0.299 * color[0] as f32 + 0.587 * color[1] as f32 + 0.114 * color[2] as f32;
    if luma > 150.0 {
        Color32::from_rgb(0x1c, 0x24, 0x16)
    } else {
        PAPER
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([1040.0, 700.0])
            .with_title("Hydra RGB")
            .with_icon(app_icon())
            .with_decorations(false)
            .with_taskbar(false)
            .with_transparent(false),
        renderer: eframe::Renderer::Wgpu,
        hardware_acceleration: eframe::HardwareAcceleration::Required,
        vsync: false,
        ..Default::default()
    };
    eframe::run_native(
        "Hydra RGB",
        options,
        Box::new(|cc| Ok(Box::new(HydraWindow::new(cc)))),
    )
}
