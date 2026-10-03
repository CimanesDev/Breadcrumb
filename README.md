# Breadcrumb

**Every file leaves a trail.** Breadcrumb is a local Windows file history utility. It watches common user folders, records files in SQLite, reads Windows `Zone.Identifier` download metadata, and shows each file's origin and timeline in a desktop interface. Closing the window leaves the watcher running in the system tray; use the tray menu to reopen or quit.

## Current features

- Event based tracking of new, removed, and renamed files in Downloads, Desktop, Documents, Pictures, and Videos
- Rename and same volume move continuity using Windows file identity, including moves reported as separate remove and create events
- Persistent SQLite history in `%LOCALAPPDATA%\Breadcrumb\history.db`
- Download URL, referrer, and domain from `Zone.Identifier` when Windows provides them
- Best effort Chrome, Edge, and Brave download matching, including secondary profiles, using temporary read only copies of browser history
- Browser download-page recovery when the original file was moved but its name, size, and modification time still match; model IDs are shown for MakerWorld pages
- Search by filename, path, domain, or URL
- Compact Explorer-style file inspector with origin evidence, original and current paths, disk dates, and a history timeline
- Explorer file context menu: right-click a file and choose **Show file trail** (under **Show more options** on Windows 11) to open its details directly
- Release builds start in the tray and register per-user startup so tracking resumes when you sign in
- Local only operation, with no account, telemetry, or network upload

## Development

Requires Windows 10/11, Node.js, Rust with the MSVC toolchain, and the Tauri 2 Windows prerequisites (Microsoft C++ Build Tools and WebView2).

```powershell
npm install
npm run tauri dev
```

Create an installer with `npm run tauri build`. The React code can be checked independently with `npm run build`.

The Explorer command and sign-in startup entry are registered when a release build first runs. Development builds open the window normally and do not alter Explorer or sign-in settings. The tray menu can open the full history view or quit the background tracker.

## Architecture

React and TypeScript render the interface. The Tauri Rust backend owns the filesystem watcher, SQLite database, and Windows metadata inspection. Database migrations live in `src-tauri/migrations/`.

## Privacy and limits

Your file history stays on your computer. Breadcrumb does not upload your files or browsing history. Breadcrumb cannot always determine the origin of files that existed before it was installed. Source information depends on metadata preserved by Windows, browsers, and applications. Moves outside watched folders may appear as deletions. Moves across volumes may create a new identity. Browser matching is best effort and varies with Chromium history schema and timing. Hashing, archive relationships, configurable watched folders, and recovery scanning remain planned.

## Screenshots

Dashboard screenshot: to be added after the first native build.

File detail screenshot: to be added after the first native build.

## Roadmap

1. Add configurable watched locations and identity based moves across disks.
2. Add queued BLAKE3 hashing, duplicate and copy relationships.
3. Add ZIP extraction provenance, onboarding, and optional existing file scans.
