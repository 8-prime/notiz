use std::{
    collections::HashSet,
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
};

const INK: Color32 = Color32::from_rgb(29, 39, 53);
const MUTED: Color32 = Color32::from_rgb(105, 115, 130);
const PAPER: Color32 = Color32::from_rgb(249, 250, 252);
const RAIL: Color32 = Color32::from_rgb(235, 239, 244);
const ACCENT: Color32 = Color32::from_rgb(53, 96, 143);
const ERROR: Color32 = Color32::from_rgb(154, 60, 58);

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
        let new_key = HotKey::new(Some(Modifiers::ALT), Code::KeyN);
        let search_key = HotKey::new(Some(Modifiers::ALT), Code::KeyM);
        let (mut new_registered, mut search_registered) = (false, false);
        if let Some(manager) = &manager {
            match manager.register(new_key) {
                Ok(()) => new_registered = true,
                Err(error) => errors.push(format!("Alt+N is unavailable: {error}")),
            }
            match manager.register(search_key) {
                Ok(()) => search_registered = true,
                Err(error) => errors.push(format!("Alt+M is unavailable: {error}")),
            }
        }

        let running = Arc::new(AtomicBool::new(true));
        let listener = if new_registered || search_registered {
            let running = Arc::clone(&running);
            Some(std::thread::spawn(move || {
                while running.load(Ordering::Relaxed) {
                    if let Ok(event) =
                        GlobalHotKeyEvent::receiver().recv_timeout(Duration::from_millis(200))
                    {
                        if event.state != HotKeyState::Pressed {
                            continue;
                        }
                        let action = if new_registered && event.id == new_key.id {
                            Some(HotkeyAction::NewNote)
                        } else if search_registered && event.id == search_key.id {
                            Some(HotkeyAction::Search)
                        } else {
                            None
                        };
                        if let Some(action) = action {
                            if sender.send(action).is_err() {
                                break;
                            }
                            ctx.request_repaint();
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
    focus_editor: bool,
    focus_search: bool,
    focus_settings: bool,
    focus_list_id: Option<i64>,
    editor_has_focus: bool,
    search_has_focus: bool,
    list_has_focus: bool,
    hotkeys: Hotkeys,
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
    Settings,
}

impl NotizApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        runtime: Runtime,
        store: Store,
        notes: Vec<Note>,
        settings: Settings,
    ) -> Self {
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = PAPER;
        visuals.window_fill = PAPER;
        visuals.override_text_color = Some(INK);
        cc.egui_ctx.set_visuals(visuals);

        let selected = notes.first().map(|note| note.id);
        let draft = notes
            .first()
            .map(|note| note.body.clone())
            .unwrap_or_default();
        let (search_sender, search_updates) = mpsc::channel();
        Self {
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
            focus_editor: false,
            focus_search: false,
            focus_settings: false,
            focus_list_id: None,
            editor_has_focus: false,
            search_has_focus: false,
            list_has_focus: false,
            hotkeys: Hotkeys::new(cc.egui_ctx.clone()),
            search_updates,
            search_sender,
            active_search: Arc::new(AtomicU64::new(0)),
            pending_search: None,
            semantic_matches: HashSet::new(),
            semantic_pending: false,
            semantic_error: None,
        }
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
        if self.selected == Some(id) || !self.save_current() {
            return;
        }
        if let Some(note) = self.notes.iter().find(|note| note.id == id) {
            self.selected = Some(id);
            self.draft.clone_from(&note.body);
        }
    }

    fn create(&mut self) {
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
                    self.focus_editor = true;
                }
            }
            Err(error) => self.error = Some(format!("Could not create note: {error}")),
        }
    }

    fn delete_selected(&mut self) {
        let Some(id) = self.selected else { return };
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
                    self.focus_list_id = self.selected;
                    self.focus_search = self.selected.is_none();
                    self.invalidate_semantic();
                }
            }
            Err(error) => self.error = Some(format!("Could not delete note: {error}")),
        }
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

    fn open_search(&mut self) {
        if !self.save_current() {
            return;
        }
        self.page = Page::Notes;
        self.search.clear();
        self.invalidate_semantic();
        self.focus_search = true;
    }

    fn bring_to_front(ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }

    fn handle_global_hotkeys(&mut self, ctx: &egui::Context) {
        let global_actions: Vec<_> = self.hotkeys.receiver.try_iter().collect();
        for action in global_actions {
            Self::bring_to_front(ctx);
            match action {
                HotkeyAction::NewNote => self.create(),
                HotkeyAction::Search => self.open_search(),
            }
        }
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let keys = ctx.input(|input| {
            (
                input.modifiers.alt && input.key_pressed(egui::Key::N),
                input.modifiers.alt && input.key_pressed(egui::Key::M),
                input.modifiers.command && input.key_pressed(egui::Key::Comma),
                input.key_pressed(egui::Key::Escape),
                input.key_pressed(egui::Key::ArrowDown),
                input.key_pressed(egui::Key::ArrowUp),
                input.key_pressed(egui::Key::Enter),
                input.key_pressed(egui::Key::Delete),
            )
        });
        if keys.0 && !self.hotkeys.new_registered {
            self.create();
            return;
        }
        if keys.1 && !self.hotkeys.search_registered {
            self.open_search();
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
                self.focus_editor = true;
                self.list_has_focus = false;
            } else if keys.3 {
                self.focus_search = true;
            }
        } else if self.search_has_focus && keys.4 {
            if let Some(id) = self.visible_ids().first().copied() {
                self.select(id);
                self.focus_list_id = Some(id);
            }
        } else if self.editor_has_focus && keys.3 {
            self.focus_list_id = self.selected;
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) -> Option<SidebarAction> {
        let mut action = None;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Notiz").size(25.0).strong().color(INK));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("New note").color(Color32::WHITE))
                            .fill(ACCENT),
                    )
                    .clicked()
                {
                    action = Some(SidebarAction::Create);
                }
            });
        });
        ui.add_space(13.0);
        let search_response = ui.add_sized(
            [ui.available_width(), 30.0],
            egui::TextEdit::singleline(&mut self.search)
                .id_salt("note_search")
                .hint_text("Search notes  ·  Alt+M"),
        );
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
        ui.add_space(8.0);

        let visible: Vec<_> = self
            .visible_ids()
            .into_iter()
            .filter_map(|id| {
                self.notes
                    .iter()
                    .find(|note| note.id == id)
                    .map(|note| (id, note_title(&note.body).to_owned()))
            })
            .collect();
        self.list_has_focus = false;
        if visible.is_empty() {
            ui.add_space(12.0);
            let message = if self.notes.is_empty() {
                "No notes yet. Press Alt+N to start."
            } else {
                "No matching notes."
            };
            ui.label(RichText::new(message).color(MUTED));
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (id, title) in visible {
                    let response = ui.selectable_label(self.selected == Some(id), title);
                    if self.focus_list_id == Some(id) {
                        response.request_focus();
                        self.focus_list_id = None;
                        self.list_has_focus = true;
                        self.search_has_focus = false;
                        self.editor_has_focus = false;
                    }
                    if !focus_search_now {
                        self.list_has_focus |= response.has_focus();
                    }
                    if response.clicked() || response.gained_focus() {
                        action = Some(SidebarAction::Select(id));
                    }
                    ui.add_space(4.0);
                }
            });
        }
        ui.add_space(12.0);
        if ui.button("Settings  ·  Ctrl+,").clicked() {
            action = Some(SidebarAction::Settings);
        }
        action
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let title = if self.selected.is_some() {
                note_title(&self.draft)
            } else {
                "Your notes"
            };
            ui.label(RichText::new(title).size(21.0).strong().color(INK));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.selected.is_some() && ui.button("Delete note").clicked() {
                    self.delete_selected();
                }
            });
        });
        ui.separator();

        if let Some(error) = &self.error {
            ui.colored_label(ERROR, error);
            if self.dirty && ui.button("Retry save").clicked() {
                self.save_current();
            }
            ui.add_space(8.0);
        }

        if self.selected.is_some() {
            let response = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.draft)
                    .id_salt(self.selected)
                    .hint_text("Start writing…")
                    .desired_rows(20),
            );
            if self.focus_editor {
                response.request_focus();
                self.focus_editor = false;
                self.editor_has_focus = true;
                self.list_has_focus = false;
                self.search_has_focus = false;
            } else {
                self.editor_has_focus = response.has_focus();
            }
            if response.changed() {
                self.dirty = true;
                self.save_current();
            }
        } else {
            self.editor_has_focus = false;
            ui.add_space(28.0);
            ui.label(
                RichText::new("Press Alt+N to create a note.")
                    .size(17.0)
                    .color(MUTED),
            );
        }
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.label(RichText::new("Semantic search").size(18.0).strong());
        ui.label("When enabled, searches also ask Laya whether each note matches your query.");
        ui.label(
            RichText::new("Note text and the query are sent to the endpoint below.").color(MUTED),
        );
        ui.add_space(10.0);

        let enabled_response = ui.checkbox(
            &mut self.settings.semantic_enabled,
            "Enable semantic search",
        );
        if self.focus_settings {
            enabled_response.request_focus();
            self.focus_settings = false;
        }
        let enabled_changed = enabled_response.changed();
        ui.label("Laya POST endpoint");
        let endpoint_changed = ui
            .add_sized(
                [ui.available_width().min(540.0), 30.0],
                egui::TextEdit::singleline(&mut self.settings.laya_endpoint)
                    .hint_text("http://127.0.0.1:8000/v1/systemone"),
            )
            .changed();
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
        ui.add_space(18.0);
        ui.label(
            RichText::new("Alt+N  New note     Alt+M  Search     Ctrl+,  Settings").color(MUTED),
        );
        ui.label(
            RichText::new("In the note list: ↑/↓ select, Enter edit, Delete remove, Esc search.")
                .color(MUTED),
        );
        ui.add_space(18.0);
        if ui.button("Back to notes  ·  Esc").clicked() {
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
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_global_hotkeys(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_search_updates();
        self.start_due_search(&ctx);
        self.handle_keyboard(&ctx);

        if self.page == Page::Settings {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::default()
                        .fill(PAPER)
                        .inner_margin(egui::Margin::same(26)),
                )
                .show(ui, |ui| self.settings_page(ui));
            return;
        }

        let action = egui::Panel::left("notes")
            .exact_size(270.0)
            .frame(
                egui::Frame::default()
                    .fill(RAIL)
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ui, |ui| self.sidebar(ui))
            .inner;

        if let Some(action) = action {
            match action {
                SidebarAction::Create => self.create(),
                SidebarAction::Select(id) => self.select(id),
                SidebarAction::Settings => {
                    self.page = Page::Settings;
                    self.focus_settings = true;
                }
            }
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(PAPER)
                    .inner_margin(egui::Margin::same(26)),
            )
            .show(ui, |ui| self.editor(ui));
    }
}

fn note_title(body: &str) -> &str {
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Untitled")
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
