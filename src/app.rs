use std::{
    collections::HashSet,
    error::Error as StdError,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use eframe::egui::{self, Color32, RichText};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use tokio::runtime::Runtime;

use crate::{
    semantic::{self, SearchUpdate},
    storage::{Note, Settings, Store},
    tray::{Tray, TrayAction},
};

const INK: Color32 = Color32::from_rgb(236, 239, 244);
const MUTED: Color32 = Color32::from_rgb(156, 165, 178);
const SUBTLE: Color32 = Color32::from_rgb(113, 123, 138);
const PAPER: Color32 = Color32::from_rgb(27, 30, 36);
const NOTE_PAPER: Color32 = Color32::from_rgb(30, 34, 41);
const FIELD: Color32 = Color32::from_rgb(37, 42, 50);
const HOVER: Color32 = Color32::from_rgb(42, 48, 58);
const SELECTED: Color32 = Color32::from_rgb(43, 53, 68);
const BORDER: Color32 = Color32::from_rgb(58, 64, 75);
const ACCENT: Color32 = Color32::from_rgb(158, 177, 238);
const ERROR: Color32 = Color32::from_rgb(242, 143, 143);

fn configure_style(ctx: &egui::Context) {
    #[cfg(target_os = "windows")]
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Segoe UI".into(),
            Arc::new(egui::FontData::from_owned(bytes)),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .expect("default proportional font family")
            .insert(0, "Segoe UI".into());
        ctx.set_fonts(fonts);
    }

    ctx.set_theme(egui::Theme::Dark);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(11.0, 7.0);
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = PAPER;
    style.visuals.window_fill = PAPER;
    style.visuals.extreme_bg_color = FIELD;
    style.visuals.override_text_color = Some(INK);
    style.visuals.selection.bg_fill = SELECTED;
    style.visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);
    style.visuals.widgets.inactive.bg_fill = FIELD;
    style.visuals.widgets.inactive.weak_bg_fill = FIELD;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.hovered.bg_fill = HOVER;
    style.visuals.widgets.hovered.weak_bg_fill = HOVER;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.active.bg_fill = SELECTED;
    style.visuals.widgets.active.weak_bg_fill = SELECTED;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
    ctx.set_style_of(egui::Theme::Dark, style);
}

fn note_viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("notiz_note_editor")
}

fn search_viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("notiz_note_search")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Notes,
    Settings,
}

#[derive(Clone, Copy)]
enum HotkeyAction {
    NewNote,
    Search,
}

struct Hotkeys {
    _manager: Option<GlobalHotKeyManager>,
    receiver: Receiver<HotkeyAction>,
    running: Arc<AtomicBool>,
    listener: Option<JoinHandle<()>>,
    new_registered: bool,
    search_registered: bool,
    alt_new_registered: bool,
    alt_search_registered: bool,
    error: Option<String>,
}

impl Hotkeys {
    fn new(ctx: egui::Context) -> Self {
        let (sender, receiver) = mpsc::channel();
        let mut errors = Vec::new();
        let manager = match GlobalHotKeyManager::new() {
            Ok(manager) => Some(manager),
            Err(error) => {
                errors.push(format!("Global shortcuts could not start: {error}"));
                None
            }
        };
        let new_key = HotKey::new(Some(Modifiers::CONTROL), Code::KeyN);
        let search_key = HotKey::new(Some(Modifiers::CONTROL), Code::KeyM);
        let alt_new_key = HotKey::new(Some(Modifiers::ALT), Code::KeyN);
        let alt_search_key = HotKey::new(Some(Modifiers::ALT), Code::KeyM);
        let (mut new_registered, mut search_registered) = (false, false);
        let (mut alt_new_registered, mut alt_search_registered) = (false, false);
        if let Some(manager) = &manager {
            match manager.register(new_key) {
                Ok(()) => new_registered = true,
                Err(error) => errors.push(format!("Ctrl+N is unavailable: {error}")),
            }
            match manager.register(search_key) {
                Ok(()) => search_registered = true,
                Err(error) => errors.push(format!("Ctrl+M is unavailable: {error}")),
            }
            match manager.register(alt_new_key) {
                Ok(()) => alt_new_registered = true,
                Err(error) => errors.push(format!("Alt+N is unavailable: {error}")),
            }
            match manager.register(alt_search_key) {
                Ok(()) => alt_search_registered = true,
                Err(error) => errors.push(format!("Alt+M is unavailable: {error}")),
            }
        }

        let running = Arc::new(AtomicBool::new(true));
        let listener =
            if new_registered || search_registered || alt_new_registered || alt_search_registered {
                let running = Arc::clone(&running);
                Some(std::thread::spawn(move || {
                    while running.load(Ordering::Relaxed) {
                        if let Ok(event) =
                            GlobalHotKeyEvent::receiver().recv_timeout(Duration::from_millis(200))
                        {
                            if event.state != HotKeyState::Pressed {
                                continue;
                            }
                            let action = if (new_registered && event.id == new_key.id)
                                || (alt_new_registered && event.id == alt_new_key.id)
                            {
                                Some(HotkeyAction::NewNote)
                            } else if (search_registered && event.id == search_key.id)
                                || (alt_search_registered && event.id == alt_search_key.id)
                            {
                                Some(HotkeyAction::Search)
                            } else {
                                None
                            };
                            if let Some(action) = action {
                                if sender.send(action).is_err() {
                                    break;
                                }
                                ctx.request_repaint_of(egui::ViewportId::ROOT);
                            }
                        }
                    }
                }))
            } else {
                None
            };

        Self {
            _manager: manager,
            receiver,
            running,
            listener,
            new_registered,
            search_registered,
            alt_new_registered,
            alt_search_registered,
            error: (!errors.is_empty()).then(|| errors.join(" ")),
        }
    }
}

impl Drop for Hotkeys {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(listener) = self.listener.take() {
            let _ = listener.join();
        }
    }
}

pub struct NotizApp {
    runtime: Runtime,
    store: Store,
    notes: Vec<Note>,
    settings: Settings,
    page: Page,
    selected: Option<i64>,
    draft: String,
    dirty: bool,
    search: String,
    error: Option<String>,
    show_editor: bool,
    focus_editor: bool,
    focus_editor_window: bool,
    focus_search: bool,
    focus_settings: bool,
    focus_list_id: Option<i64>,
    search_has_focus: bool,
    list_has_focus: bool,
    hotkeys: Hotkeys,
    tray: Tray,
    search_visible: bool,
    focus_search_window: bool,
    quitting: bool,
    search_updates: Receiver<SearchUpdate>,
    search_sender: mpsc::Sender<SearchUpdate>,
    active_search: Arc<AtomicU64>,
    pending_search: Option<Instant>,
    semantic_matches: HashSet<i64>,
    semantic_pending: bool,
    semantic_error: Option<String>,
}

enum SidebarAction {
    Create,
    Select(i64),
    Open(i64),
    Settings,
}

fn window_chrome(ui: &mut egui::Ui, title: &str) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::drag());
    let painter = ui.painter();
    painter.text(
        rect.left_center() + egui::vec2(18.0, 0.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(12.5),
        MUTED,
    );
    painter.text(
        rect.right_center() - egui::vec2(18.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        "Ctrl+W",
        egui::FontId::proportional(11.0),
        SUBTLE,
    );
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        egui::Stroke::new(1.0, BORDER),
    );
    if response.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
}

fn resize_grip(ui: &egui::Ui) {
    let corner = ui.max_rect().right_bottom();
    let rect = egui::Rect::from_min_max(corner - egui::vec2(20.0, 20.0), corner);
    let response = ui
        .interact(rect, ui.id().with("resize_grip"), egui::Sense::drag())
        .on_hover_cursor(egui::CursorIcon::ResizeNwSe);
    if response.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::BeginResize(
                egui::ResizeDirection::SouthEast,
            ));
    }
    let painter = ui.painter();
    for offset in [7.0, 11.0, 15.0] {
        painter.line_segment(
            [
                corner - egui::vec2(offset + 4.0, 4.0),
                corner - egui::vec2(4.0, offset + 4.0),
            ],
            egui::Stroke::new(1.0, SUBTLE),
        );
    }
}

impl NotizApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        runtime: Runtime,
        store: Store,
        notes: Vec<Note>,
        settings: Settings,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        configure_style(&cc.egui_ctx);
        let tray = Tray::new(cc.egui_ctx.clone())?;

        let selected = notes.first().map(|note| note.id);
        let draft = notes
            .first()
            .map(|note| note.body.clone())
            .unwrap_or_default();
        let (search_sender, search_updates) = mpsc::channel();
        Ok(Self {
            runtime,
            store,
            notes,
            settings,
            page: Page::Notes,
            selected,
            draft,
            dirty: false,
            search: String::new(),
            error: None,
            show_editor: false,
            focus_editor: false,
            focus_editor_window: false,
            focus_search: false,
            focus_settings: false,
            focus_list_id: None,
            search_has_focus: false,
            list_has_focus: false,
            hotkeys: Hotkeys::new(cc.egui_ctx.clone()),
            tray,
            search_visible: false,
            focus_search_window: false,
            quitting: false,
            search_updates,
            search_sender,
            active_search: Arc::new(AtomicU64::new(0)),
            pending_search: None,
            semantic_matches: HashSet::new(),
            semantic_pending: false,
            semantic_error: None,
        })
    }

    fn reload(&mut self) -> bool {
        match self.runtime.block_on(self.store.list()) {
            Ok(notes) => {
                self.notes = notes;
                self.error = None;
                true
            }
            Err(error) => {
                self.error = Some(format!("Could not load notes: {error}"));
                false
            }
        }
    }

    fn save_current(&mut self) -> bool {
        if !self.dirty {
            return true;
        }
        let Some(id) = self.selected else {
            return true;
        };
        match self.runtime.block_on(self.store.update(id, &self.draft)) {
            Ok(()) => {
                self.dirty = false;
                if self.reload() {
                    self.invalidate_semantic();
                    true
                } else {
                    false
                }
            }
            Err(error) => {
                self.error = Some(format!("Could not save note: {error}. Try again."));
                false
            }
        }
    }

    fn select(&mut self, id: i64) {
        if self.selected == Some(id) {
            return;
        }
        if self.show_editor && self.draft.trim().is_empty() {
            if !self.delete_selected() {
                return;
            }
        } else if !self.save_current() {
            return;
        }
        if let Some(note) = self.notes.iter().find(|note| note.id == id) {
            self.selected = Some(id);
            self.draft.clone_from(&note.body);
        }
    }

    fn create(&mut self) {
        if self.show_editor && !self.finish_current_note() {
            return;
        }
        if !self.save_current() {
            return;
        }
        match self.runtime.block_on(self.store.create()) {
            Ok(id) => {
                if self.reload() {
                    self.page = Page::Notes;
                    self.search.clear();
                    self.invalidate_semantic();
                    self.select(id);
                    self.open_editor();
                }
            }
            Err(error) => self.error = Some(format!("Could not create note: {error}")),
        }
    }

    fn delete_selected(&mut self) -> bool {
        let Some(id) = self.selected else {
            return true;
        };
        let visible = self.visible_ids();
        let replacement = visible
            .iter()
            .position(|candidate| *candidate == id)
            .and_then(|index| {
                visible
                    .get(index + 1)
                    .or_else(|| index.checked_sub(1).and_then(|i| visible.get(i)))
            })
            .copied();

        match self.runtime.block_on(self.store.delete(id)) {
            Ok(()) => {
                self.selected = None;
                self.draft.clear();
                self.dirty = false;
                if self.reload() {
                    self.selected = replacement
                        .filter(|next| self.notes.iter().any(|note| note.id == *next))
                        .or_else(|| self.notes.first().map(|note| note.id));
                    self.draft = self
                        .notes
                        .iter()
                        .find(|note| Some(note.id) == self.selected)
                        .map(|note| note.body.clone())
                        .unwrap_or_default();
                    if self.selected.is_none() {
                        self.show_editor = false;
                    }
                    self.focus_list_id = self.selected;
                    self.focus_search = self.selected.is_none();
                    self.invalidate_semantic();
                }
                true
            }
            Err(error) => {
                self.error = Some(format!("Could not delete note: {error}"));
                false
            }
        }
    }

    fn finish_current_note(&mut self) -> bool {
        if self.selected.is_some() && self.draft.trim().is_empty() {
            self.delete_selected()
        } else {
            self.save_current()
        }
    }

    fn close_editor(&mut self, ctx: &egui::Context) {
        if !self.finish_current_note() {
            return;
        }
        self.show_editor = false;
        self.focus_editor = false;
        self.focus_editor_window = false;
        ctx.send_viewport_cmd_to(note_viewport_id(), egui::ViewportCommand::Close);
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }

    fn invalidate_semantic(&mut self) {
        self.active_search.fetch_add(1, Ordering::Relaxed);
        self.semantic_matches.clear();
        self.semantic_error = None;
        self.semantic_pending = false;
        self.pending_search = (self.settings.semantic_enabled && !self.search.trim().is_empty())
            .then(|| Instant::now() + Duration::from_millis(350));
    }

    fn start_due_search(&mut self, ctx: &egui::Context) {
        let Some(due) = self.pending_search else {
            return;
        };
        if Instant::now() < due {
            ctx.request_repaint_after(due.saturating_duration_since(Instant::now()));
            return;
        }
        self.pending_search = None;
        let endpoint = self.settings.laya_endpoint.trim();
        if let Err(message) = semantic::validate_endpoint(endpoint) {
            self.semantic_error = Some(message);
            return;
        }
        self.semantic_pending = true;
        let query = self.search.trim().to_owned();
        let lower_query = query.to_lowercase();
        let candidates = self
            .notes
            .iter()
            .filter(|note| !note.body.to_lowercase().contains(&lower_query))
            .cloned()
            .collect();
        semantic::start_search(
            endpoint.to_owned(),
            query,
            candidates,
            self.active_search.load(Ordering::Relaxed),
            Arc::clone(&self.active_search),
            self.search_sender.clone(),
            ctx.clone(),
        );
    }

    fn poll_search_updates(&mut self) {
        for update in self.search_updates.try_iter() {
            match update {
                SearchUpdate::Match {
                    generation,
                    note_id,
                } if generation == self.active_search.load(Ordering::Relaxed) => {
                    self.semantic_matches.insert(note_id);
                }
                SearchUpdate::Finished { generation }
                    if generation == self.active_search.load(Ordering::Relaxed) =>
                {
                    self.semantic_pending = false;
                }
                SearchUpdate::Error {
                    generation,
                    message,
                } if generation == self.active_search.load(Ordering::Relaxed) => {
                    self.semantic_pending = false;
                    self.semantic_matches.clear();
                    self.semantic_error = Some(message);
                }
                _ => {}
            }
        }
    }

    fn visible_ids(&self) -> Vec<i64> {
        let query = self.search.trim().to_lowercase();
        self.notes
            .iter()
            .filter(|note| {
                query.is_empty()
                    || note.body.to_lowercase().contains(&query)
                    || self.semantic_matches.contains(&note.id)
            })
            .map(|note| note.id)
            .collect()
    }

    fn move_selection(&mut self, direction: isize) {
        let visible = self.visible_ids();
        if visible.is_empty() {
            return;
        }
        let current = self
            .selected
            .and_then(|id| visible.iter().position(|item| *item == id));
        let next = match current {
            Some(index) => {
                (index as isize + direction).clamp(0, visible.len() as isize - 1) as usize
            }
            None => 0,
        };
        let id = visible[next];
        self.select(id);
        self.focus_list_id = Some(id);
    }

    fn open_search(&mut self, ctx: &egui::Context) {
        self.save_current();
        self.page = Page::Notes;
        self.search.clear();
        self.invalidate_semantic();
        self.focus_search = true;
        self.focus_list_id = None;
        self.list_has_focus = false;
        self.show_search_window(ctx);
    }

    fn open_editor(&mut self) {
        if self.selected.is_some() {
            self.show_editor = true;
            self.focus_editor = true;
            self.focus_editor_window = true;
        }
    }

    fn show_search_window(&mut self, ctx: &egui::Context) {
        self.search_visible = true;
        self.focus_search_window = true;
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }

    fn hide_search_window(&mut self, ctx: &egui::Context) {
        self.search_visible = false;
        self.focus_search_window = false;
        ctx.send_viewport_cmd_to(search_viewport_id(), egui::ViewportCommand::Close);
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }

    fn new_note(&mut self, ctx: &egui::Context) {
        self.create();
        if self.show_editor {
            if self.search_visible {
                self.hide_search_window(ctx);
            }
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        } else {
            self.show_search_window(ctx);
        }
    }

    fn handle_global_hotkeys(&mut self, ctx: &egui::Context) {
        let global_actions: Vec<_> = self.hotkeys.receiver.try_iter().collect();
        for action in global_actions {
            match action {
                HotkeyAction::NewNote => self.new_note(ctx),
                HotkeyAction::Search => self.open_search(ctx),
            }
        }
    }

    fn handle_tray_actions(&mut self, ctx: &egui::Context) {
        for action in self.tray.actions() {
            match action {
                TrayAction::NewNote => self.new_note(ctx),
                TrayAction::Search => self.open_search(ctx),
                TrayAction::Quit => {
                    if self.show_editor && !self.finish_current_note() {
                        continue;
                    }
                    self.quitting = true;
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
                }
            }
        }
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let keys = ctx.input(|input| {
            (
                input.modifiers.command && input.key_pressed(egui::Key::N),
                input.modifiers.command && input.key_pressed(egui::Key::M),
                input.modifiers.command && input.key_pressed(egui::Key::Comma),
                input.key_pressed(egui::Key::Escape),
                input.key_pressed(egui::Key::ArrowDown),
                input.key_pressed(egui::Key::ArrowUp),
                input.key_pressed(egui::Key::Enter),
                input.key_pressed(egui::Key::Delete),
                input.modifiers.command && input.key_pressed(egui::Key::W),
                input.modifiers.alt && input.key_pressed(egui::Key::N),
                input.modifiers.alt && input.key_pressed(egui::Key::M),
            )
        });
        if keys.8 {
            self.hide_search_window(ctx);
            return;
        }
        if (keys.0 && !self.hotkeys.new_registered) || (keys.9 && !self.hotkeys.alt_new_registered)
        {
            self.new_note(ctx);
            return;
        }
        if (keys.1 && !self.hotkeys.search_registered)
            || (keys.10 && !self.hotkeys.alt_search_registered)
        {
            self.open_search(ctx);
            return;
        }
        if keys.2 {
            self.page = Page::Settings;
            self.focus_settings = true;
            return;
        }

        if self.page == Page::Settings {
            if keys.3 {
                self.page = Page::Notes;
                self.focus_list_id = self.selected;
            }
            return;
        }

        if self.list_has_focus {
            if keys.7 {
                self.delete_selected();
            } else if keys.4 {
                self.move_selection(1);
            } else if keys.5 {
                self.move_selection(-1);
            } else if keys.6 {
                self.open_editor();
            } else if keys.3 {
                self.focus_search = true;
            }
        } else if self.search_has_focus && (keys.4 || keys.5) {
            let visible = self.visible_ids();
            let id = if keys.4 {
                visible.first()
            } else {
                visible.last()
            };
            if let Some(id) = id.copied() {
                self.select(id);
                self.focus_list_id = Some(id);
            }
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) -> Option<SidebarAction> {
        let mut action = None;
        let search_response = egui::Frame::default()
            .fill(FIELD)
            .stroke(egui::Stroke::new(
                1.0,
                if self.search_has_focus {
                    ACCENT
                } else {
                    BORDER
                },
            ))
            .corner_radius(egui::CornerRadius::same(9))
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.add_sized(
                    [ui.available_width(), 23.0],
                    egui::TextEdit::singleline(&mut self.search)
                        .id_salt("note_search")
                        .font(egui::FontId::proportional(14.0))
                        .frame(egui::Frame::NONE)
                        .hint_text("Search notes"),
                )
            })
            .inner;
        let focus_search_now = self.focus_search;
        if focus_search_now {
            search_response.request_focus();
            self.focus_search = false;
        }
        self.search_has_focus = focus_search_now || search_response.has_focus();
        if search_response.changed() {
            self.invalidate_semantic();
        }

        if self.semantic_pending || self.pending_search.is_some() {
            ui.label(
                RichText::new("Checking semantic matches…")
                    .small()
                    .color(MUTED),
            );
        }
        if let Some(error) = &self.semantic_error {
            ui.label(
                RichText::new(format!("Semantic search: {error}"))
                    .small()
                    .color(ERROR),
            );
        }
        if let Some(error) = &self.error {
            ui.label(RichText::new(error).small().color(ERROR));
        }
        ui.add_space(11.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Notes").size(13.0).strong().color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(egui::Button::new(RichText::new("New note").color(ACCENT)).frame(false))
                    .clicked()
                {
                    action = Some(SidebarAction::Create);
                }
            });
        });
        ui.add_space(5.0);

        let visible: Vec<_> = self
            .visible_ids()
            .into_iter()
            .filter_map(|id| {
                self.notes.iter().find(|note| note.id == id).map(|note| {
                    (
                        id,
                        note_title(&note.body).to_owned(),
                        note_preview(&note.body).to_owned(),
                    )
                })
            })
            .collect();
        self.list_has_focus = false;
        if visible.is_empty() {
            ui.add_space(18.0);
            let message = if self.notes.is_empty() {
                "No notes yet. Press Ctrl+N to start."
            } else {
                "No matching notes."
            };
            ui.label(RichText::new(message).color(MUTED));
        } else {
            let list_height = (ui.available_height() - 55.0).max(80.0);
            egui::ScrollArea::vertical()
                .max_height(list_height)
                .show(ui, |ui| {
                    for (id, title, preview) in visible {
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 55.0),
                            egui::Sense::click(),
                        );
                        if self.focus_list_id == Some(id) {
                            response.request_focus();
                            self.focus_list_id = None;
                            self.list_has_focus = true;
                            self.search_has_focus = false;
                        }
                        if !focus_search_now {
                            self.list_has_focus |= response.has_focus();
                        }
                        let selected = self.selected == Some(id);
                        let fill = if selected || response.has_focus() {
                            SELECTED
                        } else if response.hovered() {
                            HOVER
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(8), fill);
                        if selected {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    rect.left_top() + egui::vec2(0.0, 12.0),
                                    egui::vec2(2.0, rect.height() - 24.0),
                                ),
                                egui::CornerRadius::same(1),
                                ACCENT,
                            );
                        }
                        let painter = ui
                            .painter()
                            .with_clip_rect(rect.shrink2(egui::vec2(13.0, 0.0)));
                        painter.text(
                            rect.left_top()
                                + egui::vec2(14.0, if preview.is_empty() { 27.0 } else { 18.0 }),
                            egui::Align2::LEFT_CENTER,
                            title,
                            egui::FontId::proportional(14.0),
                            INK,
                        );
                        if !preview.is_empty() {
                            painter.text(
                                rect.left_top() + egui::vec2(14.0, 38.0),
                                egui::Align2::LEFT_CENTER,
                                preview,
                                egui::FontId::proportional(12.0),
                                MUTED,
                            );
                        }
                        if response.clicked() {
                            action = Some(SidebarAction::Open(id));
                        } else if response.gained_focus() {
                            action = Some(SidebarAction::Select(id));
                        }
                        ui.add_space(4.0);
                    }
                });
        }
        ui.add_space(7.0);
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new(RichText::new("Settings").color(MUTED)).frame(false))
                .clicked()
            {
                action = Some(SidebarAction::Settings);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("↑↓ select  ·  Enter open")
                        .size(11.0)
                        .color(SUBTLE),
                );
            });
        });
        action
    }

    fn note_window(&mut self, ui: &mut egui::Ui) {
        let keys = ui.input(|input| {
            (
                input.viewport().close_requested()
                    || (input.modifiers.command && input.key_pressed(egui::Key::W)),
                input.modifiers.command && input.key_pressed(egui::Key::N),
                input.modifiers.command && input.key_pressed(egui::Key::M),
                input.modifiers.command && input.key_pressed(egui::Key::Comma),
                input.modifiers.alt && input.key_pressed(egui::Key::N),
                input.modifiers.alt && input.key_pressed(egui::Key::M),
            )
        });
        if keys.0 {
            self.close_editor(ui.ctx());
            return;
        }
        if (keys.1 && !self.hotkeys.new_registered) || (keys.4 && !self.hotkeys.alt_new_registered)
        {
            self.new_note(ui.ctx());
            return;
        }
        if (keys.2 && !self.hotkeys.search_registered)
            || (keys.5 && !self.hotkeys.alt_search_registered)
        {
            self.open_search(ui.ctx());
            return;
        }
        if keys.3 {
            self.page = Page::Settings;
            self.focus_settings = true;
            self.show_search_window(ui.ctx());
            return;
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(NOTE_PAPER)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .corner_radius(egui::CornerRadius::same(11))
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ui, |ui| {
                window_chrome(ui, "Notiz");
                egui::Frame::default()
                    .inner_margin(egui::Margin::symmetric(18, 14))
                    .show(ui, |ui| {
                        if let Some(error) = &self.error {
                            ui.colored_label(ERROR, error);
                            if self.dirty && ui.small_button("Retry save").clicked() {
                                self.save_current();
                            }
                        }
                        let response = ui.add_sized(
                            ui.available_size(),
                            egui::TextEdit::multiline(&mut self.draft)
                                .id_salt(self.selected)
                                .font(egui::FontId::proportional(15.0))
                                .text_color(INK)
                                .frame(egui::Frame::NONE)
                                .hint_text("Write a note…"),
                        );
                        if self.focus_editor {
                            response.request_focus();
                            self.focus_editor = false;
                        }
                        if response.changed() {
                            self.dirty = true;
                            self.save_current();
                            ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
                        }
                    });
                resize_grip(ui);
            });
    }

    fn search_window(&mut self, ui: &mut egui::Ui) {
        if ui.input(|input| input.viewport().close_requested()) {
            self.hide_search_window(ui.ctx());
            return;
        }

        self.handle_keyboard(ui.ctx());
        if !self.search_visible {
            return;
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(PAPER)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .corner_radius(egui::CornerRadius::same(11))
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ui, |ui| {
                window_chrome(
                    ui,
                    if self.page == Page::Settings {
                        "Settings"
                    } else {
                        "Notiz"
                    },
                );
                egui::Frame::default()
                    .inner_margin(egui::Margin::symmetric(18, 16))
                    .show(ui, |ui| {
                        if self.page == Page::Settings {
                            self.settings_page(ui);
                        } else if let Some(action) = self.sidebar(ui) {
                            match action {
                                SidebarAction::Create => self.new_note(ui.ctx()),
                                SidebarAction::Select(id) => self.select(id),
                                SidebarAction::Open(id) => {
                                    self.select(id);
                                    self.open_editor();
                                }
                                SidebarAction::Settings => {
                                    self.page = Page::Settings;
                                    self.focus_settings = true;
                                }
                            }
                        }
                    });
                resize_grip(ui);
            });
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Semantic search").size(18.0).strong());
        ui.label(
            RichText::new("Include notes that match the meaning of your search.")
                .size(13.0)
                .color(MUTED),
        );
        ui.add_space(14.0);

        let (enabled_changed, endpoint_changed) = egui::Frame::default()
            .fill(FIELD)
            .stroke(egui::Stroke::new(1.0, BORDER))
            .corner_radius(egui::CornerRadius::same(9))
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                let enabled_response = ui.checkbox(
                    &mut self.settings.semantic_enabled,
                    "Enable semantic search",
                );
                if self.focus_settings {
                    enabled_response.request_focus();
                    self.focus_settings = false;
                }
                ui.add_space(12.0);
                ui.label(RichText::new("Laya endpoint").size(12.0).color(MUTED));
                let endpoint_changed = ui
                    .add_sized(
                        [ui.available_width(), 34.0],
                        egui::TextEdit::singleline(&mut self.settings.laya_endpoint)
                            .font(egui::FontId::proportional(13.0))
                            .hint_text("http://127.0.0.1:8000/v1/systemone")
                            .frame(
                                egui::Frame::default()
                                    .fill(PAPER)
                                    .stroke(egui::Stroke::new(1.0, BORDER))
                                    .corner_radius(egui::CornerRadius::same(6))
                                    .inner_margin(egui::Margin::symmetric(9, 6)),
                            ),
                    )
                    .changed();
                (enabled_response.changed(), endpoint_changed)
            })
            .inner;
        ui.add_space(8.0);
        ui.label(
            RichText::new("Your search and note text are sent to this endpoint when enabled.")
                .size(12.0)
                .color(SUBTLE),
        );
        if enabled_changed || endpoint_changed {
            self.invalidate_semantic();
            match self
                .runtime
                .block_on(self.store.save_settings(&self.settings))
            {
                Ok(()) => self.error = None,
                Err(error) => self.error = Some(format!("Could not save settings: {error}")),
            }
        }
        if self.settings.semantic_enabled
            && let Err(message) = semantic::validate_endpoint(&self.settings.laya_endpoint)
        {
            ui.colored_label(ERROR, message);
        }
        if let Some(error) = &self.error {
            ui.colored_label(ERROR, error);
        }
        ui.add_space(21.0);
        ui.separator();
        ui.add_space(9.0);
        ui.label(RichText::new("Keyboard").size(13.0).strong().color(MUTED));
        ui.label(
            RichText::new("Ctrl+N  New note       Ctrl+M  Search")
                .size(12.0)
                .color(MUTED),
        );
        ui.label(
            RichText::new("Ctrl+W  Close window    Ctrl+,  Settings")
                .size(12.0)
                .color(MUTED),
        );
        ui.label(
            RichText::new("↑↓  Select    Enter  Open    Delete  Remove")
                .size(12.0)
                .color(MUTED),
        );
        ui.add_space(19.0);
        if ui
            .add(egui::Button::new("Back to notes  ·  Esc").frame(false))
            .clicked()
        {
            self.page = Page::Notes;
            self.focus_list_id = self.selected;
        }
        if let Some(error) = &self.hotkeys.error {
            ui.add_space(12.0);
            ui.colored_label(ERROR, error);
        }
    }
}

impl eframe::App for NotizApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.quitting && ctx.input(|input| input.viewport().close_requested()) {
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::CancelClose);
        }
        self.handle_global_hotkeys(ctx);
        self.handle_tray_actions(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_search_updates();
        self.start_due_search(&ctx);

        if self.search_visible {
            let mut builder = egui::ViewportBuilder::default()
                .with_title("Notiz")
                .with_inner_size([420.0, 520.0])
                .with_min_inner_size([320.0, 320.0])
                .with_decorations(false)
                .with_transparent(true);
            if self.focus_search_window {
                builder = builder.with_active(true);
            }
            ctx.show_viewport_immediate(search_viewport_id(), builder, |ui, _class| {
                self.search_window(ui);
            });
            if self.search_visible && self.focus_search_window {
                ctx.send_viewport_cmd_to(
                    search_viewport_id(),
                    egui::ViewportCommand::Visible(true),
                );
                ctx.send_viewport_cmd_to(
                    search_viewport_id(),
                    egui::ViewportCommand::Minimized(false),
                );
                ctx.send_viewport_cmd_to(search_viewport_id(), egui::ViewportCommand::Focus);
                self.focus_search_window = false;
            }
        }

        if self.show_editor {
            let mut builder = egui::ViewportBuilder::default()
                .with_title("Notiz note")
                .with_inner_size([340.0, 300.0])
                .with_min_inner_size([240.0, 200.0])
                .with_decorations(false)
                .with_transparent(true);
            if self.focus_editor_window {
                builder = builder.with_active(true);
            }
            ctx.show_viewport_immediate(note_viewport_id(), builder, |ui, _class| {
                self.note_window(ui);
            });
            if self.show_editor && self.focus_editor_window {
                ctx.send_viewport_cmd_to(note_viewport_id(), egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd_to(
                    note_viewport_id(),
                    egui::ViewportCommand::Minimized(false),
                );
                ctx.send_viewport_cmd_to(note_viewport_id(), egui::ViewportCommand::Focus);
                self.focus_editor_window = false;
            }
        }
    }
}

fn note_title(body: &str) -> &str {
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Untitled")
}

fn note_preview(body: &str) -> &str {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .nth(1)
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::note_title;

    #[test]
    fn title_comes_from_first_nonempty_line() {
        assert_eq!(note_title("\n  Heading  \nBody"), "Heading");
        assert_eq!(note_title("  \n "), "Untitled");
    }
}
