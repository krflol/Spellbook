//! Native Spellbook application. The editor never executes macro or Lua text.
mod macro_editor;
mod macros_view;
mod widgets;

use crate::{
    macros::{catalogue::CatalogueFile, language::Catalogue},
    session::{self, Session},
};
use anyhow::{Context, Result};
use eframe::egui::{self, Color32, FontId, Key, Modifiers, RichText};
use macros_view::MacrosView;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

const BG: Color32 = Color32::from_rgb(17, 22, 28);
const PANEL: Color32 = Color32::from_rgb(23, 30, 38);
const CARD: Color32 = Color32::from_rgb(30, 39, 48);
const GOLD: Color32 = Color32::from_rgb(225, 188, 112);
const INK: Color32 = Color32::from_rgb(227, 230, 230);
const MUTED: Color32 = Color32::from_rgb(151, 166, 177);
const TEAL: Color32 = Color32::from_rgb(100, 207, 185);

struct Capture {
    dir: PathBuf,
    step: usize,
    since: Instant,
    pending: bool,
    done: Arc<AtomicBool>,
}
struct Spellbook {
    view: MacrosView,
    data_dir: PathBuf,
    settings: bool,
    about: bool,
    notice: Option<String>,
    last_session: Session,
    last_save: Instant,
    capture: Option<Capture>,
}

impl Spellbook {
    fn persist_session(&mut self, ctx: &egui::Context) -> Result<()> {
        let size = ctx
            .input(|i| i.viewport().inner_rect.map(|r| [r.width(), r.height()]))
            .unwrap_or(self.last_session.window_size);
        let state = self.view.session(size);
        if state != self.last_session || !self.data_dir.join("session.json").exists() {
            state.save(&self.data_dir.join("session.json"))?;
            self.last_session = state;
        }
        self.last_save = Instant::now();
        Ok(())
    }
    fn menu(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::O)) {
            self.view.choose_import();
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S)) {
            self.view.choose_export(true);
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F1)) {
            self.about = true;
        }
        egui::Panel::top("main-menu")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(12, 3)),
            )
            .show(ui, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("New macro       Ctrl+N").clicked() {
                            self.view.new_macro();
                            ui.close();
                        }
                        if ui.button("Import files…    Ctrl+O").clicked() {
                            self.view.choose_import();
                            ui.close();
                        }
                        if ui.button("Import / export text…").clicked() {
                            self.view.show_transfer();
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Save all           Ctrl+S").clicked() {
                            self.view.save_if_dirty();
                            ui.close();
                        }
                        if ui.button("Export library…  Ctrl+Shift+S").clicked() {
                            self.view.choose_export(true);
                            ui.close();
                        }
                        if ui.button("Export current macro…").clicked() {
                            self.view.choose_export(false);
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Exit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button("Workspace", |ui| {
                        let narrow = ctx.viewport_rect().width() < 1120.0;
                        if ui.button("Sample library").clicked() {
                            self.view.show_samples(narrow);
                            ui.close();
                        }
                        if ui.button("Language reference").clicked() {
                            self.view.show_reference(narrow);
                            ui.close();
                        }
                        if ui.button("Settings…").clicked() {
                            self.settings = true;
                            ui.close();
                        }
                    });
                    if ui.button("Help").clicked() {
                        self.about = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("WoW Forever / Classic-style macros")
                                .size(10.0)
                                .color(MUTED),
                        );
                        ui.label(RichText::new("LOCAL").size(10.0).color(TEAL));
                    });
                });
            });
    }
    fn settings(&mut self, ctx: &egui::Context) {
        let mut open = self.settings;
        egui::Window::new("Settings").open(&mut open).anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO).default_width(470.0).resizable(false).show(ctx,|ui| {
            widgets::caption(ui,"EDITOR");
            let (mut font_size,mut auto_pairs) = self.view.preferences();
            let changed=ui.add(egui::Slider::new(&mut font_size,11.0..=28.0).text("Code size")).changed()
                | ui.checkbox(&mut auto_pairs,"Automatically pair brackets and quotes").changed();
            if changed {self.view.set_preferences(font_size,auto_pairs);}
            ui.add_space(12.0);ui.separator();widgets::caption(ui,"AUTOCOMPLETE");
            ui.label("Bundled English spell and item names work offline. Import a catalogue to add client-specific or localized names.");
            ui.label(RichText::new("Names are suggestions; they do not check your character's spells or talents.").small().color(MUTED));
            ui.horizontal(|ui| {
                if ui.button("Import catalogue…").on_hover_text("Replaces your custom catalogue; bundled names remain available").clicked() {
                    if let Some(path)=rfd::FileDialog::new().set_title("Import autocomplete catalogue").add_filter("Catalogue JSON", &["json"]).pick_file() {
                        let result=(|| -> Result<()> {
                            let custom=CatalogueFile::read(&path)?;
                            session::write_atomic(&self.data_dir.join("catalogue.json"),serde_json::to_string_pretty(&custom)?.as_bytes())?;
                            let mut catalogue=Catalogue::bundled();catalogue.merge(&custom);self.view.set_catalogue(catalogue);
                            Ok(())
                        })();
                        self.notice=Some(match result {Ok(())=>"Custom autocomplete catalogue installed.".into(),Err(e)=>format!("Catalogue import failed: {e:#}")});
                    }
                }
                if ui.button("Reset to bundled names").clicked() {
                    let path=self.data_dir.join("catalogue.json");
                    match if path.exists(){std::fs::remove_file(path)}else{Ok(())} {
                        Ok(())=>{self.view.set_catalogue(Catalogue::bundled());self.notice=Some("Bundled autocomplete restored.".into());},
                        Err(e)=>self.notice=Some(format!("Could not reset catalogue: {e}")),
                    }
                }
            });
            ui.hyperlink_to("Catalogue format and example","https://github.com/krflol/Spellbook/blob/main/docs/CATALOGUES.md");
            ui.add_space(12.0);ui.separator();widgets::caption(ui,"YOUR DATA");
            ui.label("Macros and revision history stay in your local library. Export the library from File to share or back it up.");
            ui.add(egui::Label::new(RichText::new(self.data_dir.display().to_string()).monospace().small()).wrap());
            if ui.button("Copy data folder path").clicked(){ctx.copy_text(self.data_dir.display().to_string());}
            ui.add_space(8.0);
            ui.label(RichText::new("No account, network connection, game installation, or Atlas database required.").small().color(TEAL));
        });
        self.settings = open;
    }
    fn about(&mut self, ctx: &egui::Context) {
        egui::Window::new("About Spellbook").open(&mut self.about).anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO).default_width(485.0).resizable(false).show(ctx,|ui| {
            ui.heading(RichText::new("Spellbook").color(GOLD));
            ui.label(format!("Version {} · MIT license",env!("CARGO_PKG_VERSION")));
            ui.label("A dedicated workspace for World of Warcraft macros. Write, organize, refine, and copy your macros into the game.");
            ui.add_space(12.0);
            for (keys,action) in [
                ("Ctrl+Space","Contextual autocomplete"),("Tab / Enter","Accept suggestion"),("Esc","Dismiss suggestions or find"),
                ("Ctrl+F / Ctrl+H","Find / replace"),("Ctrl+G","Go to line"),("Ctrl+D","Duplicate line"),
                ("Ctrl+Z / Ctrl+Shift+Z","Undo / redo"),("Ctrl+N / Ctrl+W","New macro / close tab"),
                ("Ctrl+O","Import files"),("Ctrl+S","Save all macros"),("Ctrl+Shift+S","Export library"),
            ] {ui.horizontal(|ui| {ui.add_sized([190.0,18.0],egui::Label::new(RichText::new(keys).monospace().color(GOLD)));ui.label(action);});}
            ui.separator();
            ui.label("Problems checks structure, not gameplay. Samples target Forever / Classic-style mechanics. Check learned form slots with Inspect forms and adapt the examples to your client.");
            ui.label(RichText::new("Independent community software. Not affiliated with or endorsed by Blizzard Entertainment. World of Warcraft is a Blizzard trademark.").small().color(MUTED));
            ui.horizontal(|ui| {
                ui.hyperlink_to("Source & releases","https://github.com/krflol/Spellbook");
                ui.hyperlink_to("User guide","https://github.com/krflol/Spellbook/blob/main/docs/GUIDE.md");
                ui.hyperlink_to("Report an issue","https://github.com/krflol/Spellbook/issues");
            });
        });
    }
    fn capture(&mut self, ctx: &egui::Context) {
        let Some(c) = &mut self.capture else { return };
        const SCREENS: &[&str] = &[
            "welcome",
            "editor",
            "autocomplete",
            "druid-samples",
            "diagnostics",
            "history",
            "compact",
            "settings",
        ];
        if c.step >= SCREENS.len() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if c.pending {
            if !c.done.swap(false, Ordering::SeqCst) {
                ctx.request_repaint_after(Duration::from_millis(80));
                return;
            }
            c.pending = false;
            c.step += 1;
            if c.step >= SCREENS.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            self.view.prepare_capture(c.step);
            self.settings = c.step == 7;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(if c.step == 6 {
                egui::vec2(1000.0, 760.0)
            } else {
                egui::vec2(1480.0, 940.0)
            }));
            c.since = Instant::now();
            ctx.request_repaint_after(Duration::from_millis(250));
            return;
        }
        if c.since.elapsed() < Duration::from_millis(700) {
            ctx.request_repaint_after(Duration::from_millis(150));
            return;
        }
        let path = c.dir.join(format!("{}.png", SCREENS[c.step]));
        let done = c.done.clone();
        ctx.request_screenshot(move |image| {
            let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            if let Err(e) = image::save_buffer(
                &path,
                &pixels,
                image.width() as u32,
                image.height() as u32,
                image::ColorType::Rgba8,
            ) {
                let _ = std::fs::write(path.with_extension("error.txt"), e.to_string());
            }
            done.store(true, Ordering::SeqCst);
        });
        c.pending = true;
    }
}

impl eframe::App for Spellbook {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.view.poll(&ctx);
        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.view.save_if_dirty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
            if let Err(e) = self.persist_session(&ctx) {
                self.notice = Some(format!("Could not save workspace settings: {e:#}"));
            }
        }
        for path in ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect::<Vec<_>>()
        }) {
            self.notice = Some(match self.view.import_path(&path) {
                Ok(n) => format!("Imported {n} macros."),
                Err(e) => format!("Import failed: {e:#}"),
            });
        }
        self.menu(ui);
        if let Some(notice) = self.notice.clone() {
            egui::Panel::top("app-notice")
                .frame(egui::Frame::new().fill(CARD).inner_margin(8))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button("Dismiss").clicked() {
                            self.notice = None;
                        }
                        ui.label(notice);
                    });
                });
        }
        if let Some(review) = self.view.ui(ui) {
            ctx.copy_text(review);
            self.notice =
                Some("Review prompt copied. Paste it into your preferred assistant.".into());
        }
        if self.settings {
            self.settings(&ctx);
        }
        if self.about {
            self.about(&ctx);
        }
        if self.last_save.elapsed() > Duration::from_secs(2) {
            if let Err(e) = self.persist_session(&ctx) {
                self.notice = Some(format!("Could not save workspace settings: {e:#}"));
                self.last_save = Instant::now();
            }
        }
        ctx.request_repaint_after(Duration::from_secs(2));
        self.capture(&ctx);
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.view.save_if_dirty();
    }
}

fn configure(ctx: &egui::Context) {
    let mut style = egui::Style {
        visuals: egui::Visuals::dark(),
        ..Default::default()
    };
    style.visuals.text_edit_bg_color = Some(CARD);
    style.visuals.override_text_color = Some(INK);
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = BG;
    style.visuals.widgets.inactive.bg_fill = CARD;
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(49, 61, 70);
    style.visuals.selection.bg_fill = Color32::from_rgb(37, 67, 62);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, TEAL);
    style.visuals.hyperlink_color = GOLD;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 7.0);
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(11.0));
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_style_of(egui::Theme::Dark, style);
}

pub fn run(data_dir: PathBuf, files: Vec<PathBuf>, capture_dir: Option<PathBuf>) -> Result<()> {
    std::fs::create_dir_all(&data_dir).context("Create Spellbook data folder")?;
    let mut view = MacrosView::open(&data_dir.join("library.sqlite3"), "wow-forever")?;
    let session_path = data_dir.join("session.json");
    let mut notice = None;
    let saved = if session_path.exists() {
        match Session::load(&session_path) {
            Ok(s) => Some(s),
            Err(e) => {
                notice = Some(format!("Workspace settings could not be read: {e:#}"));
                None
            }
        }
    } else {
        None
    };
    if let Some(s) = &saved {
        view.restore_session(s);
    }
    let catalogue_path = data_dir.join("catalogue.json");
    if catalogue_path.exists() {
        match CatalogueFile::read(&catalogue_path) {
            Ok(custom) => {
                let mut c = Catalogue::bundled();
                c.merge(&custom);
                view.set_catalogue(c);
            }
            Err(e) => notice = Some(format!("Custom catalogue could not be read: {e:#}")),
        }
    }
    for path in files {
        if let Err(e) = view.import_path(&path) {
            notice = Some(format!("Import {}: {e:#}", path.display()));
        }
    }
    let last_session = saved.unwrap_or_default();
    let capture = if let Some(dir) = capture_dir {
        std::fs::create_dir_all(&dir)?;
        view.prepare_capture(0);
        Some(Capture {
            dir,
            step: 0,
            since: Instant::now(),
            pending: false,
            done: Arc::new(AtomicBool::new(false)),
        })
    } else {
        None
    };
    let icon = image::load_from_memory(include_bytes!("../../assets/spellbook.png"))?
        .resize_exact(128, 128, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    let icon_data = egui::IconData {
        width: icon.width(),
        height: icon.height(),
        rgba: icon.to_vec(),
    };
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(last_session.window_size)
            .with_min_inner_size([960.0, 640.0])
            .with_icon(icon_data),
        ..Default::default()
    };
    eframe::run_native(
        "Spellbook",
        native,
        Box::new(move |cc| {
            configure(&cc.egui_ctx);
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [icon.width() as usize, icon.height() as usize],
                icon.as_raw(),
            );
            view.set_brand(cc.egui_ctx.load_texture(
                "spellbook-emblem",
                image,
                egui::TextureOptions::LINEAR,
            ));
            Ok(Box::new(Spellbook {
                view,
                data_dir,
                notice,
                last_session,
                last_save: Instant::now(),
                capture,
                settings: false,
                about: false,
            }))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
