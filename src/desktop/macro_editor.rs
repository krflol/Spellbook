use super::{GOLD, INK, MUTED, PANEL, TEAL};
use crate::macros::{
    byte_index, char_index,
    language::{self, Catalogue, Completion, Diagnostic, Kind, Severity},
    normalize,
};
use eframe::egui::{
    self,
    text::{CCursor, CCursorRange, LayoutJob},
    Color32, FontId, Key, Modifiers, RichText,
};
use std::ops::Range;

pub const ERROR: Color32 = Color32::from_rgb(244, 130, 126);
const BLUE: Color32 = Color32::from_rgb(135, 184, 245);
const PURPLE: Color32 = Color32::from_rgb(197, 162, 238);

pub struct Editor {
    pub cursor: usize,
    pub selection: Range<usize>,
    pub font_size: f32,
    pub auto_pairs: bool,
    pub find_open: bool,
    pub replace_open: bool,
    pub query: String,
    replacement: String,
    match_case: bool,
    match_index: usize,
    focus_find: bool,
    goto_open: bool,
    goto_line: usize,
    suggestions: Vec<Completion>,
    suggestion_index: usize,
    scroll_suggestion: bool,
    pub completion_open: bool,
    pending_selection: Option<Range<usize>>,
    pub focus: bool,
}
impl Default for Editor {
    fn default() -> Self {
        Self {
            cursor: 0,
            selection: 0..0,
            font_size: 16.0,
            auto_pairs: true,
            find_open: false,
            replace_open: false,
            query: String::new(),
            replacement: String::new(),
            match_case: false,
            match_index: 0,
            focus_find: false,
            goto_open: false,
            goto_line: 1,
            suggestions: vec![],
            suggestion_index: 0,
            scroll_suggestion: false,
            completion_open: false,
            pending_selection: None,
            focus: false,
        }
    }
}
pub fn editor_id(id: i64) -> egui::Id {
    egui::Id::unique(("wow-macro-source", id))
}
impl Editor {
    pub fn show_completion(&mut self, text: &str, catalogue: &Catalogue) {
        self.suggestions = language::complete(text, self.cursor, catalogue);
        self.suggestion_index = 0;
        self.scroll_suggestion = true;
        self.completion_open = !self.suggestions.is_empty();
        self.focus = true;
    }
    pub fn select(&mut self, range: Range<usize>) {
        self.cursor = range.end;
        self.selection = range.clone();
        self.pending_selection = Some(range);
        self.focus = true;
    }
    pub fn go_to(&mut self, text: &str, line: usize, column: usize) {
        let start: usize = text
            .split_inclusive('\n')
            .take(line.saturating_sub(1))
            .map(|s| s.chars().count())
            .sum();
        let length = text
            .lines()
            .nth(line.saturating_sub(1))
            .map_or(0, |s| s.chars().count());
        let pos = (start + column.saturating_sub(1).min(length)).min(text.chars().count());
        self.select(pos..pos);
    }
    pub fn insert(&mut self, ctx: &egui::Context, id: i64, text: &mut String, insert: &str) {
        let start = byte_index(text, self.selection.start);
        let end = byte_index(text, self.selection.end);
        self.replace(ctx, id, text, start..end, insert);
    }
    fn replace(
        &mut self,
        ctx: &egui::Context,
        id: i64,
        text: &mut String,
        range: Range<usize>,
        insert: &str,
    ) {
        let mut next = text.clone();
        let cursor = char_index(text, range.start) + insert.chars().count();
        next.replace_range(range, insert);
        self.edit(ctx, id, text, next, cursor..cursor);
    }
    pub fn edit(
        &mut self,
        ctx: &egui::Context,
        id: i64,
        text: &mut String,
        next: String,
        selection: Range<usize>,
    ) {
        let id = editor_id(id);
        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        let mut undoer = state.undoer();
        undoer.add_undo(&(state.cursor.char_range().unwrap_or_default(), text.clone()));
        *text = next;
        let range = CCursorRange::two(CCursor::new(selection.start), CCursor::new(selection.end));
        undoer.feed_state(ctx.input(|i| i.time), &(range, text.clone()));
        undoer.add_undo(&(range, text.clone()));
        state.set_undoer(undoer);
        state.cursor.set_char_range(Some(range));
        state.store(ctx, id);
        self.select(selection);
    }
    fn undo(&mut self, ctx: &egui::Context, id: i64, text: &mut String, redo: bool) {
        let id = editor_id(id);
        if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
            let mut undoer = state.undoer();
            let current = (state.cursor.char_range().unwrap_or_default(), text.clone());
            let restored = if redo {
                undoer.redo(&current)
            } else {
                undoer.undo(&current)
            }
            .cloned();
            if let Some((range, body)) = restored {
                *text = body;
                state.cursor.set_char_range(Some(range));
                let selection = range.as_sorted_char_range();
                self.select(selection.start.into()..selection.end.into());
            }
            state.set_undoer(undoer);
            state.store(ctx, id);
        }
    }
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        id: i64,
        text: &mut String,
        catalogue: &Catalogue,
        diagnostics: &[Diagnostic],
    ) -> bool {
        let ctx = ui.ctx().clone();
        let before = text.clone();
        let editor_id = editor_id(id);
        let focused = ui.memory(|m| m.has_focus(editor_id));
        let mut request_completion = false;
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::F)) {
            self.find_open = true;
            self.focus_find = true;
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::H)) {
            self.find_open = true;
            self.replace_open = true;
            self.focus_find = true;
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::G)) {
            self.goto_open = !self.goto_open;
        }
        if focused {
            request_completion = ui.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Space));
            if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::D)) {
                let byte = byte_index(text, self.cursor);
                let start = text[..byte].rfind('\n').map_or(0, |i| i + 1);
                let end = text[byte..].find('\n').map_or(text.len(), |i| byte + i);
                let duplicate = format!("\n{}", &text[start..end]);
                self.replace(&ctx, id, text, end..end, &duplicate);
            }
        }
        if focused && self.completion_open && !self.suggestions.is_empty() {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
                self.suggestion_index = (self.suggestion_index + 1) % self.suggestions.len();
                self.scroll_suggestion = true;
            }
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
                self.suggestion_index =
                    (self.suggestion_index + self.suggestions.len() - 1) % self.suggestions.len();
                self.scroll_suggestion = true;
            }
            if ui.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Tab)
                    || i.consume_key(Modifiers::NONE, Key::Enter)
            }) {
                let suggestion =
                    self.suggestions[self.suggestion_index.min(self.suggestions.len() - 1)].clone();
                self.replace(&ctx, id, text, suggestion.range, &suggestion.insert);
                self.completion_open = false;
            }
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.completion_open {
                self.completion_open = false;
            } else {
                self.find_open = false;
                self.goto_open = false;
                self.focus = true;
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("Undo").on_hover_text("Ctrl+Z").clicked() {
                self.undo(&ctx, id, text, false);
            }
            if ui
                .small_button("Redo")
                .on_hover_text("Ctrl+Shift+Z / Ctrl+Y")
                .clicked()
            {
                self.undo(&ctx, id, text, true);
            }
            ui.separator();
            if ui
                .small_button("Find")
                .on_hover_text("Ctrl+F · Ctrl+H for replace")
                .clicked()
            {
                self.find_open = !self.find_open;
                self.focus_find = self.find_open;
            }
            if ui
                .small_button("Complete")
                .on_hover_text("Ctrl+Space")
                .clicked()
            {
                request_completion = true;
                self.focus = true;
            }
            ui.separator();
            ui.label(RichText::new("Text size").small().color(MUTED));
            ui.add(
                egui::DragValue::new(&mut self.font_size)
                    .range(12.0..=24.0)
                    .speed(0.25),
            );
            ui.checkbox(&mut self.auto_pairs, "Pair brackets")
                .on_hover_text("Pair [ ] in macro conditions and ( ) / { } in Lua.");
        });
        if self.find_open {
            self.find_bar(ui, id, text);
        }
        if self.goto_open {
            ui.horizontal(|ui| {
                ui.label("Go to line");
                ui.add(
                    egui::DragValue::new(&mut self.goto_line)
                        .range(1..=text.lines().count().max(1)),
                );
                if ui.small_button("Go").clicked() {
                    self.go_to(text, self.goto_line, 1);
                    self.goto_open = false;
                }
            });
        }
        ui.add_space(8.0);
        // Normalize pasted line endings before TextEdit receives the paste event.
        if focused {
            ui.input_mut(|i| {
                for event in &mut i.events {
                    if let egui::Event::Paste(value) = event {
                        *value = normalize(value);
                    }
                }
            });
        }
        // Pair only typed delimiters, never paste or IME input. Chat text stays literal.
        if focused && self.auto_pairs {
            let byte = byte_index(text, self.cursor);
            let start = text[..byte].rfind('\n').map_or(0, |i| i + 1);
            let line = &text[start..byte];
            let cmd = line.split_whitespace().next().unwrap_or("");
            let lua = matches!(cmd, "/run" | "/script")
                && !language::highlight(line)
                    .last()
                    .is_some_and(|s| matches!(s.kind, Kind::String | Kind::Comment));
            let conditional = language::command(cmd).is_some()
                && !matches!(
                    cmd,
                    "/say"
                        | "/s"
                        | "/party"
                        | "/p"
                        | "/raid"
                        | "/rw"
                        | "/yell"
                        | "/emote"
                        | "/whisper"
                        | "/w"
                        | "/guild"
                        | "/g"
                        | "/run"
                        | "/script"
                        | "/console"
                );
            let typed = ui.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| {
                        if let egui::Event::Text(s) = e {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            });
            if typed.len() == 1 {
                let value = &typed[0];
                let pair = if conditional && value == "[" {
                    Some("]")
                } else if lua && value == "(" {
                    Some(")")
                } else if lua && value == "{" {
                    Some("}")
                } else {
                    None
                };
                if let Some(close) = pair {
                    ui.input_mut(|i| {
                        i.events
                            .retain(|e| !matches!(e,egui::Event::Text(s) if s==value))
                    });
                    let range = byte_index(text, self.selection.start)
                        ..byte_index(text, self.selection.end);
                    let selected = text[range.clone()].to_owned();
                    let pos = self.selection.start + 1;
                    self.replace(&ctx, id, text, range, &format!("{value}{selected}{close}"));
                    self.select(pos..pos + selected.chars().count());
                } else if ["]", ")", "}"].contains(&value.as_str())
                    && self.selection.is_empty()
                    && text[byte..].starts_with(value)
                    && (lua || conditional)
                {
                    ui.input_mut(|i| {
                        i.events
                            .retain(|e| !matches!(e,egui::Event::Text(s) if s==value))
                    });
                    self.select(self.cursor + 1..self.cursor + 1);
                }
            }
        }
        if let Some(range) = self.pending_selection.take() {
            let mut state = egui::TextEdit::load_state(&ctx, editor_id).unwrap_or_default();
            state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(range.start),
                CCursor::new(range.end),
            )));
            state.store(&ctx, editor_id);
        }
        let hits = if self.find_open {
            language::find_matches(text, &self.query, self.match_case)
        } else {
            vec![]
        };
        let font = self.font_size;
        let cursor = self.cursor;
        let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, _wrap: f32| {
            ui.fonts_mut(|f| f.layout_job(layout(buffer.as_str(), font, cursor, &hits)))
        };
        let height = ui.available_height().max(150.0);
        let popup_layer =
            egui::LayerId::new(egui::Order::Foreground, editor_id.with("completions"));
        let pointer_on_popup = self.completion_open
            && ctx.pointer_interact_pos().and_then(|p| ctx.layer_id_at(p)) == Some(popup_layer);
        let surrender_focus_on = ctx.options(|o| o.input_options.surrender_focus_on);
        if pointer_on_popup {
            ctx.options_mut(|o| o.input_options.surrender_focus_on = egui::SurrenderFocusOn::Never);
        }
        let output = egui::Frame::new()
            .fill(Color32::from_rgb(13, 18, 24))
            .corner_radius(6.0)
            .inner_margin(10)
            .show(ui, |ui| {
                egui::ScrollArea::both()
                    .id_salt(("macro-scroll", id))
                    .auto_shrink([false, false])
                    .max_height(height - 20.0)
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            let (gutter, _) = ui.allocate_exact_size(
                                egui::vec2(42.0, (text.lines().count() + 1) as f32 * (font + 8.0)),
                                egui::Sense::hover(),
                            );
                            let output = egui::TextEdit::multiline(text)
                                .id(editor_id)
                                .code_editor()
                                .font(FontId::monospace(font))
                                .lock_focus(true)
                                .event_filter(egui::EventFilter {
                                    tab: true,
                                    escape: true,
                                    horizontal_arrows: true,
                                    vertical_arrows: true,
                                })
                                .frame(egui::Frame::NONE)
                                .margin(egui::Margin::ZERO)
                                .desired_width(ui.available_width().max(240.0))
                                .desired_rows(((height - 40.0) / (font + 8.0)).max(5.0) as usize)
                                .layouter(&mut layouter)
                                .show(ui);
                            let current_line =
                                text[..byte_index(text, self.cursor)].matches('\n').count() + 1;
                            for (index, row) in output.galley.rows.iter().enumerate() {
                                let y = output.galley_pos.y + row.pos.y;
                                let rect = egui::Rect::from_min_size(
                                    egui::pos2(gutter.left(), y),
                                    egui::vec2(40.0, font + 8.0),
                                );
                                if !ui.is_rect_visible(rect) {
                                    continue;
                                }
                                let issue = diagnostics.iter().find(|d| d.line == index + 1);
                                let color = issue.map_or(
                                    if current_line == index + 1 {
                                        GOLD
                                    } else {
                                        MUTED
                                    },
                                    |d| severity_color(d.severity),
                                );
                                ui.painter().text(
                                    egui::pos2(gutter.right() - 8.0, y + 4.0),
                                    egui::Align2::RIGHT_TOP,
                                    (index + 1).to_string(),
                                    FontId::monospace(font - 2.0),
                                    color,
                                );
                                if let Some(diagnostic) = issue {
                                    ui.painter().circle_filled(
                                        egui::pos2(gutter.left() + 3.0, y + font * 0.5 + 4.0),
                                        2.5,
                                        color,
                                    );
                                    ui.interact(
                                        rect,
                                        editor_id.with(("line", index)),
                                        egui::Sense::hover(),
                                    )
                                    .on_hover_text(&diagnostic.message);
                                }
                            }
                            output
                        })
                        .inner
                    })
                    .inner
            })
            .inner;
        ctx.options_mut(|o| o.input_options.surrender_focus_on = surrender_focus_on);
        if self.focus {
            output.response.request_focus();
            self.focus = false;
        }
        if let Some(range) = output.cursor_range {
            self.cursor = range.primary.index.into();
            let selection = range.as_sorted_char_range();
            self.selection = selection.start.into()..selection.end.into();
        }
        let has_focus = output.response.has_focus();
        if (has_focus && output.response.changed()) || request_completion {
            self.suggestions = language::complete(text, self.cursor, catalogue);
            self.suggestion_index = 0;
            self.scroll_suggestion = true;
            let prefix = &text[..byte_index(text, self.cursor)];
            let has_prefix = !prefix.rsplit('\n').next().unwrap_or("").trim().is_empty();
            self.completion_open =
                !self.suggestions.is_empty() && (request_completion || has_prefix);
        } else if self.completion_open && has_focus {
            // Cursor movement changes the replacement range even without editing text.
            self.suggestions = language::complete(text, self.cursor, catalogue);
            if self.suggestions.is_empty() {
                self.completion_open = false;
            }
            self.suggestion_index = self
                .suggestion_index
                .min(self.suggestions.len().saturating_sub(1));
        }
        if self.completion_open && (has_focus || request_completion || pointer_on_popup) {
            let caret = output
                .galley
                .pos_from_cursor(CCursor::new(self.cursor))
                .translate(output.galley_pos.to_vec2());
            if output.text_clip_rect.intersects(caret) {
                self.popup(ui, id, text, caret);
            }
        } else if !has_focus && ui.input(|i| i.pointer.any_pressed()) {
            self.completion_open = false;
        }
        if !self.completion_open && output.response.hovered() {
            if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                let hovered = output.galley.cursor_from_pos(pointer - output.galley_pos);
                let byte = byte_index(text, hovered.index.into());
                if let Some(span) = language::highlight(text)
                    .into_iter()
                    .find(|s| s.range.contains(&byte))
                {
                    let word = &text[span.range];
                    let reference = language::COMMANDS
                        .iter()
                        .chain(language::CONDITIONS)
                        .chain(language::TARGETS)
                        .chain(language::LUA)
                        .find(|r| r.name.eq_ignore_ascii_case(word));
                    if let Some(reference) = reference {
                        output.response.response.on_hover_text(format!(
                            "{}\n{}",
                            reference.description, reference.example
                        ));
                    }
                }
            }
        }
        before != *text
    }
    fn find_bar(&mut self, ui: &mut egui::Ui, id: i64, text: &mut String) {
        let ctx = ui.ctx().clone();
        let mut step = 0i32;
        let mut replace = false;
        let mut replace_all = false;
        ui.horizontal_wrapped(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .id_salt(("macro-find", id))
                    .hint_text("Find in this macro")
                    .desired_width(180.0),
            );
            if self.focus_find {
                response.request_focus();
                self.focus_find = false;
            }
            if response.changed() {
                self.match_index = 0;
            }
            if response.has_focus() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))
            {
                step = 1;
            }
            if ui.small_button("Previous").clicked() {
                step = -1;
            }
            if ui.small_button("Next").clicked() {
                step = 1;
            }
            ui.checkbox(&mut self.match_case, "Aa")
                .on_hover_text("Match case");
            ui.checkbox(&mut self.replace_open, "Replace");
            if ui.small_button("Close find").clicked() {
                self.find_open = false;
                self.focus = true;
            }
        });
        if self.replace_open {
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.replacement)
                        .hint_text("Replace with")
                        .desired_width(180.0),
                );
                replace = ui.small_button("Replace one").clicked();
                replace_all = ui.small_button("Replace all").clicked();
            });
        }
        let hits = language::find_matches(text, &self.query, self.match_case);
        self.match_index = self.match_index.min(hits.len().saturating_sub(1));
        ui.label(
            RichText::new(if hits.is_empty() {
                "No matches".into()
            } else {
                format!("{} of {} matches", self.match_index + 1, hits.len())
            })
            .small()
            .color(MUTED),
        );
        if hits.is_empty() {
            return;
        }
        if step != 0 {
            // Start relative to the caret so the first Next selects the first following match.
            let byte = byte_index(text, self.cursor);
            self.match_index = if step > 0 {
                hits.iter().position(|r| r.start >= byte).unwrap_or(0)
            } else {
                hits.iter()
                    .rposition(|r| r.end < byte)
                    .unwrap_or(hits.len() - 1)
            };
            let range = &hits[self.match_index];
            self.select(char_index(text, range.start)..char_index(text, range.end));
        }
        if replace_all {
            let mut next = text.clone();
            for range in hits.iter().rev() {
                next.replace_range(range.clone(), &self.replacement);
            }
            self.edit(&ctx, id, text, next, 0..0);
        } else if replace {
            let replacement = self.replacement.clone();
            self.replace(&ctx, id, text, hits[self.match_index].clone(), &replacement);
        }
    }
    fn popup(&mut self, ui: &egui::Ui, id: i64, text: &mut String, caret: egui::Rect) {
        let ctx = ui.ctx();
        let screen = ctx.content_rect();
        let width = 370.0f32.min(screen.width() - 20.0);
        let height = 295.0;
        let pos = egui::pos2(
            caret
                .left()
                .min(screen.right() - width - 12.0)
                .max(screen.left() + 8.0),
            if caret.bottom() + height < screen.bottom() {
                caret.bottom() + 4.0
            } else {
                (caret.top() - height).max(screen.top() + 8.0)
            },
        );
        let mut chosen = None;
        egui::Area::new(editor_id(id).with("completions"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(PANEL)
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(64, 83, 97)))
                    .show(ui, |ui| {
                        ui.set_width(width);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("SUGGESTIONS").small().color(TEAL));
                            ui.label(
                                RichText::new("Up / Down  ·  Tab / Enter")
                                    .small()
                                    .color(MUTED),
                            );
                        });
                        egui::ScrollArea::vertical()
                            .id_salt(("completion-list", id))
                            .max_height(176.0)
                            .show(ui, |ui| {
                                for (index, suggestion) in self.suggestions.iter().enumerate() {
                                    let response = ui.selectable_label(
                                        index == self.suggestion_index,
                                        format!("{}    {}", suggestion.label, suggestion.kind),
                                    );
                                    if index == self.suggestion_index && self.scroll_suggestion {
                                        response.scroll_to_me(Some(egui::Align::Center));
                                    }
                                    if response.clicked() {
                                        chosen = Some(index);
                                    }
                                }
                            });
                        ui.separator();
                        if let Some(selected) = self.suggestions.get(self.suggestion_index) {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&selected.detail).size(11.0).color(MUTED),
                                )
                                .wrap(),
                            );
                        }
                    });
            });
        self.scroll_suggestion = false;
        if let Some(index) = chosen {
            let suggestion = self.suggestions[index].clone();
            self.replace(ctx, id, text, suggestion.range, &suggestion.insert);
            self.completion_open = false;
        }
    }
}

pub fn severity_color(severity: Severity) -> Color32 {
    match severity {
        Severity::Error => ERROR,
        Severity::Warning => GOLD,
        Severity::Info => BLUE,
    }
}

fn layout(text: &str, font: f32, cursor: usize, matches: &[Range<usize>]) -> LayoutJob {
    let byte = byte_index(text, cursor);
    let line_start = text[..byte].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[byte..].find('\n').map_or(text.len(), |i| byte + i);
    let spans = language::highlight(text);
    let mut brackets = Vec::new();
    let delimiter = spans
        .iter()
        .filter(|s| s.kind == Kind::Operator && s.range.len() == 1)
        .find(|s| s.range.start == byte || s.range.end == byte)
        .filter(|s| "[](){}".contains(&text[s.range.clone()]));
    if let Some(delimiter) = delimiter {
        let ch = text.as_bytes()[delimiter.range.start] as char;
        let (open, close, forward) = match ch {
            '[' => ('[', ']', true),
            ']' => ('[', ']', false),
            '(' => ('(', ')', true),
            ')' => ('(', ')', false),
            '{' => ('{', '}', true),
            _ => ('{', '}', false),
        };
        let mut candidates: Vec<_> = spans
            .iter()
            .filter(|s| {
                s.kind == Kind::Operator
                    && s.range.len() == 1
                    && if forward {
                        s.range.start >= delimiter.range.start
                    } else {
                        s.range.start <= delimiter.range.start
                    }
            })
            .collect();
        if !forward {
            candidates.reverse();
        }
        let mut depth = 0i32;
        for candidate in candidates {
            let c = text.as_bytes()[candidate.range.start] as char;
            if c == if forward { open } else { close } {
                depth += 1;
            }
            if c == if forward { close } else { open } {
                depth -= 1;
                if depth == 0 {
                    brackets = vec![delimiter.range.start, candidate.range.start];
                    break;
                }
            }
        }
    }
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    for span in spans {
        let color = match span.kind {
            Kind::Command => BLUE,
            Kind::Directive => GOLD,
            Kind::Condition => TEAL,
            Kind::Target => Color32::from_rgb(238, 166, 112),
            Kind::String => Color32::from_rgb(177, 207, 142),
            Kind::Number => Color32::from_rgb(238, 177, 130),
            Kind::Keyword => PURPLE,
            Kind::Function => BLUE,
            Kind::Comment => MUTED,
            Kind::Operator => Color32::from_rgb(192, 201, 213),
            Kind::Text => INK,
        };
        // Split at search boundaries so highlights never color the wrong UTF-8 byte.
        let mut cuts = vec![span.range.start, span.range.end];
        for range in matches {
            for cut in [range.start, range.end] {
                if span.range.contains(&cut) {
                    cuts.push(cut);
                }
            }
        }
        cuts.sort_unstable();
        cuts.dedup();
        for range in cuts.windows(2) {
            let start = range[0];
            let end = range[1];
            let background = if matches.iter().any(|r| r.contains(&start)) {
                Color32::from_rgb(97, 76, 34)
            } else if brackets.contains(&start) {
                Color32::from_rgb(68, 56, 89)
            } else if start >= line_start && start < line_end {
                Color32::from_rgb(23, 31, 41)
            } else {
                Color32::TRANSPARENT
            };
            job.append(
                &text[start..end],
                0.0,
                egui::TextFormat {
                    font_id: FontId::monospace(font),
                    color,
                    background,
                    line_height: Some(font + 8.0),
                    ..Default::default()
                },
            );
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        editor: &mut Editor,
        ctx: &egui::Context,
        text: &mut String,
        events: Vec<egui::Event>,
    ) -> Vec<(String, egui::Pos2)> {
        let catalogue = Catalogue {
            symbols: vec![language::Symbol {
                name: "Fireball".into(),
                folded: "fireball".into(),
                id: 133,
                kind: language::SymbolKind::Spell,
            }],
        };
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 640.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    editor.ui(ui, 1, text, &catalogue, &[]);
                });
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
    fn click(editor: &mut Editor, ctx: &egui::Context, text: &mut String, label: &str) {
        frame(editor, ctx, text, vec![]);
        let nodes = frame(editor, ctx, text, vec![]);
        let pos = nodes
            .iter()
            .find(|(s, _)| s == label)
            .unwrap_or_else(|| panic!("Missing {label}: {nodes:?}"))
            .1;
        frame(
            editor,
            ctx,
            text,
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
            editor,
            ctx,
            text,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
        );
    }
    #[test]
    fn suggestion_mouse_click_and_replace_all_keep_focus_and_undo_history() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut editor = Editor::default();
        let mut text = "/cast Fi".to_string();
        editor.select(8..8);
        frame(&mut editor, &ctx, &mut text, vec![]);
        frame(
            &mut editor,
            &ctx,
            &mut text,
            key(
                Key::Space,
                Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            ),
        );
        click(&mut editor, &ctx, &mut text, "Fireball    spell");
        assert_eq!(text, "/cast Fireball");
        frame(&mut editor, &ctx, &mut text, vec![]);
        assert!(ctx.memory(|m| m.has_focus(editor_id(1))));
        editor.find_open = true;
        editor.replace_open = true;
        editor.query = "fireball".into();
        editor.replacement = "Éclair de givre".into();
        click(&mut editor, &ctx, &mut text, "Replace all");
        assert_eq!(text, "/cast Éclair de givre");
        frame(
            &mut editor,
            &ctx,
            &mut text,
            key(
                Key::Z,
                Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            ),
        );
        assert_eq!(text, "/cast Fireball");
    }
    fn key(key: Key, modifiers: Modifiers) -> Vec<egui::Event> {
        vec![
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            },
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers,
            },
        ]
    }
    #[test]
    fn keyboard_completion_pairs_and_native_undo_work_in_the_editor() {
        let ctx = egui::Context::default();
        let mut editor = Editor {
            focus: true,
            ..Default::default()
        };
        let mut text = String::new();
        frame(&mut editor, &ctx, &mut text, vec![]);
        frame(
            &mut editor,
            &ctx,
            &mut text,
            vec![egui::Event::Text("/ca".into())],
        );
        assert!(editor.completion_open);
        frame(&mut editor, &ctx, &mut text, key(Key::Tab, Modifiers::NONE));
        assert_eq!(text, "/cast");
        frame(
            &mut editor,
            &ctx,
            &mut text,
            vec![egui::Event::Text(" ".into())],
        );
        frame(
            &mut editor,
            &ctx,
            &mut text,
            vec![egui::Event::Text("[".into())],
        );
        assert_eq!(text, "/cast []");
        assert_eq!(editor.cursor, 7);
        frame(
            &mut editor,
            &ctx,
            &mut text,
            vec![egui::Event::Text("@m".into())],
        );
        frame(&mut editor, &ctx, &mut text, key(Key::Tab, Modifiers::NONE));
        assert_eq!(text, "/cast [@mouseover]");
        frame(
            &mut editor,
            &ctx,
            &mut text,
            vec![egui::Event::Text("]".into())],
        );
        assert_eq!(text, "/cast [@mouseover]");
        frame(
            &mut editor,
            &ctx,
            &mut text,
            vec![egui::Event::Text(" Fi".into())],
        );
        frame(&mut editor, &ctx, &mut text, key(Key::Tab, Modifiers::NONE));
        assert_eq!(text, "/cast [@mouseover] Fireball");
        frame(
            &mut editor,
            &ctx,
            &mut text,
            key(
                Key::Z,
                Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            ),
        );
        assert_eq!(text, "/cast [@mouseover] Fi");
        frame(
            &mut editor,
            &ctx,
            &mut text,
            key(Key::Escape, Modifiers::NONE),
        );
        assert!(!editor.completion_open, "Escape did not dismiss: {text:?}");
        frame(&mut editor, &ctx, &mut text, key(Key::End, Modifiers::NONE));
        assert!(
            !editor.completion_open,
            "End reopened suggestions: {text:?}"
        );
        frame(
            &mut editor,
            &ctx,
            &mut text,
            key(Key::Enter, Modifiers::NONE),
        );
        assert!(
            text.ends_with('\n'),
            "Enter produced {text:?}; focus: {:?}",
            ctx.memory(|m| m.focused())
        );
        assert!(
            !editor.completion_open,
            "Blank lines must not intercept Enter with a suggestion"
        );
    }
    #[test]
    fn typing_and_completion_replace_a_selection_without_losing_unicode() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        let mut text = "/say Éclair\n/cast Fi".to_string();
        editor.select(text.chars().count()..text.chars().count());
        frame(&mut editor, &ctx, &mut text, vec![]);
        frame(
            &mut editor,
            &ctx,
            &mut text,
            key(
                Key::Space,
                Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            ),
        );
        assert!(editor.completion_open);
        frame(&mut editor, &ctx, &mut text, key(Key::Tab, Modifiers::NONE));
        assert_eq!(text, "/say Éclair\n/cast Fireball");
        editor.select(5..11);
        frame(&mut editor, &ctx, &mut text, vec![]);
        frame(
            &mut editor,
            &ctx,
            &mut text,
            vec![egui::Event::Text("Bonjour".into())],
        );
        assert_eq!(text, "/say Bonjour\n/cast Fireball");
    }
    #[test]
    fn programmatic_completion_and_replace_are_undoable() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        let mut text = "/cast Fi".to_string();
        editor.replace(&ctx, 1, &mut text, 6..8, "Fireball");
        assert_eq!(text, "/cast Fireball");
        editor.undo(&ctx, 1, &mut text, false);
        assert_eq!(text, "/cast Fi");
        editor.undo(&ctx, 1, &mut text, true);
        assert_eq!(text, "/cast Fireball");
        editor.edit(&ctx, 1, &mut text, "/cast Éclair".into(), 11..11);
        editor.undo(&ctx, 1, &mut text, false);
        assert_eq!(text, "/cast Fireball");
    }
}
