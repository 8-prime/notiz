# Notiz v2

A small native desktop app for plain text notes. The interface uses egui/eframe; notes live in a local SQLite database accessed through SQLx. There is no web frontend, Tauri, rich text, or Markdown rendering.

## Run

Install a current Rust toolchain, then run:

```sh
cargo run
```

The first launch creates `notes.sqlite3` in the operating system's local application data directory for Notiz. Set `NOTIZ_DATA_DIR` to use a different directory, for example when trying the rewrite without touching your usual notes.

### Install on Windows

Quit any running Notiz app from its tray menu, then run this in PowerShell from the project directory:

```powershell
.\scripts\install-windows.ps1
```

The script builds a release executable, copies it to `%LOCALAPPDATA%\Programs\Notiz`, and adds shortcuts to your Start menu and your personal Startup folder. Notiz then starts in the tray when you sign in to Windows. The release build does not open a console window. To stop automatic launch, remove `Notiz v2.lnk` from `shell:startup` or disable it in Windows Startup settings. Notes remain in the local application data directory when you replace or remove the executable.

Notiz starts in the system tray with no window open. The tray menu can open a new note, open search, or quit the app. Search and the note editor are separate native windows with a dark theme. The editor opens as a small, plain text window with no toolbar; typing saves immediately. The first non-empty line becomes its title in the search window. Search always finds plain text matches.

## Keyboard

- **Alt+N** creates a note and opens only the note window, even when Notiz is in the background.
- **Alt+M** opens the notes list with search focused.
- **Ctrl+W** closes the focused window. Notiz remains in the tray; use its menu to quit.
- From search, **Down/Up** moves into the note list at its first/last match. In the list, **Up/Down** selects notes, **Enter** opens the selected note, **Delete** removes it, and **Esc** returns to search.
- **Ctrl+,** opens settings; **Esc** returns to notes.

Global shortcuts depend on OS support and whether another app has already claimed them. If registration fails, the settings page shows the error and the shortcuts still work while Notiz is focused.

## Optional semantic search

Open Settings with **Ctrl+,**, enable semantic search, and enter the full Laya POST endpoint. The default is `http://127.0.0.1:8000/v1/systemone` for the [Laya self-hosted server](https://github.com/NandhaKishorM/laya#self-hosting-http-server-jev-compatible). The app sends each note's text and the search query to that endpoint for a yes/no relevance decision. Literal matches remain visible. If Laya is unavailable, search continues to show literal matches and displays the error. Settings are saved automatically in the local SQLite database.

The v2 database is new; it does not import data from the former Tauri app.

## Check

```sh
cargo test
```
