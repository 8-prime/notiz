use std::{
    error::Error,
    sync::mpsc::{self, Receiver},
};

use eframe::egui;
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

#[derive(Clone, Copy)]
pub enum TrayAction {
    NewNote,
    Search,
    Quit,
}

pub struct Tray {
    _icon: TrayIcon,
    receiver: Receiver<TrayAction>,
}

impl Tray {
    pub fn new(ctx: egui::Context) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let new_note = MenuItem::with_id("new-note", "New note  (Alt+N)", true, None);
        let search = MenuItem::with_id("search", "Search notes  (Alt+M)", true, None);
        let quit = MenuItem::with_id("quit", "Quit Notiz", true, None);
        let menu = Menu::new();
        menu.append(&new_note)?;
        menu.append(&search)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&quit)?;

        let icon = TrayIconBuilder::new()
            .with_tooltip("Notiz")
            .with_icon(notiz_icon()?)
            .with_menu(Box::new(menu))
            .build()?;

        let (sender, receiver) = mpsc::channel();
        let new_id = new_note.id().clone();
        let search_id = search.id().clone();
        let quit_id = quit.id().clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let action = if event.id == new_id {
                Some(TrayAction::NewNote)
            } else if event.id == search_id {
                Some(TrayAction::Search)
            } else if event.id == quit_id {
                Some(TrayAction::Quit)
            } else {
                None
            };
            if let Some(action) = action {
                let _ = sender.send(action);
                ctx.request_repaint_of(egui::ViewportId::ROOT);
            }
        }));

        Ok(Self {
            _icon: icon,
            receiver,
        })
    }

    pub fn actions(&self) -> Vec<TrayAction> {
        self.receiver.try_iter().collect()
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
    }
}

fn notiz_icon() -> Result<Icon, tray_icon::BadIcon> {
    const SIZE: usize = 32;
    let mut pixels = Vec::with_capacity(SIZE * SIZE * 4);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let background = (3..29).contains(&x) && (3..29).contains(&y);
            let glyph = (8..24).contains(&y)
                && ((9..12).contains(&x)
                    || (20..23).contains(&x)
                    || (12..20).contains(&x) && (2 * x as isize - y as isize - 16).abs() <= 2);
            let color = if glyph {
                [229, 234, 240, 255]
            } else if background {
                [35, 39, 46, 255]
            } else {
                [0, 0, 0, 0]
            };
            pixels.extend_from_slice(&color);
        }
    }
    Icon::from_rgba(pixels, SIZE as u32, SIZE as u32)
}
