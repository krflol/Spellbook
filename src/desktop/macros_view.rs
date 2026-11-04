use super::{
    macro_editor::{self, Editor},
    widgets::{caption, primary},
    BG, GOLD, INK, MUTED, PANEL, TEAL,
};
use crate::macros::{
    self,
    language::{self, Catalogue, Severity},
    store::{Revision, SavedMacro, Store},
    LibraryFile, MacroDocument, CLASSES,
};
use anyhow::{ensure, Context, Result};
use eframe::egui::{self, Key, Modifiers, RichText};
use std::{
    io::{Read, Write},
    path::Path,
    time::{Duration, Instant},
};

struct Tab {
    saved: SavedMacro,
    draft: MacroDocument,
    editor: Editor,
    changed_at: Option<Instant>,
}
impl Tab {
    fn new(saved: SavedMacro) -> Self {
        Self {
            draft: saved.document.clone(),
            saved,
            editor: Editor::default(),
            changed_at: None,
        }
    }
    fn dirty(&self) -> bool {
        self.draft != self.saved.document
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolTab {
    Snippets,
    Reference,
    Details,
    History,
}
#[derive(Default)]
struct Actions {
    open: Option<i64>,
    close: Option<usize>,
    create: Option<MacroDocument>,
    insert: Option<String>,
    duplicate: bool,
    archive: bool,
    restore_archive: Option<i64>,
    reload: bool,
    restore_revision: Option<MacroDocument>,
    refresh: bool,
    save: bool,
    ask: bool,
}
pub struct MacrosView {
    brand: Option<egui::TextureHandle>,
    font_size: f32,
    auto_pairs: bool,
    store: Store,
    scope: String,
    entries: Vec<SavedMacro>,
    tabs: Vec<Tab>,
    active: usize,
    query: String,
    class_filter: String,
    favorites: bool,
    archived: bool,
    tool_tab: ToolTab,
    tools_visible: bool,
    tools_window: bool,
    reference_query: String,
    snippet_query: String,
    snippet_class: String,
    revisions: Vec<Revision>,
    revisions_key: Option<(i64, i64)>,
    catalogue: Catalogue,
    catalogue_status: String,
    error: Option<String>,
    message: Option<String>,
    transfer_open: bool,
    transfer_path: String,
    transfer_text: String,
    transfer_library: bool,
    problems_open: bool,
}
impl MacrosView {
    pub fn set_brand(&mut self, texture: egui::TextureHandle) {
        self.brand = Some(texture);
    }
    pub fn new_macro(&mut self) {
        self.create(MacroDocument::default());
    }
    pub fn show_transfer(&mut self) {
        self.transfer_open = true;
    }
    pub fn show_samples(&mut self, narrow: bool) {
        self.tool_tab = ToolTab::Snippets;
        self.tools_visible = true;
        self.tools_window = narrow;
    }
    pub fn show_reference(&mut self, narrow: bool) {
        self.tool_tab = ToolTab::Reference;
        self.tools_visible = true;
        self.tools_window = narrow;
    }
    pub fn preferences(&self) -> (f32, bool) {
        self.tabs
            .get(self.active)
            .map_or((self.font_size, self.auto_pairs), |t| {
                (t.editor.font_size, t.editor.auto_pairs)
            })
    }
    pub fn set_preferences(&mut self, font_size: f32, auto_pairs: bool) {
        self.font_size = font_size;
        self.auto_pairs = auto_pairs;
        for tab in &mut self.tabs {
            tab.editor.font_size = font_size;
            tab.editor.auto_pairs = auto_pairs;
        }
    }
    pub fn session(&self, window_size: [f32; 2]) -> crate::session::Session {
        let (font_size, auto_pairs) = self.preferences();
        crate::session::Session {
            tabs: self
                .tabs
                .iter()
                .map(|t| crate::session::OpenTab {
                    id: t.saved.id,
                    cursor: t.editor.cursor,
                })
                .collect(),
            active: self.tabs.get(self.active).map(|t| t.saved.id),
            font_size,
            auto_pairs,
            tools_visible: self.tools_visible,
            problems_open: self.problems_open,
            window_size,
        }
    }
    pub fn restore_session(&mut self, session: &crate::session::Session) {
        self.tabs.clear();
        self.active = 0;
        self.set_preferences(session.font_size, session.auto_pairs);
        for open in &session.tabs {
            self.open_macro(open.id);
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.saved.id == open.id) {
                let cursor = open.cursor.min(tab.draft.body.chars().count());
                tab.editor.select(cursor..cursor);
            }
        }
        self.active = self
            .tabs
            .iter()
            .position(|t| Some(t.saved.id) == session.active)
            .unwrap_or(0);
        self.tools_visible = session.tools_visible;
        self.problems_open = session.problems_open;
    }
    pub fn import_path(&mut self, path: &Path) -> Result<usize> {
        let text = crate::session::read_limited(path, 8 * 1024 * 1024)?;
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported macro");
        self.import_text(&text, name)
    }
    pub fn choose_import(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .set_title("Import macros")
            .add_filter("Macros", &["json", "txt", "macro", "lua"])
            .add_filter("All files", &["*"])
            .pick_files()
        {
            let mut count = 0;
            for path in paths {
                match self.import_path(&path) {
                    Ok(n) => count += n,
                    Err(e) => {
                        self.error = Some(format!("Import {}: {e:#}", path.display()));
                        break;
                    }
                }
            }
            if count > 0 {
                self.message = Some(format!("Imported {count} macros."));
            }
        }
    }
    pub fn choose_export(&mut self, library: bool) {
        if let Some(text) = self.export_text(library) {
            let name = if library {
                "Spellbook-library.json"
            } else {
                "macro.txt"
            };
            let ext = if library { "json" } else { "txt" };
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Export macros — choose a new filename")
                .set_file_name(name)
                .add_filter("Macro export", &[ext])
                .save_file()
            {
                let result = (|| -> Result<()> {
                    let mut file = std::fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(&path)
                        .context("Choose a new filename; existing files are never replaced")?;
                    file.write_all(text.as_bytes())?;
                    file.sync_all()?;
                    Ok(())
                })();
                match result {
                    Ok(()) => self.message = Some(format!("Exported {}", path.display())),
                    Err(e) => self.error = Some(format!("Export failed: {e:#}")),
                }
            }
        }
    }
    pub fn set_catalogue(&mut self, catalogue: Catalogue) {
        self.catalogue_status = format!("{} offline reference names", catalogue.symbols.len());
        self.catalogue = catalogue;
    }
    pub fn open(path: &Path, scope: &str) -> Result<Self> {
        let store = Store::open(path, scope)?;
        let entries = store.list()?;
        let mut tabs = Vec::new();
        if let Some(saved) = entries.iter().find(|s| !s.archived) {
            tabs.push(Tab::new(saved.clone()));
        }
        let catalogue = Catalogue::bundled();
        let catalogue_status = format!("{} offline reference names", catalogue.symbols.len());
        Ok(Self {
            brand: None,
            font_size: 16.0,
            auto_pairs: true,
            store,
            scope: scope.into(),
            entries,
            tabs,
            active: 0,
            query: String::new(),
            class_filter: "All classes".into(),
            favorites: false,
            archived: false,
            tool_tab: ToolTab::Snippets,
            tools_visible: true,
            tools_window: false,
            reference_query: String::new(),
            snippet_query: String::new(),
            snippet_class: "All classes".into(),
            revisions: vec![],
            revisions_key: None,
            catalogue,
            catalogue_status,
            error: None,
            message: None,
            transfer_open: false,
            transfer_path: String::new(),
            transfer_text: String::new(),
            transfer_library: true,
            problems_open: true,
        })
    }
    pub fn poll(&mut self, ctx: &egui::Context) {
        let due = self.tabs.iter().any(|t| {
            t.changed_at
                .is_some_and(|at| at.elapsed() >= Duration::from_millis(900))
        });
        if due {
            self.save_if_dirty();
        }
        if self.tabs.iter().any(|t| t.changed_at.is_some()) {
            ctx.request_repaint_after(Duration::from_millis(950));
        }
    }
    pub fn save_if_dirty(&mut self) -> bool {
        let mut success = true;
        let mut changed = false;
        for tab in &mut self.tabs {
            tab.changed_at = None;
            if tab.dirty() {
                match self.store.save(&mut tab.saved, &tab.draft) {
                    Ok(()) => {
                        changed = true;
                    }
                    Err(e) => {
                        self.error = Some(format!("Could not save {}: {e:#}", tab.draft.name));
                        success = false;
                    }
                }
            }
        }
        if changed {
            self.refresh();
        }
        success
    }
    fn refresh(&mut self) {
        match self.store.list() {
            Ok(entries) => self.entries = entries,
            Err(e) => self.error = Some(format!("Could not load macro library: {e:#}")),
        }
    }
    fn create(&mut self, doc: MacroDocument) {
        match self.store.create(&doc) {
            Ok(saved) => {
                self.tabs.push(Tab::new(saved));
                self.active = self.tabs.len() - 1;
                self.tabs[self.active].editor.focus = true;
                self.tabs[self.active].editor.font_size = self.font_size;
                self.tabs[self.active].editor.auto_pairs = self.auto_pairs;
                self.archived = false;
                self.query.clear();
                self.class_filter = "All classes".into();
                self.favorites = false;
                self.refresh();
            }
            Err(e) => self.error = Some(format!("Could not create macro: {e:#}")),
        }
    }
    fn open_macro(&mut self, id: i64) {
        if let Some(index) = self.tabs.iter().position(|t| t.saved.id == id) {
            self.active = index;
        } else if let Some(saved) = self.entries.iter().find(|s| s.id == id && !s.archived) {
            self.tabs.push(Tab::new(saved.clone()));
            self.active = self.tabs.len() - 1;
            self.tabs[self.active].editor.font_size = self.font_size;
            self.tabs[self.active].editor.auto_pairs = self.auto_pairs;
        }
    }
    /// Returns a review prompt to copy; authoring never executes the macro.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let ctx = ui.ctx().clone();
        let mut actions = Actions::default();
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::S)) {
            actions.save = true;
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::N)) {
            actions.create = Some(MacroDocument::default());
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::W)) && !self.tabs.is_empty() {
            actions.close = Some(self.active);
        }
        let wide = ui.available_width() >= 1120.0;
        egui::Panel::top("macro-heading")
            .exact_size(88.0)
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(24, 14)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(brand) = &self.brand {
                        ui.image((brand.id(), egui::vec2(48.0, 48.0)));
                        ui.add_space(6.0);
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Spellbook").size(27.0).strong().color(GOLD));
                        ui.label(
                            RichText::new("WoW macro IDE  /  Offline workspace")
                                .size(12.0)
                                .color(MUTED),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(primary("+ New macro"))
                            .on_hover_text("Ctrl+N")
                            .clicked()
                        {
                            actions.create = Some(MacroDocument::default());
                        }
                        if ui.button("Import / export").clicked() {
                            self.transfer_open = true;
                        }
                        if ui
                            .selectable_label(self.tools_visible && wide, "Tools")
                            .clicked()
                        {
                            if wide {
                                self.tools_visible = !self.tools_visible;
                            } else {
                                self.tools_window = !self.tools_window;
                            }
                        }
                    });
                });
            });
        egui::Panel::left("macro-library")
            .default_size(235.0)
            .size_range(195.0..=310.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(16))
            .show(ui, |ui| self.library(ui, &mut actions));
        if self.tools_visible && wide {
            egui::Panel::right("macro-tools")
                .default_size(330.0)
                .size_range(260.0..=410.0)
                .resizable(true)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(16))
                .show(ui, |ui| self.tools(ui, &mut actions));
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(18)).show(ui,|ui| {
            if let Some(error)=self.error.clone() {
                egui::Frame::new().fill(egui::Color32::from_rgb(62,31,34)).inner_margin(10).show(ui,|ui| {
                    ui.add(egui::Label::new(RichText::new(error).color(macro_editor::ERROR)).wrap());
                    if ui.small_button("Dismiss error").clicked() {self.error=None;}
                });
            }
            if let Some(message)=&self.message { ui.label(RichText::new(message).small().color(TEAL)); }
            self.tab_bar(ui,&mut actions);
            if self.tabs.is_empty() { self.empty(ui,&mut actions); return; }
            let tab=&mut self.tabs[self.active];
            let before=tab.draft.clone();
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut tab.draft.name).id_salt(("macro-name",tab.saved.id)).font(egui::FontId::proportional(21.0)).desired_width((ui.available_width()-235.0).max(100.0)).hint_text("Macro name"));
                if ui.selectable_label(tab.draft.favorite,"Favorite").on_hover_text("Favorite macro").clicked() {tab.draft.favorite = !tab.draft.favorite;}
                if ui.button("Copy macro").clicked() {ui.ctx().copy_text(tab.draft.body.clone()); self.message=Some("Macro copied. Paste it into WoW's macro editor.".into());}
                ui.menu_button("···",|ui| {
                    if ui.button("Save now").clicked() {actions.save=true;ui.close();}
                    if ui.button("Duplicate macro").clicked() {actions.duplicate=true;ui.close();}
                    if ui.button("Copy review prompt").clicked() {actions.ask=true;ui.close();}
                    if ui.button("Reload saved version").clicked() {actions.reload=true;ui.close();}
                    if ui.button("Move to trash").clicked() {actions.archive=true;ui.close();}
                });
            });
            ui.horizontal_wrapped(|ui| {
                let dirty=tab.dirty();
                ui.label(RichText::new(if dirty {"Unsaved changes"} else {"Saved locally"}).small().color(if dirty {GOLD} else {TEAL}));
                ui.label(RichText::new(format!("·  {}  ·  {}",tab.draft.class,if tab.draft.character.is_empty() {"Account-wide"} else {&tab.draft.character})).small().color(MUTED));
                ui.label(RichText::new(format!("·  revision {}",tab.saved.revision)).small().color(MUTED));
            });
            ui.add_space(10.0);
            let diagnostics=language::diagnose(&tab.draft.name,&tab.draft.body);
            egui::Panel::bottom("macro-editor-status").exact_size(28.0).frame(egui::Frame::new().fill(BG).inner_margin(4)).show(ui,|ui| {
                let byte=macros::byte_index(&tab.draft.body,tab.editor.cursor);
                let prefix=&tab.draft.body[..byte];
                let line=prefix.matches('\n').count()+1;
                let column=prefix.rsplit('\n').next().unwrap_or("").chars().count()+1;
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("Ln {line}, Col {column}")).monospace().size(11.0).color(MUTED));
                    let count=tab.draft.body.chars().count();
                    ui.label(RichText::new(format!("{count} / {} chars",macros::BODY_LIMIT)).monospace().size(11.0).color(if count>macros::BODY_LIMIT {macro_editor::ERROR} else {TEAL}));
                    ui.add(egui::ProgressBar::new(count as f32 / macros::BODY_LIMIT as f32).desired_width(50.0).desired_height(4.0).fill(if count>macros::BODY_LIMIT {macro_editor::ERROR} else {TEAL}));
                    ui.label(RichText::new(format!("{} bytes · UTF-8",tab.draft.body.len())).size(11.0).color(MUTED));
                    if ui.small_button(if self.problems_open {"Hide problems"} else {"Show problems"}).clicked() {self.problems_open = !self.problems_open;}
                });
            });
            if self.problems_open {
                egui::Panel::bottom("macro-problems").default_size(138.0).size_range(80.0..=260.0).resizable(true).frame(egui::Frame::new().fill(BG).inner_margin(8)).show(ui,|ui| {
                    ui.horizontal(|ui| {
                        caption(ui,&format!("PROBLEMS  ·  {}",diagnostics.len()));
                        ui.label(RichText::new("Structural checks").small().color(MUTED)).on_hover_text("Checks macro length, commands and conditional brackets, plus Lua delimiter balance. Does not execute Lua or validate gameplay availability.");
                    });
                    egui::ScrollArea::vertical().id_salt("macro-diagnostics").show(ui,|ui| {
                        if diagnostics.is_empty() { ui.label(RichText::new("No structural issues found.").color(TEAL)); }
                        for issue in &diagnostics {
                            let severity=match issue.severity {Severity::Error=>"ERROR",Severity::Warning=>"WARN",Severity::Info=>"INFO"};
                            if ui.add(egui::Button::new(RichText::new(format!("{severity}  {}:{}  {}",issue.line,issue.column,issue.message)).size(12.0).color(macro_editor::severity_color(issue.severity))).frame(false).wrap()).clicked() {tab.editor.go_to(&tab.draft.body,issue.line,issue.column);}
                        }
                    });
                });
            }
            tab.editor.ui(ui,tab.saved.id,&mut tab.draft.body,&self.catalogue,&diagnostics);
            if before!=tab.draft {tab.changed_at=Some(Instant::now());ctx.request_repaint_after(Duration::from_millis(950));}
        });
        if self.tools_window {
            let mut open = true;
            egui::Window::new("Macro tools")
                .open(&mut open)
                .default_width(360.0)
                .default_height(550.0)
                .show(&ctx, |ui| self.tools(ui, &mut actions));
            self.tools_window = open;
        }
        if self.transfer_open {
            self.transfer(&ctx);
        }
        self.apply(&ctx, actions)
    }
    fn library(&mut self, ui: &mut egui::Ui, actions: &mut Actions) {
        ui.horizontal(|ui| {
            caption(ui, "YOUR MACROS");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("Refresh")
                    .on_hover_text("Refresh saved library")
                    .clicked()
                {
                    actions.refresh = true;
                }
            });
        });
        ui.add_space(10.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text("Search names, tags, code…")
                .desired_width(f32::INFINITY),
        );
        egui::ComboBox::from_id_salt("macro-class-filter")
            .width(ui.available_width())
            .selected_text(&self.class_filter)
            .show_ui(ui, |ui| {
                for class in std::iter::once(&"All classes").chain(CLASSES.iter()) {
                    ui.selectable_value(&mut self.class_filter, (*class).into(), *class);
                }
            });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.favorites, false, "All");
            ui.selectable_value(&mut self.favorites, true, "Favorites");
        });
        ui.separator();
        let query = self.query.to_lowercase();
        let selected = self.tabs.get(self.active).map(|t| t.saved.id);
        let matches: Vec<_> = self
            .entries
            .iter()
            .filter(|s| s.archived == self.archived)
            .filter_map(|saved| {
                let doc = self
                    .tabs
                    .iter()
                    .find(|t| t.saved.id == saved.id)
                    .map_or(&saved.document, |t| &t.draft);
                ((!self.favorites || doc.favorite)
                    && (self.class_filter == "All classes" || self.class_filter == doc.class)
                    && format!(
                        "{} {} {} {} {}",
                        doc.name, doc.tags, doc.body, doc.notes, doc.character
                    )
                    .to_lowercase()
                    .contains(&query))
                .then_some((saved, doc))
            })
            .collect();
        let count = matches.len();
        ui.label(
            RichText::new(format!(
                "{count} {}",
                if self.archived { "in trash" } else { "macros" }
            ))
            .small()
            .color(MUTED),
        );
        egui::ScrollArea::vertical()
            .id_salt("macro-library-list")
            .max_height((ui.available_height() - 78.0).max(90.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if matches.is_empty() {
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new(if self.entries.is_empty() {
                            "Your next shortcut starts here.\nCreate a macro or choose a snippet."
                        } else {
                            "No matching macros."
                        })
                        .color(MUTED),
                    );
                }
                for (saved, doc) in matches {
                    ui.push_id(saved.id, |ui| {
                        egui::Frame::new()
                            .fill(if selected == Some(saved.id) {
                                egui::Color32::from_rgb(29, 49, 47)
                            } else {
                                PANEL
                            })
                            .stroke(egui::Stroke::new(
                                1.0,
                                if selected == Some(saved.id) {
                                    TEAL.gamma_multiply(0.55)
                                } else {
                                    egui::Color32::TRANSPARENT
                                },
                            ))
                            .corner_radius(6.0)
                            .inner_margin(10)
                            .show(ui, |ui| {
                                let name = if doc.name.trim().is_empty() {
                                    "Untitled macro"
                                } else {
                                    &doc.name
                                };
                                let label =
                                    format!("{}{}", if doc.favorite { "*  " } else { "" }, name);
                                let button = ui.add_sized(
                                    [ui.available_width(), 25.0],
                                    egui::Button::new(RichText::new(label).strong().color(
                                        if selected == Some(saved.id) {
                                            GOLD
                                        } else {
                                            INK
                                        },
                                    ))
                                    .frame(false)
                                    .wrap(),
                                );
                                if button.clicked() && !self.archived {
                                    actions.open = Some(saved.id);
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "{} · {} chars",
                                        doc.class,
                                        doc.body.chars().count()
                                    ))
                                    .size(10.0)
                                    .color(MUTED),
                                );
                                if !doc.tags.is_empty() {
                                    ui.label(RichText::new(&doc.tags).size(10.0).color(TEAL));
                                }
                                if self.archived && ui.small_button("Restore macro").clicked() {
                                    actions.restore_archive = Some(saved.id);
                                }
                            });
                        ui.add_space(5.0);
                    });
                }
            });
        ui.separator();
        if ui
            .selectable_label(
                self.archived,
                if self.archived {
                    "Back to library"
                } else {
                    "Trash"
                },
            )
            .clicked()
        {
            self.archived = !self.archived;
        }
        ui.label(
            RichText::new("Autosaved on this computer")
                .size(10.0)
                .color(TEAL),
        );
    }
    fn tab_bar(&mut self, ui: &mut egui::Ui, actions: &mut Actions) {
        if self.tabs.is_empty() {
            return;
        }
        egui::ScrollArea::horizontal()
            .id_salt("macro-tabs")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (index, tab) in self.tabs.iter().enumerate() {
                        ui.push_id(tab.saved.id, |ui| {
                            let name = tab.draft.name.chars().take(22).collect::<String>();
                            if ui
                                .selectable_label(
                                    index == self.active,
                                    format!("{}{}", name, if tab.dirty() { " •" } else { "" }),
                                )
                                .clicked()
                            {
                                self.active = index;
                            }
                            if ui
                                .small_button("×")
                                .on_hover_text("Close tab · Ctrl+W")
                                .clicked()
                            {
                                actions.close = Some(index);
                            }
                        });
                    }
                });
            });
        ui.separator();
        ui.add_space(8.0);
    }
    fn empty(&mut self, ui: &mut egui::Ui, actions: &mut Actions) {
        ui.add_space(65.0);
        ui.label(
            RichText::new("A little code. A better adventure.")
                .size(27.0)
                .color(GOLD),
        );
        ui.add_space(12.0);
        ui.label(
            "Build your own collection of WoW macros, with the tools to get every detail right.",
        );
        ui.add_space(20.0);
        ui.horizontal_wrapped(|ui| {
            if ui.add(primary("Create your first macro")).clicked() {
                actions.create = Some(MacroDocument::default());
            }
            if ui.button("Start with a mouseover heal").clicked() {
                actions.create = Some(template(&language::SNIPPETS[0]));
            }
        });
        ui.add_space(28.0);
        for (title, description) in [
            (
                "Write with confidence",
                "Syntax colors, contextual suggestions, bracket matching and live problems.",
            ),
            (
                "Keep your flow",
                "Multiple tabs, undo and redo, find and replace, keyboard shortcuts.",
            ),
            (
                "Keep your work",
                "Automatic local saves, revision history, import and export.",
            ),
        ] {
            ui.label(RichText::new(title).strong().color(INK));
            ui.label(RichText::new(description).color(MUTED));
            ui.add_space(14.0);
        }
    }
    fn tools(&mut self, ui: &mut egui::Ui, actions: &mut Actions) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().button_padding.x = 5.0;
            for (tab, name) in [
                (ToolTab::Snippets, "Snippets"),
                (ToolTab::Reference, "Reference"),
                (ToolTab::Details, "Details"),
                (ToolTab::History, "History"),
            ] {
                ui.selectable_value(&mut self.tool_tab, tab, name);
            }
        });
        ui.separator();
        ui.add_space(8.0);
        if self.tool_tab == ToolTab::History {
            let key = self
                .tabs
                .get(self.active)
                .map(|t| (t.saved.id, t.saved.revision));
            if key != self.revisions_key {
                self.revisions.clear();
                self.revisions_key = key;
                if let Some((id, _)) = key {
                    match self.store.revisions(id) {
                        Ok(revisions) => self.revisions = revisions,
                        Err(e) => self.error = Some(e.to_string()),
                    }
                }
            }
        }
        egui::ScrollArea::vertical().id_salt("macro-tools-scroll").auto_shrink([false,false]).show(ui,|ui| {
            match self.tool_tab {
                ToolTab::Snippets=>self.snippets(ui,actions),
                ToolTab::Reference=>{
                    ui.add(egui::TextEdit::singleline(&mut self.reference_query).hint_text("Find a command or condition…").desired_width(f32::INFINITY));
                    let query=self.reference_query.to_lowercase();
                    for (name,refs) in [("Commands",language::COMMANDS),("Conditions",language::CONDITIONS),("Targets",language::TARGETS),("Lua & API",language::LUA)] {
                        egui::CollapsingHeader::new(name).default_open(!query.is_empty() || name=="Commands").show(ui,|ui| {
                            for r in refs.iter().filter(|r| query.is_empty() || format!("{} {}",r.name,r.description).to_lowercase().contains(&query)) {
                                ui.label(RichText::new(r.name).monospace().color(GOLD));ui.label(RichText::new(r.description).size(12.0));ui.add(egui::Label::new(RichText::new(r.example).monospace().size(11.0).color(TEAL)).wrap());ui.add_space(10.0);
                            }
                        });
                    }
                    ui.separator();ui.label(RichText::new("Condition support depends on the game version. Local name suggestions are a catalogue, not your character's spellbook.").small().color(MUTED));
                    self.shortcuts(ui);
                }
                ToolTab::Details=>{
                    if let Some(tab)=self.tabs.get_mut(self.active) {
                        let before=tab.draft.clone();
                        caption(ui,"ORGANIZE");ui.label("Class");
                        egui::ComboBox::from_id_salt("macro-class").selected_text(&tab.draft.class).width(ui.available_width()).show_ui(ui,|ui| {for class in CLASSES {ui.selectable_value(&mut tab.draft.class,(*class).into(),*class);}});
                        ui.label("Character");ui.add(egui::TextEdit::singleline(&mut tab.draft.character).hint_text("Blank = account-wide").desired_width(f32::INFINITY));
                        ui.label("Tags");ui.add(egui::TextEdit::singleline(&mut tab.draft.tags).hint_text("healing, mouseover, pvp").desired_width(f32::INFINITY));
                        ui.add_space(12.0);ui.label("Personal notes");ui.add(egui::TextEdit::multiline(&mut tab.draft.notes).desired_rows(6).hint_text("Keybind, when to use it, things to test…").desired_width(f32::INFINITY));
                        ui.checkbox(&mut tab.draft.favorite,"Favorite");
                        if before!=tab.draft{tab.changed_at=Some(Instant::now());}
                        ui.separator();ui.label(RichText::new("Copy the macro into WoW to use it. Character and class labels organize this library.").small().color(MUTED));
                    }else{ui.label("Open a macro to edit its details.");}
                }
                ToolTab::History=>{
                    caption(ui,"SAVED VERSIONS");ui.label(RichText::new("Restoring creates a new revision. Earlier versions remain available. Latest 100 shown.").small().color(MUTED));ui.add_space(8.0);
                    for revision in &self.revisions {
                        egui::CollapsingHeader::new(format!("Revision {} · {} UTC",revision.number,revision.at)).id_salt(revision.number).show(ui,|ui| {
                            ui.label(RichText::new(&revision.document.name).strong());
                            ui.add(egui::Label::new(RichText::new(&revision.document.body).monospace().size(11.0)).wrap());
                            if ui.button("Restore this version").clicked(){actions.restore_revision=Some(revision.document.clone());}
                        });
                    }
                    if self.revisions.is_empty(){ui.label("Saved versions appear here.");}
                }
            }
            ui.add_space(16.0);ui.separator();ui.label(RichText::new(&self.catalogue_status).size(10.0).color(TEAL));
        });
    }
    fn shortcuts(&self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Keyboard shortcuts").show(ui, |ui| {
            for text in [
                "Ctrl+Space  Suggestions",
                "Tab / Enter  Accept suggestion",
                "Esc  Dismiss suggestions / find",
                "Ctrl+F / Ctrl+H  Find / replace",
                "Ctrl+G  Go to line",
                "Ctrl+D  Duplicate line",
                "Ctrl+Z / Ctrl+Shift+Z  Undo / redo",
                "Ctrl+S  Save all open macros",
                "Ctrl+N / Ctrl+W  New / close tab",
            ] {
                ui.label(RichText::new(text).size(11.0).color(MUTED));
            }
        });
    }
    fn snippets(&mut self, ui: &mut egui::Ui, actions: &mut Actions) {
        caption(ui, "A HEAD START");
        ui.label(
            RichText::new("Choose a sample, then adapt it to your character.")
                .small()
                .color(MUTED),
        );
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.snippet_query)
                .hint_text("Search samples, spells, conditions…")
                .desired_width(f32::INFINITY),
        );
        egui::ComboBox::from_id_salt("macro-snippet-class")
            .selected_text(&self.snippet_class)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for class in std::iter::once(&"All classes").chain(CLASSES.iter()) {
                    ui.selectable_value(&mut self.snippet_class, (*class).into(), *class);
                }
            });
        let samples: Vec<_> = language::SNIPPETS
            .iter()
            .filter(|s| s.matches(&self.snippet_query, &self.snippet_class))
            .collect();
        ui.label(
            RichText::new(format!(
                "{} samples · includes class-independent helpers",
                samples.len()
            ))
            .small()
            .color(MUTED),
        );
        if samples.is_empty() {
            ui.label("No samples match. Try a spell name, stance, or form.");
        }
        let mut category = "";
        for snippet in samples {
            if category != snippet.category {
                category = snippet.category;
                ui.add_space(12.0);
                caption(ui, &category.to_uppercase());
                if let Some(guidance) = snippet.guidance() {
                    ui.label(RichText::new(guidance).small().color(MUTED));
                    if ui.small_button("Create Inspect forms helper").clicked() {
                        actions.create = language::SNIPPETS
                            .iter()
                            .find(|s| s.name == "Inspect forms")
                            .map(template);
                    }
                    ui.add_space(6.0);
                }
            }
            ui.push_id(snippet.name, |ui| {
                egui::CollapsingHeader::new(RichText::new(snippet.name).strong())
                    .default_open(snippet.name == "Mouseover heal")
                    .open((!self.snippet_query.trim().is_empty()).then_some(true))
                    .show(ui, |ui| {
                        ui.label(RichText::new(snippet.class).small().color(GOLD));
                        ui.label(RichText::new(snippet.description).small().color(MUTED));
                        ui.add(
                            egui::Label::new(
                                RichText::new(snippet.example)
                                    .monospace()
                                    .size(11.0)
                                    .color(TEAL),
                            )
                            .wrap(),
                        );
                        ui.horizontal(|ui| {
                            if ui.small_button("New from template").clicked() {
                                actions.create = Some(template(snippet));
                            }
                            if ui
                                .add_enabled(
                                    !self.tabs.is_empty(),
                                    egui::Button::new("Insert").small(),
                                )
                                .clicked()
                            {
                                actions.insert = Some(snippet.example.into());
                            }
                        });
                    });
            });
        }
        ui.add_space(18.0);
        self.shortcuts(ui);
    }
    fn apply(&mut self, ctx: &egui::Context, actions: Actions) -> Option<String> {
        if actions.refresh {
            self.refresh();
        }
        if actions.save && self.save_if_dirty() {
            self.message = Some("All open macros saved.".into());
        }
        if let Some(id) = actions.open {
            self.open_macro(id);
        }
        if let Some(doc) = actions.create {
            self.create(doc);
        }
        if actions.duplicate {
            if let Some(tab) = self.tabs.get(self.active) {
                let mut doc = tab.draft.clone();
                doc.name = format!("{} copy", doc.name);
                self.create(doc);
            }
        }
        if actions.reload {
            self.refresh();
            if let Some(tab) = self.tabs.get_mut(self.active) {
                if let Some(saved) = self
                    .entries
                    .iter()
                    .find(|s| s.id == tab.saved.id && !s.archived)
                {
                    tab.editor.edit(
                        ctx,
                        tab.saved.id,
                        &mut tab.draft.body,
                        saved.document.body.clone(),
                        0..0,
                    );
                    tab.saved = saved.clone();
                    tab.draft = saved.document.clone();
                    tab.changed_at = None;
                    self.error = None;
                }
            }
        }
        if let Some(insert) = actions.insert {
            if let Some(tab) = self.tabs.get_mut(self.active) {
                tab.editor
                    .insert(ctx, tab.saved.id, &mut tab.draft.body, &insert);
                tab.changed_at = Some(Instant::now());
            }
        }
        if let Some(doc) = actions.restore_revision {
            if self.save_if_dirty() {
                if let Some(tab) = self.tabs.get_mut(self.active) {
                    tab.editor.edit(
                        ctx,
                        tab.saved.id,
                        &mut tab.draft.body,
                        doc.body.clone(),
                        0..0,
                    );
                    tab.draft = doc;
                    tab.changed_at = Some(Instant::now());
                }
                if self.save_if_dirty() {
                    self.message = Some("Earlier version restored as a new save.".into());
                }
            }
        }
        let mut close = actions.close;
        if actions.archive && self.save_if_dirty() {
            if let Some(tab) = self.tabs.get(self.active) {
                match self.store.archive(&tab.saved, true) {
                    Ok(()) => {
                        close = Some(self.active);
                        self.refresh();
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
        }
        if let Some(id) = actions.restore_archive {
            if let Some(saved) = self.entries.iter().find(|s| s.id == id).cloned() {
                match self.store.archive(&saved, false) {
                    Ok(()) => {
                        self.refresh();
                        self.archived = false;
                        self.open_macro(id);
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
        }
        if let Some(index) = close {
            if self.save_if_dirty() && index < self.tabs.len() {
                self.tabs.remove(index);
                if self.active > index {
                    self.active -= 1;
                }
                self.active = self.active.min(self.tabs.len().saturating_sub(1));
            }
        }
        if actions.ask {
            return self.tabs.get(self.active).map(|tab|format!("Help me review and improve this WoW macro. Explain any changes and provide a macro I can paste.\n\nName: {}\nClass: {}\nNotes: {}\n\n```\n{}\n```",tab.draft.name,tab.draft.class,tab.draft.notes,tab.draft.body));
        }
        if self.tabs.iter().any(|t| t.changed_at.is_some()) {
            ctx.request_repaint_after(Duration::from_millis(950));
        }
        None
    }
    fn transfer(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("Import / export macros").open(&mut open).default_width(650.0).resizable(true).show(ctx,|ui| {
            ui.horizontal(|ui| {
                if ui.button("Browse import files…").clicked(){self.choose_import();}
                if ui.button("Export library…").clicked(){self.choose_export(true);}
                if ui.add_enabled(!self.tabs.is_empty(),egui::Button::new("Export current macro…")).clicked(){self.choose_export(false);}
            });
            ui.separator();
            ui.label("Import a plain-text macro or a Spellbook or Forever Atlas JSON library. Imports create new macros.");
            ui.add(egui::TextEdit::multiline(&mut self.transfer_text).font(egui::TextStyle::Monospace).hint_text("Paste macro text or library JSON here…").desired_rows(8).desired_width(f32::INFINITY));
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(!self.transfer_text.trim().is_empty(),egui::Button::new("Import pasted text")).clicked(){let text=self.transfer_text.clone();match self.import_text(&text,"Imported macro"){Ok(n)=>{self.message=Some(format!("Imported {n} macros."));self.transfer_text.clear();},Err(e)=>self.error=Some(e.to_string())}}
                if ui.button("Copy library JSON").clicked(){if let Some(text)=self.export_text(true){ctx.copy_text(text);self.message=Some("Library JSON copied.".into());}}
            });
            ui.separator();ui.label("File path");
            ui.add(egui::TextEdit::singleline(&mut self.transfer_path).hint_text("C:\\Users\\…\\macros.json").desired_width(f32::INFINITY));
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(!self.transfer_path.trim().is_empty(),egui::Button::new("Import file")).clicked(){
                    let path=self.transfer_path.trim().to_string();
                    let result=(||->Result<usize>{let mut file=std::fs::File::open(&path).context("Open import file")?;ensure!(file.metadata()?.len()<=8*1024*1024,"Import exceeds 8 MiB");let mut text=String::new();Read::by_ref(&mut file).take(8*1024*1024+1).read_to_string(&mut text)?;ensure!(text.len()<=8*1024*1024,"Import exceeds 8 MiB");let name=Path::new(&path).file_stem().and_then(|s|s.to_str()).unwrap_or("Imported macro");self.import_text(&text,name)})();
                    match result{Ok(n)=>self.message=Some(format!("Imported {n} macros from {path}.")),Err(e)=>self.error=Some(format!("Import failed: {e:#}"))}
                }
                ui.checkbox(&mut self.transfer_library,"Entire library (JSON)");
                if ui.add_enabled(!self.transfer_path.trim().is_empty() && (self.transfer_library || !self.tabs.is_empty()),egui::Button::new("Export file")).clicked(){
                    if let Some(text)=self.export_text(self.transfer_library){let result=(||->Result<()>{let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(self.transfer_path.trim()).context("Choose a new file path; existing files are never replaced")?;file.write_all(text.as_bytes())?;file.sync_all()?;Ok(())})();match result{Ok(())=>self.message=Some("Macro export saved.".into()),Err(e)=>self.error=Some(format!("Export failed: {e:#}"))}}
                }
            });
            ui.label(RichText::new("Uncheck Entire library to export the current macro as plain text. Choose a new filename for each export.").small().color(MUTED));
            if let Some(error)=&self.error{ui.add(egui::Label::new(RichText::new(error).color(macro_editor::ERROR)).wrap());}
            if let Some(message)=&self.message{ui.label(RichText::new(message).color(TEAL));}
        });
        self.transfer_open = open;
    }
    fn export_text(&mut self, library: bool) -> Option<String> {
        if !self.save_if_dirty() {
            return None;
        }
        if library {
            match LibraryFile::encode(
                &self.scope,
                self.entries
                    .iter()
                    .filter(|s| !s.archived)
                    .map(|s| s.document.clone())
                    .collect(),
            ) {
                Ok(json) => Some(json),
                Err(e) => {
                    self.error = Some(e.to_string());
                    None
                }
            }
        } else {
            self.tabs.get(self.active).map(|t| t.draft.body.clone())
        }
    }
    fn import_text(&mut self, text: &str, name: &str) -> Result<usize> {
        let normalized = macros::normalize(text);
        let docs = if normalized.trim_start().starts_with('{') {
            LibraryFile::decode(&normalized)?.macros
        } else {
            vec![MacroDocument {
                name: name.into(),
                body: normalized,
                ..Default::default()
            }]
        };
        let imported = self.store.import(&docs)?;
        let count = imported.len();
        self.refresh();
        self.query.clear();
        self.class_filter = "All classes".into();
        self.favorites = false;
        self.archived = false;
        if let Some(saved) = imported.first() {
            self.open_macro(saved.id);
        }
        Ok(count)
    }
    pub fn prepare_capture(&mut self, step: usize) {
        self.tools_window = false;
        self.tool_tab = ToolTab::Snippets;
        self.tools_visible = true;
        self.snippet_query.clear();
        self.snippet_class = "All classes".into();
        if step == 0 {
            self.tabs.clear();
            self.active = 0;
            return;
        }
        if step == 1 {
            for name in [
                "Charge dance",
                "Cat Prowl",
                "Mouseover heal",
                "Travel forms",
            ] {
                if let Some(sample) = language::SNIPPETS.iter().find(|s| s.name == name) {
                    self.create(template(sample));
                }
            }
            self.active = 0;
            self.snippet_class = "Warrior".into();
            self.snippet_query = "Charge".into();
            return;
        }
        if step == 3 {
            if let Some(index) = self.tabs.iter().position(|t| t.draft.name == "Cat Prowl") {
                self.active = index;
            }
            self.snippet_class = "Druid".into();
            self.snippet_query = "Cat".into();
            return;
        }
        if step == 5 {
            self.active = 0;
            if let Some(tab) = self.tabs.first_mut() {
                tab.draft
                    .notes
                    .push_str("\nReview: confirm stance slots before binding.");
            }
            self.save_if_dirty();
            self.tool_tab = ToolTab::History;
            return;
        }
        if step >= 6 {
            self.active = 0;
            return;
        }
        if step == 2 {
            self.create(MacroDocument {
                name: "Mouseover cast".into(),
                body: "#showtooltip\n/cast [@mouseover,harm,nodead][] Fireba".into(),
                class: "Mage".into(),
                ..Default::default()
            });
            let tab = &mut self.tabs[self.active];
            let end = tab.draft.body.chars().count();
            tab.editor.select(end..end);
            tab.editor.show_completion(&tab.draft.body, &self.catalogue);
            return;
        }
        if step == 4 {
            self.create(MacroDocument {
                name: "Draft check".into(),
                body: "#showtooltip\n/cast [@mouseover,harm,nodead Fireball\n/run print(\"Hello\""
                    .into(),
                class: "Mage".into(),
                ..Default::default()
            });
            return;
        }
        if self.tabs.is_empty() {
            self.create(MacroDocument {
                name: "Focus interrupt".into(),
                body: language::SNIPPETS[1].example.into(),
                class: "Mage".into(),
                tags: "utility, focus".into(),
                ..Default::default()
            });
        }
        self.tool_tab = if step == 1 {
            ToolTab::Reference
        } else {
            ToolTab::Snippets
        };
        if step == 1 {
            self.reference_query = "cast".into();
        }
    }
}

fn template(snippet: &macros::snippets::Snippet) -> MacroDocument {
    snippet.document()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, MacrosView, egui::Context) {
        let dir = tempfile::tempdir().unwrap();

        let ctx = egui::Context::default();
        super::super::configure(&ctx);
        ctx.enable_accesskit();
        let view = MacrosView::open(&dir.path().join("desktop.sqlite3"), "test").unwrap();
        (dir, view, ctx)
    }
    fn frame(
        view: &mut MacrosView,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> Vec<(String, egui::Pos2)> {
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1304.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                view.ui(ui);
            },
        );
        let nodes = output
            .platform_output
            .accesskit_update
            .as_ref()
            .map(|update| {
                update
                    .nodes
                    .iter()
                    .filter_map(|(_, node)| {
                        Some((node.label()?.to_owned(), {
                            let r = node.bounds()?;
                            egui::pos2(((r.x0 + r.x1) / 2.0) as f32, ((r.y0 + r.y1) / 2.0) as f32)
                        }))
                    })
                    .collect()
            })
            .unwrap_or_default();
        output.drop_without_applying_deltas();
        nodes
    }
    fn click(view: &mut MacrosView, ctx: &egui::Context, label: &str) {
        frame(view, ctx, vec![]);
        let nodes = frame(view, ctx, vec![]);
        let pos = nodes
            .iter()
            .find(|(text, _)| text == label)
            .unwrap_or_else(|| {
                panic!(
                    "Missing {label}; available: {:?}",
                    nodes.iter().map(|(s, _)| s).collect::<Vec<_>>()
                )
            })
            .1;
        frame(
            view,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
        frame(
            view,
            ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
        );
    }
    #[test]
    fn workshop_controls_create_edit_copy_and_persist_macros() {
        let (dir, mut view, ctx) = fixture();
        click(&mut view, &ctx, "+ New macro");
        assert_eq!(view.tabs.len(), 1);
        frame(&mut view, &ctx, vec![]);
        frame(
            &mut view,
            &ctx,
            vec![egui::Event::Paste("#showtooltip\r\n/cast Fireball".into())],
        );
        assert_eq!(view.tabs[0].draft.body, "#showtooltip\n/cast Fireball");
        click(&mut view, &ctx, "Favorite");
        assert!(view.tabs[0].draft.favorite);
        click(&mut view, &ctx, "Copy macro");
        assert!(view.message.as_ref().unwrap().contains("copied"));
        assert!(view.save_if_dirty());
        let stored = Store::open(&dir.path().join("desktop.sqlite3"), "test")
            .unwrap()
            .list()
            .unwrap();
        assert_eq!(stored[0].document, view.tabs[0].draft);
        click(&mut view, &ctx, "New from template");
        assert_eq!(view.tabs.len(), 2);
        assert!(view.tabs[1].draft.body.contains("@mouseover"));
        view.apply(
            &ctx,
            Actions {
                archive: true,
                ..Default::default()
            },
        );
        assert_eq!(view.tabs.len(), 1);
        let archived = view.entries.iter().find(|s| s.archived).unwrap().id;
        view.apply(
            &ctx,
            Actions {
                restore_archive: Some(archived),
                ..Default::default()
            },
        );
        assert_eq!(view.tabs.len(), 2);
        assert!(view.entries.iter().all(|s| !s.archived));
        let exported = view.export_text(true).unwrap();
        assert_eq!(view.import_text(&exported, "unused").unwrap(), 2);
        assert_eq!(view.entries.len(), 4);
        assert!(view.import_text("{\"format\":\"forever-atlas-macros\",\"version\":99,\"build\":\"test\",\"macros\":[]}","bad").is_err());
        assert_eq!(view.entries.len(), 4);
    }
    #[test]
    fn revisions_restore_without_discarding_current_edits_and_failed_saves_keep_tabs() {
        let (dir, mut view, ctx) = fixture();
        view.create(MacroDocument {
            name: "Original".into(),
            body: "/cast Fireball".into(),
            ..Default::default()
        });
        let original = view.tabs[0].draft.clone();
        view.tabs[0].draft.body = "/cast Frostbolt".into();
        view.apply(
            &ctx,
            Actions {
                restore_revision: Some(original.clone()),
                ..Default::default()
            },
        );
        assert_eq!(view.tabs[0].saved.revision, 3);
        let revisions = view.store.revisions(view.tabs[0].saved.id).unwrap();
        assert_eq!(revisions[1].document.body, "/cast Frostbolt");
        let mut other = Store::open(&dir.path().join("desktop.sqlite3"), "test").unwrap();
        let mut saved = other.list().unwrap().remove(0);
        other
            .save(
                &mut saved,
                &MacroDocument {
                    body: "/cast Polymorph".into(),
                    ..original
                },
            )
            .unwrap();
        view.tabs[0].draft.body = "/cast Blink".into();
        view.apply(
            &ctx,
            Actions {
                close: Some(0),
                ..Default::default()
            },
        );
        assert_eq!(view.tabs.len(), 1);
        assert_eq!(view.tabs[0].draft.body, "/cast Blink");
        assert!(view
            .error
            .as_ref()
            .unwrap()
            .contains("another Spellbook window"));
    }

    #[test]
    fn reopening_restores_tabs_preferences_and_unicode_cursors() {
        let (dir, mut view, _) = fixture();
        for name in ["First", "Second"] {
            view.create(MacroDocument {
                name: name.into(),
                body: "/say 猫 hello".into(),
                ..Default::default()
            });
        }
        view.tabs[0].editor.select(7..7);
        view.tabs[1].editor.select(10..10);
        view.active = 0;
        view.set_preferences(21.0, false);
        let mut state = view.session([1300.0, 800.0]);
        state.tabs.push(crate::session::OpenTab {
            id: 9999,
            cursor: 0,
        });
        drop(view);
        let mut reopened = MacrosView::open(&dir.path().join("desktop.sqlite3"), "test").unwrap();
        reopened.restore_session(&state);
        assert_eq!(reopened.tabs.len(), 2);
        assert_eq!(reopened.tabs[0].draft.name, "First");
        assert_eq!(reopened.tabs[0].editor.cursor, 7);
        assert_eq!(reopened.tabs[1].editor.cursor, 10);
        assert_eq!(reopened.active, 0);
        assert_eq!(reopened.preferences(), (21.0, false));
    }
}
