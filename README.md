# Breadcrumb

**Every file leaves a trail.** Breadcrumb is a local Windows file history utility. This first milestone watches Downloads, records new files in SQLite, reads Windows `Zone.Identifier` download metadata, and shows each file's origin and timeline in a desktop interface. Closing the window leaves the watcher running in the system tray; use the tray menu to reopen or quit.

## Current features

- Event based tracking of new, changed, removed, and renamed files in Downloads
- Persistent SQLite history in `%LOCALAPPDATA%\Breadcrumb\history.db`
- Download URL, referrer, and domain from `Zone.Identifier` when Windows provides them
- Search by filename, path, domain, or URL
- File detail view with origin and chronological events
- Local only operation, with no account, telemetry, or network upload

## Development

Requires Windows 10/11, Node.js, Rust with the MSVC toolchain, and the Tauri 2 Windows prerequisites (Microsoft C++ Build Tools and WebView2).

```powershell
npm install
npm run tauri dev
```

Create an installer with `npm run tauri build`. The React code can be checked independently with `npm run build`.

## Architecture

React and TypeScript render the interface. The Tauri Rust backend owns the filesystem watcher, SQLite database, and Windows metadata inspection. The initial schema lives in `src-tauri/migrations/001_initial.sql`.

## Privacy and limits

Your file history stays on your computer. Breadcrumb does not upload your files or browsing history. Breadcrumb cannot always determine the origin of files that existed before it was installed. Source information depends on metadata preserved by Windows, browsers, and applications. This milestone only watches Downloads; moves outside that folder may appear as deletions. Browser history correlation, broader watched folders, hashing, archive relationships, and recovery scanning remain planned.

## Screenshots

Dashboard screenshot: to be added after the first native build.

File detail screenshot: to be added after the first native build.

## Roadmap

1. Correlate Chromium download history, including nondefault profiles.
2. Add configurable watched locations and identity based moves across folders.
3. Add queued BLAKE3 hashing, duplicate and copy relationships.
4. Add ZIP extraction provenance, onboarding, and optional existing file scans.
