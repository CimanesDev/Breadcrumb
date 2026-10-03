# Breadcrumb — Windows File Provenance & History Desktop App

You are building **Breadcrumb**, a Windows desktop utility that answers one simple question:

> **“Where did this file come from?”**

Breadcrumb should quietly track the origin and lifecycle of files on a user's computer so that, weeks or months later, the user can inspect any file and understand:

- where it originally came from
- when it appeared on the computer
- which website or URL it was downloaded from, when available
- which application/browser created or downloaded it
- its original filename
- its original location
- where it was moved
- whether it was renamed
- whether it came from an extracted archive
- whether duplicates or identical copies exist
- what happened to it over time

The product should feel like:

> **Browser history, but for files.**

or:

> **Git history for normal files — without version control.**

The name is:

# Breadcrumb

Primary tagline:

> **Every file leaves a trail.**

Alternative supporting line:

> **Know where every file came from.**

---

# 1. Product Philosophy

Breadcrumb is **NOT**:

- a traditional file manager
- a disk cleanup tool
- a replacement for Windows Explorer
- a cloud drive
- a backup program
- an antivirus
- a generic search utility
- a download manager

Breadcrumb is specifically a **file provenance and lifecycle tracker**.

Its most important feature is being able to select or search for a file and reconstruct its history.

Example:

```text
dentex.zip

Downloaded
September 11, 2026 — 9:31 PM

Source
https://zenodo.org/records/xxxxx

Referrer
https://zenodo.org/record/xxxxx

Downloaded using
Google Chrome

Original location
C:\Users\Josh\Downloads\dentex.zip

Moved
E:\SP\dentex-sp\data\dentex.zip

Extracted
E:\SP\dentex-sp\data\DENTEX\

Contains
7,218 files
```

Another example:

```text
image001.png

Created from
DENTEX.zip

DENTEX.zip was downloaded from
zenodo.org

Current location
E:\SP\dentex-sp\data\train\image001.png
```

This provenance chain is what makes Breadcrumb valuable.

---

# 2. Target Platform

Build **Windows first**.

Do not attempt macOS or Linux support in the first implementation.

Target:

- Windows 10
- Windows 11
- x64

The app should be lightweight enough to remain running in the background continuously.

---

# 3. Preferred Tech Stack

Use a modern stack suitable for a polished Windows utility.

Preferred architecture:

## Desktop shell

Use:

**Tauri 2**

with:

- Rust backend
- React
- TypeScript
- Vite

Reasons:

- lower memory usage than Electron
- strong filesystem APIs through Rust
- native Windows integration
- easy background/tray application
- SQLite integration
- suitable for long-running filesystem monitoring

Frontend:

- React
- TypeScript
- Tailwind CSS
- shadcn/ui where useful
- Lucide icons

Backend:

- Rust

Database:

- SQLite

Use migrations from the beginning.

Do not introduce a cloud backend.

Everything should initially work **100% locally**.

---

# 4. Privacy Requirements

Privacy is a core product feature.

Breadcrumb should:

- store its database locally
- never upload filenames
- never upload file contents
- never upload browsing history
- never upload file paths
- never require an account
- never require an internet connection for normal tracking

Display somewhere in onboarding/settings:

> **Your file history stays on your computer. Breadcrumb does not upload your files or browsing history.**

Do not collect telemetry in the MVP.

Structure the project so optional privacy-respecting telemetry could theoretically be added later, but do not implement it now.

---

# 5. Core MVP

Build the following as the first usable version.

## Feature 1 — Background File Tracking

Breadcrumb should run in the Windows system tray.

Track filesystem activity in configurable locations.

Default monitored folders:

```text
%USERPROFILE%\Downloads
%USERPROFILE%\Desktop
%USERPROFILE%\Documents
%USERPROFILE%\Pictures
%USERPROFILE%\Videos
```

Allow the user to add custom folders/drives later.

Detect:

- file creation
- file rename
- file move
- file deletion
- file modification

Do not try to record every tiny modification as a timeline event.

Focus timeline events primarily on:

- created
- downloaded
- moved
- renamed
- copied
- extracted
- deleted
- restored/reappeared

Debounce noisy filesystem events.

A single file operation should not produce several duplicate records.

---

# 6. File Identity

Paths cannot be the only identity.

If:

```text
Downloads\report.pdf
```

becomes:

```text
Documents\School\CMSC199\final-report.pdf
```

Breadcrumb should ideally know this is still the same file.

Track:

- internal Breadcrumb file ID
- current path
- previous paths
- original path
- filename
- original filename
- extension
- MIME/file type when detectable
- size
- created timestamp
- modified timestamp
- first seen timestamp
- last seen timestamp

Calculate a content hash when reasonable.

Use:

**BLAKE3**

by default because it is fast.

For huge files, hashing must not freeze or significantly slow the user's machine.

Implement background hashing with sensible limits and queueing.

Possible strategy:

- immediately hash files under 100 MB
- background hash larger files
- avoid repeated hashes if metadata is unchanged
- configurable maximum automatic hash size

Store:

```text
content_hash
hash_algorithm
hash_status
```

Use hashes to identify:

- renamed files
- moved files
- duplicate copies
- reappearing files

---

# 7. Download Origin Detection

This is extremely important.

Breadcrumb should attempt to determine where downloaded files came from.

Use multiple data sources.

## A. Windows Mark of the Web

Inspect the NTFS Alternate Data Stream:

```text
Zone.Identifier
```

Parse useful fields where present, especially:

```text
ZoneId
HostUrl
ReferrerUrl
```

Store these values.

Do not assume they are always available.

Never fail if they are missing.

Example:

```text
[ZoneTransfer]
ZoneId=3
ReferrerUrl=https://example.com/download-page
HostUrl=https://cdn.example.com/file.zip
```

Store:

```text
source_url
referrer_url
source_domain
```

---

# 8. Browser Download History

Support Chromium browsers first.

Attempt to correlate files with browser download history.

Initial supported browsers:

- Google Chrome
- Microsoft Edge
- Brave

Later architecture should allow:

- Firefox
- Opera
- Vivaldi

Chromium download databases are SQLite files.

Relevant Chromium profile locations should be discovered dynamically.

Do not assume the user only uses the Default profile.

Look for:

```text
Default
Profile 1
Profile 2
...
```

Read browser history safely.

Browsers may lock their database.

Do not modify browser databases.

If required:

1. create a temporary copy
2. read the copy
3. delete the temporary copy

Retrieve relevant download information such as:

- target path
- original URL
- final URL
- referrer
- start time
- end time
- MIME type
- browser/profile

Correlate the browser download record with Breadcrumb's file record using:

- path
- filename
- timestamp
- filesize
- hash where applicable

Do not depend on only one field.

Assign a provenance confidence level:

```text
high
medium
low
```

For example:

```text
High:
exact path + close timestamp

Medium:
same filename + size + close timestamp

Low:
same filename + similar timestamp
```

Show uncertainty honestly.

Never fabricate a source.

If it cannot be determined, display:

> **Source unknown**

---

# 9. Application / Process Detection

When possible, determine which process created a newly tracked file.

Examples:

```text
chrome.exe
msedge.exe
discord.exe
slack.exe
explorer.exe
7z.exe
winrar.exe
```

Store:

```text
creator_process
creator_application
```

Do not make this requirement block file tracking if Windows does not expose it reliably through the chosen implementation.

Use a best-effort architecture.

If reliable direct process attribution is difficult for V1, structure the backend so it can be added later.

---

# 10. File Timeline

Every tracked file should have a timeline.

Example:

```text
September 11 · 9:31 PM
Downloaded

zenodo.org
C:\Users\Josh\Downloads\DENTEX.zip


September 11 · 9:34 PM
Renamed

DENTEX-main.zip


September 11 · 9:36 PM
Moved

E:\SP\dentex-sp\data\DENTEX-main.zip


September 11 · 9:41 PM
Extracted

E:\SP\dentex-sp\data\DENTEX\
```

Supported event types:

```text
FIRST_SEEN
DOWNLOADED
CREATED
RENAMED
MOVED
COPIED
EXTRACTED
MODIFIED
DELETED
RESTORED
SOURCE_IDENTIFIED
```

The timeline should be chronological.

Do not display low-level system noise.

---

# 11. Detecting Rename vs Move vs Copy

Use multiple signals.

## Rename

Same identity/hash, different filename, same parent directory or filesystem event strongly indicates rename.

Example:

```text
resume.pdf
→
Josh-Cimanes-Resume.pdf
```

Record:

```text
RENAMED
```

## Move

Same identity/hash, different directory.

Example:

```text
Downloads\paper.pdf
→
Documents\Thesis\paper.pdf
```

Record:

```text
MOVED
```

## Copy

Original still exists and another file with the same content hash appears elsewhere.

Record relationship:

```text
COPY_OF
```

Do not necessarily show copies as the same file identity.

Treat them as separate file entities connected by a relationship.

---

# 12. Archive Extraction Detection

This is one of Breadcrumb's strongest potential features.

Support initially:

```text
.zip
```

Later architecture should support:

```text
.rar
.7z
.tar
.tar.gz
```

When many files appear in a directory shortly after an archive operation, attempt to associate those files with the archive.

Use heuristics including:

- timestamps
- archive contents
- filenames
- extraction destination
- file sizes
- extraction application/process when available

If an archive contains:

```text
train/image001.png
train/image002.png
annotations.json
```

and those exact files appear shortly afterward, establish:

```text
image001.png
    ↓ extracted from
dataset.zip
    ↓ downloaded from
zenodo.org
```

Relationships:

```text
EXTRACTED_FROM
CONTAINS
```

This should eventually allow provenance recursion.

When viewing a child file, Breadcrumb should be able to say:

> **This file was extracted from DENTEX.zip, which was downloaded from zenodo.org.**

Avoid claiming a relationship when confidence is weak.

---

# 13. Database Design

Use a sensible normalized schema.

Possible tables:

## files

```text
id
current_name
original_name
current_path
original_path
extension
mime_type
size_bytes
content_hash
hash_algorithm
hash_status
first_seen_at
last_seen_at
created_at
modified_at
deleted_at
is_present
```

## file_events

```text
id
file_id
event_type
timestamp
old_path
new_path
old_name
new_name
metadata_json
confidence
```

## provenance

```text
id
file_id
source_type
source_url
referrer_url
source_domain
application
process_name
browser_name
browser_profile
download_started_at
download_finished_at
confidence
metadata_json
```

## file_relationships

```text
id
parent_file_id
child_file_id
relationship_type
confidence
created_at
metadata_json
```

Relationship types:

```text
COPY_OF
EXTRACTED_FROM
CONTAINS
DERIVED_FROM
```

## watched_locations

```text
id
path
enabled
recursive
created_at
```

## app_settings

Key-value or structured settings.

Use proper indexes for:

- path
- filename
- domain
- timestamp
- file hash
- extension

---

# 14. Main UI

The UI should be clean, minimal, modern, and fast.

Do not make it look like enterprise forensic software.

Think:

- Raycast
- Arc
- Linear
- modern Windows utilities

Use lots of whitespace.

Avoid excessive borders.

Support both:

- light mode
- dark mode

Follow system theme by default.

---

# 15. Main Dashboard

Layout:

```text
┌──────────────────────────────────────────────────────┐
│ Breadcrumb                              Settings ⚙   │
├──────────────────────────────────────────────────────┤
│                                                      │
│  Search your files...                                │
│                                                      │
│  [ All ] [ Today ] [ Downloads ] [ Sources ]         │
│                                                      │
│  RECENT                                              │
│                                                      │
│  📄 CMSC199.pdf                                      │
│     canvas.up.edu.ph                                 │
│     12 minutes ago                                  │
│                                                      │
│  📦 ultralytics.zip                                  │
│     github.com                                       │
│     48 minutes ago                                  │
│                                                      │
│  🖼 screenshot.png                                   │
│     Created locally                                  │
│     1 hour ago                                       │
│                                                      │
└──────────────────────────────────────────────────────┘
```

Show source domain if known.

Otherwise:

```text
Created locally
Source unknown
Extracted from <archive>
Copied from <file>
```

---

# 16. Search

Search should be one of the best parts of Breadcrumb.

Support searching by:

- filename
- partial filename
- extension
- folder
- domain
- original URL
- browser
- date
- file type
- current path
- old path

Examples:

```text
github
```

returns files originating from GitHub.

```text
.pdf
```

returns PDFs.

```text
up.edu.ph
```

returns files sourced from that domain.

```text
dentex
```

searches filename, path, source, and relationships.

Eventually support natural filters like:

```text
PDFs from github
```

but do not use AI for V1.

Implement structured search first.

---

# 17. File Detail Page

When a file is selected, show:

```text
← Back

DENTEX.zip

E:\SP\dentex-sp\data\DENTEX.zip

[ Open File ] [ Show in Explorer ]

------------------------------------------------

ORIGIN

zenodo.org
https://zenodo.org/records/xxxxx

Referrer:
https://zenodo.org/...

Downloaded using:
Google Chrome

September 11, 2026
9:31 PM

Confidence:
High

------------------------------------------------

FILE

ZIP Archive
10.4 GB

Original filename:
DENTEX.zip

Original location:
C:\Users\Josh\Downloads\

Current location:
E:\SP\dentex-sp\data\

BLAKE3:
f49a...

------------------------------------------------

HISTORY

● Downloaded
  Sep 11 · 9:31 PM

│

● Moved
  Downloads
  ↓
  E:\SP\dentex-sp\data
  Sep 11 · 9:36 PM

│

● Extracted
  7,218 files
  Sep 11 · 9:41 PM
```

Keep information hierarchy strong.

The source should be visible near the top.

---

# 18. Sources View

Create a dedicated page:

# Sources

Example:

```text
github.com                 293 files
drive.google.com           181
up.edu.ph                   96
discord.com                 61
zenodo.org                  14
stackoverflow.com            8
```

Click a domain to see files obtained from it.

Show:

- number of tracked files
- latest download
- total size if easy to calculate

Do not overcomplicate it.

---

# 19. Timeline / Activity View

Optional but desirable for MVP if practical.

Show recent file activity:

```text
TODAY

2:41 PM
Downloaded lecture05.pdf
from canvas.up.edu.ph

1:58 PM
Moved assignment.pdf
Downloads → Documents\CMSC199

12:11 PM
Extracted project.zip
31 files created
```

Useful filters:

```text
All
Downloads
Moves
Renames
Extracted
Deleted
```

---

# 20. Context Menu Integration

Eventually Breadcrumb should support Windows Explorer integration.

Desired action:

Right-click a file:

```text
Show in Breadcrumb
```

Selecting it should open Breadcrumb directly to that file.

If native Explorer shell integration becomes too complicated for initial MVP, implement it after the core app works.

Do not let this delay core functionality.

---

# 21. Drag-and-Drop Investigation

Allow dragging a file onto the Breadcrumb window.

Breadcrumb should attempt to locate it in its index.

If found:

> Open file history.

If not indexed:

inspect:

- file metadata
- hash
- Zone.Identifier
- browser history correlation

Then display whatever can be recovered.

Example:

```text
We haven't tracked this file before.

Recovered information:

Source:
github.com

Downloaded:
August 14, 2026

Browser:
Google Chrome

Tracking started now.
```

If nothing can be recovered:

```text
Source unknown.

Breadcrumb wasn't installed when this file arrived and no source metadata remains.
```

Be transparent.

---

# 22. Existing File Import

During onboarding, offer:

> **Scan existing files for origin information**

Optional.

Do not force the user.

Scan common directories for:

- Zone.Identifier
- basic file metadata
- matches against browser download history

Avoid hashing the entire disk during onboarding.

That would be too expensive.

Instead:

1. recover lightweight metadata first
2. calculate hashes later only when needed

Show progress.

Allow cancel.

---

# 23. Onboarding

Keep onboarding short.

## Screen 1

```text
Breadcrumb

Every file leaves a trail.

Breadcrumb remembers where your files came from,
where they move, and how they got there.

[ Get Started ]
```

## Screen 2

```text
Private by default.

Breadcrumb runs locally on your computer.

Your files, paths, and browsing history
are never uploaded.

[ Continue ]
```

## Screen 3

```text
Choose what Breadcrumb watches.

✓ Downloads
✓ Desktop
✓ Documents
✓ Pictures

+ Add Folder

[ Start Breadcrumb ]
```

Optional checkbox:

```text
□ Scan existing files for origin information
```

---

# 24. System Tray

Breadcrumb should be able to run without the main window open.

Tray menu:

```text
Open Breadcrumb
Pause tracking
Recent files
Settings
Quit
```

When tracking is paused, clearly indicate it.

---

# 25. Settings

Include:

## General

```text
Launch Breadcrumb when Windows starts
Run in system tray
Theme: System / Light / Dark
```

## Tracking

```text
Watched folders
Ignored folders
Ignored file extensions
Pause tracking
```

Default ignore examples:

```text
node_modules
.git
AppData\Local\Temp
browser cache folders
build artifacts where appropriate
```

Do not blindly ignore development folders outside obvious generated/cache locations.

## Privacy

```text
Local database location
Clear history
Exclude domains
Exclude folders
```

Provide:

> Delete all Breadcrumb history

with confirmation.

## Performance

```text
Automatic hashing
Maximum immediate hash size
Background indexing
```

---

# 26. Exclusions

Breadcrumb needs strong exclusion rules or the database will become noisy.

Ignore by default:

- browser cache
- temporary files
- OS temp folders
- thumbnails
- lock files
- obvious application caches
- swap files
- continuously rewritten log files where appropriate

Examples:

```text
*.tmp
*.crdownload
*.part
~$*
```

Important:

Temporary browser downloads such as:

```text
filename.zip.crdownload
```

should NOT be permanently recorded as a separate final file.

Instead detect the transition to the completed file where possible.

---

# 27. Performance Requirements

Breadcrumb must feel invisible when idle.

Target goals:

- low idle CPU usage
- low memory usage
- no blocking UI operations
- hashing done asynchronously
- filesystem events processed through queues
- database writes batched where sensible

Never scan the entire computer continuously.

Use event-based monitoring.

Perform periodic reconciliation only if needed.

---

# 28. Reliability

Filesystem tracking can miss events.

Design for eventual reconciliation.

For watched folders:

- maintain state
- periodically verify tracked files still exist
- update current paths/status where possible

Do not run expensive full scans frequently.

Persist tracking state cleanly across crashes/restarts.

Database corruption should not crash the entire app.

Use proper SQLite transactions.

---

# 29. Confidence System

Because provenance is not always certain, implement confidence.

Example:

```text
Source: github.com
Confidence: High
```

Confidence logic:

### High

Direct Zone.Identifier HostUrl or exact browser download path match.

### Medium

Filename + size + timestamp strongly correlate with browser download.

### Low

Only partial evidence exists.

Do not clutter the normal UI with technical confidence scoring everywhere.

Display it in detail views.

---

# 30. Security

Breadcrumb deals with sensitive local metadata.

Follow these rules:

- never execute tracked files
- never automatically open downloaded URLs
- sanitize displayed URLs
- protect against malicious filenames
- do not interpret file contents as HTML
- treat archive names and metadata as untrusted
- never require Administrator privileges unless absolutely necessary
- prefer user-level permissions

Do not silently elevate privileges.

---

# 31. No AI Requirement

Do not add:

- ChatGPT
- LLMs
- embeddings
- cloud AI
- automatic summarization
- AI classification

They are unnecessary for the MVP.

Breadcrumb should derive its value from good systems engineering and UX.

AI could theoretically be explored later for semantic search, but it is **not part of this build**.

---

# 32. Architecture

Structure the Rust backend into clear modules.

Suggested layout:

```text
src-tauri/
  src/
    main.rs

    database/
      mod.rs
      models.rs
      migrations.rs
      queries.rs

    tracking/
      mod.rs
      watcher.rs
      event_processor.rs
      reconciliation.rs

    provenance/
      mod.rs
      zone_identifier.rs
      chromium.rs
      matcher.rs

    files/
      mod.rs
      metadata.rs
      hashing.rs
      identity.rs

    relationships/
      mod.rs
      archives.rs
      duplicates.rs

    system/
      mod.rs
      tray.rs
      startup.rs
      explorer.rs

    commands/
      mod.rs
```

Frontend:

```text
src/
  components/
  pages/
    Dashboard.tsx
    FileDetails.tsx
    Sources.tsx
    Activity.tsx
    Settings.tsx
    Onboarding.tsx

  hooks/
  lib/
  stores/
  types/
```

Keep OS-specific behavior out of frontend code.

---

# 33. State Management

Use a lightweight React state system.

Prefer:

- Zustand

or React Query + local component state.

Do not introduce Redux unless clearly necessary.

Use TanStack Query if helpful for asynchronous Tauri commands.

---

# 34. Logging

Implement local logging for debugging.

Logs should:

- rotate
- avoid recording unnecessary sensitive data
- not upload anywhere

Have separate development and production log verbosity.

---

# 35. Development Mode

Provide a development screen or logging mechanism that allows us to inspect:

- incoming filesystem events
- matching decisions
- detected Zone.Identifier
- browser download matches
- archive relationships
- hash status

This is important because provenance matching will require debugging.

Do not expose this prominently in the production UI.

---

# 36. Testing

Write tests for critical backend logic.

At minimum:

### Unit tests

- path normalization
- file identity matching
- URL/domain extraction
- Zone.Identifier parsing
- Chromium timestamp conversion
- provenance confidence
- event deduplication
- rename detection
- move detection

### Integration tests

Simulate:

```text
create
rename
move
copy
delete
```

and confirm expected timeline events.

Test archive extraction heuristics with a fixture ZIP.

---

# 37. Seed / Demo Data

During frontend development, create optional demo data so the UI can be built before all native tracking is complete.

Example records:

```text
CMSC199-Activity5.pdf
Source: canvas.up.edu.ph

dentex.zip
Source: zenodo.org

ultralytics-main.zip
Source: github.com

invoice.pdf
Source: unknown

photo.png
Created locally
```

Demo mode should not ship as active behavior in production.

---

# 38. MVP Priority Order

Do NOT try to implement everything at once.

Build in this order.

## Phase 1 — Foundation

1. Tauri app
2. React UI
3. SQLite database
4. system tray
5. watched-folder configuration

## Phase 2 — Tracking

6. filesystem watcher
7. create events
8. rename detection
9. move detection
10. deletion detection
11. timeline persistence

## Phase 3 — Provenance

12. Zone.Identifier parsing
13. Chromium download history parsing
14. matching engine
15. source/domain display

## Phase 4 — Search/UI

16. dashboard
17. search
18. file details
19. timeline
20. sources page

## Phase 5 — Relationships

21. hashing
22. duplicate detection
23. copy relationships
24. ZIP extraction tracking

## Phase 6 — Polish

25. onboarding
26. settings
27. auto-start
28. Explorer integration
29. drag-and-drop lookup
30. existing-file recovery scan

---

# 39. Definition of MVP Success

Breadcrumb V1 is successful if I can perform this test:

1. Start Breadcrumb.
2. Download a PDF using Chrome.
3. Breadcrumb detects the new file.
4. Breadcrumb identifies Chrome as the source application if possible.
5. Breadcrumb records the original URL/domain through Zone.Identifier and/or browser history.
6. Rename the PDF.
7. Breadcrumb keeps the same file history.
8. Move the PDF into another folder.
9. Breadcrumb records the move.
10. Search for the original website domain.
11. The PDF appears.
12. Open its details.
13. See:

```text
Downloaded from:
example.com

Original name:
paper.pdf

Current name:
thesis-reference.pdf

Original location:
Downloads

Current location:
Documents\Thesis

Timeline:
Downloaded → Renamed → Moved
```

That workflow matters more than secondary features.

---

# 40. Design Direction

Breadcrumb should feel:

- calm
- polished
- lightweight
- trustworthy
- modern
- friendly

Avoid a cyber-security / forensic aesthetic.

No:

- green terminal styling
- giant graphs by default
- excessive technical terminology
- scary surveillance language

Instead of:

> File forensic metadata

say:

> **Where this file came from**

Instead of:

> Provenance record

say:

> **Origin**

Instead of:

> Filesystem event log

say:

> **History**

Technical terms can appear in expandable advanced sections.

---

# 41. Branding

Product:

# Breadcrumb

Logo concept:

A minimal breadcrumb/trail motif.

Potential icon:

```text
● · · ›
```

or a simple sequence of dots ending in a file/document shape.

Do not use literal bread illustrations.

The brand should feel like a modern desktop utility, not a bakery.

Primary tagline:

> **Every file leaves a trail.**

Supporting copy:

> Breadcrumb remembers where files came from, where they move, and how they got there.

---

# 42. Important Product Principle

The application must distinguish between:

## Known

```text
Downloaded from github.com
```

and:

## Inferred

```text
Likely downloaded from github.com
```

and:

## Unknown

```text
Source unknown
```

Never invent provenance just to make the interface look complete.

Trust is more important than coverage.

---

# 43. Future Features — DO NOT BUILD YET

Design the architecture so these can be added eventually, but do not implement them unless all MVP features are stable.

## Cross-device metadata

Install Breadcrumb on:

```text
Desktop
Laptop
```

Search:

```text
CMSC199.pdf
```

and see:

```text
Desktop
E:\School\CMSC199\

Laptop
C:\Users\Josh\Downloads\
```

Only encrypted metadata would sync.

Actual files would remain local.

---

## Device inventory

```text
MY DEVICES

Desktop
184,293 indexed files

Laptop
91,420 indexed files

8,127 appear on both
```

---

## Provenance graph

```text
zenodo.org
    ↓
DENTEX.zip
    ↓
DENTEX/
    ├── train/
    │   └── image001.png
    └── annotations.json
```

---

## Installer tracking

Example:

```text
davinci-resolve.exe
    ↓ launched
DaVinci Resolve
    ↓ installed to
C:\Program Files\Blackmagic Design\
```

This may require deeper Windows instrumentation, so leave it for later.

---

## Optional browser extension companion

A small extension could eventually capture download context with higher reliability:

```text
page URL
download URL
tab title
timestamp
```

and securely hand it to the Breadcrumb desktop app.

Do not require this for V1.

The Windows app must work independently.

---

# 44. README

Create a high-quality README.

Include:

- what Breadcrumb is
- screenshots placeholders
- current features
- architecture
- development requirements
- build instructions
- privacy statement
- limitations
- roadmap

Explicitly state:

> Breadcrumb cannot always determine the origin of files that existed before it was installed. Source information depends on metadata preserved by Windows, browsers, and applications.

---

# 45. Git Hygiene

Use sensible commits if operating in git.

Do not commit:

```text
node_modules
target
build artifacts
SQLite user databases
logs
temporary browser databases
.env files
```

Include proper `.gitignore`.

---

# 46. Code Quality

Requirements:

- TypeScript strict mode
- Rust errors handled properly
- no giant single-file components
- no giant `main.rs`
- reusable services/modules
- clear types
- helpful comments only where necessary
- avoid overengineering
- no placeholder logic pretending features work

If something cannot be implemented reliably, expose that limitation rather than faking it.

---

# 47. Build Strategy

Start by inspecting the existing repository.

If it is empty:

1. initialize Tauri 2 + React + TypeScript + Vite
2. configure Tailwind
3. configure SQLite
4. establish migrations
5. create the app shell
6. implement the backend tracking architecture

If a project already exists:

- understand it first
- preserve useful existing architecture
- do not unnecessarily rewrite working code

Work incrementally.

After each major feature:

- compile
- run tests
- fix warnings/errors
- ensure existing functionality still works

Do not leave the application in a broken intermediate state.

---

# 48. First Deliverable

For the first working milestone, I want an actual usable application, not just mockups.

It should:

- launch
- sit in the tray
- monitor Downloads
- detect new files
- save them in SQLite
- parse `Zone.Identifier`
- display the files in the React UI
- show source URL/domain where available
- show a basic timeline
- retain records across restarts

Then continue implementing rename/move/browser-history correlation.

---

# 49. Final Product Vision

The experience we are aiming for is this:

I find a random file months later:

```text
final_final_v3.pdf
```

I open Breadcrumb.

Search:

```text
final_final_v3.pdf
```

Breadcrumb immediately tells me:

```text
Downloaded from
canvas.up.edu.ph

on August 17, 2026

using Google Chrome

Original filename
CMSC199-Activity03.pdf

Originally saved to
Downloads

Renamed
August 18

Moved to
Documents\School\CMSC199

Current location
E:\UP\4th Year\CMSC199\
```

Or for a derived file:

```text
image001.png

Extracted from:
DENTEX.zip

DENTEX.zip was downloaded from:
zenodo.org

September 11, 2026
```

That is the core reason Breadcrumb exists.

Every technical or design decision should support that experience.

# Start Building

Begin by:

1. examining the repository
2. creating a short implementation plan
3. setting up the project foundation if necessary
4. defining the SQLite schema/migrations
5. implementing the Windows filesystem watcher
6. implementing file persistence
7. implementing Windows `Zone.Identifier` extraction
8. building the initial dashboard and file detail UI
9. running the application and validating the complete download → detection → provenance flow

Do not stop at generating an architecture document.

Implement the product.

When tradeoffs appear, prioritize:

**reliability → privacy → performance → simplicity → feature count.**