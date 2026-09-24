# Notiz v2

A small native desktop app for plain text notes. The interface uses egui/eframe; notes live in a local SQLite database accessed through SQLx. There is no web frontend, Tauri, rich text, or Markdown rendering.

## Run

Install a current Rust toolchain, then run:

```sh
cargo run
```

The first launch creates `notes.sqlite3` in the operating system's local application data directory for Notiz. Set `NOTIZ_DATA_DIR` to use a different directory, for example when trying the rewrite without touching your usual notes.

Search and the note editor are separate native windows. The editor opens as a small, plain text note with no toolbar; typing saves immediately. The first non-empty line becomes its title in the search window. Search always finds plain text matches.

## Keyboard

- **Alt+N** creates a note and opens the note window, even when Notiz is in the background.
- **Alt+M** brings the search window forward and focuses search.
- **Ctrl+W** closes the focused window. Closing the search window exits Notiz; closing the note window leaves search open.
- From search, **Down** moves into the note list. In the list, **Up/Down** selects notes, **Enter** opens the selected note, **Delete** removes it, and **Esc** returns to search.
- **Ctrl+,** opens settings; **Esc** returns to notes.

Global shortcuts depend on OS support and whether another app has already claimed them. If registration fails, the settings page shows the error and the shortcuts still work while Notiz is focused.

## Optional semantic search

Open Settings with **Ctrl+,**, enable semantic search, and enter the full Laya POST endpoint. The default is `http://127.0.0.1:8000/v1/systemone` for the [Laya self-hosted server](https://github.com/NandhaKishorM/laya#self-hosting-http-server-jev-compatible). The app sends each note's text and the search query to that endpoint for a yes/no relevance decision. Literal matches remain visible. If Laya is unavailable, search continues to show literal matches and displays the error. Settings are saved automatically in the local SQLite database.

The v2 database is new; it does not import data from the former Tauri app.

## Check

```sh
cargo test
```
