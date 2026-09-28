# pesu-rs

Unofficial, lightweight, minimal PESU Academy client — Tauri + Rust + Vanilla JS. Dark minimal UI, tabular numbers, blob animations, persistent sidebar.

> No tracking. Credentials stay on device via OS keyring.

## Features

- **Home** — profile (SRN, Sem, Course)
- **Timetable** — Mon-Sat pill with active blob, auto today, vertical timeline, break row, meta chips (Batch, Class, Dept, Section, Room)
- **Seating** — cute cards: assessment + course, date/time top-right, Block/Terminal big bottom
- **Attendance** — course-wise % with progress bar, status (Safe/Borderline/Shortage), bunk calc (can miss / need), summary bar
- **Courses** — semester select auto-picks top / matching home sem (e.g. Sem-3), course list
- **Settings** — accent dots + custom color picker (centered +), density compact/cozy, font S/M/L, fullscreen, clear saved creds
- **Credits** — only Stack + Scraper, GitHub avatar preloaded on start and cached as dataURL in localStorage

## Stack

- **Frontend:** Vanilla JS, HTML, CSS (no framework)
- **Backend:** Tauri v2, Rust, `reqwest` + `scraper` + `keyring` + `regex-lite`
- **Design:** dark minimal, `color-mix()` tints, `--r` / `--r-lg` radius, `cubic-bezier(.16,1,.3,1)` ease

## Quick Start

```bash
# install Rust + Tauri CLI
cargo install tauri-cli

# dev
cargo tauri dev

# build
cargo tauri build
```

Frontend lives in root (`index.html`, `style.css`, `main.js`). Backend in `src-tauri/` (`main.rs`, `scraper.rs`, `tauri.conf.json`).

## Scraper Endpoints

- `studentProfilePESU` — profile
- `studentProfilePESUAdmin?menuId=651/653` — semesters
- `studentProfilePESUAdmin POST controllerMode=6403 actionType=38 id=semId` — courses
- `studentProfilePESUAdmin?menuId=655 controllerMode=6404` — seating
- `studentProfilePESUAdmin?menuId=669 controllerMode=6415` — timetable (parses `timeTableTemplateDetailsJson`, `days`, `timeTableJson`)
- `studentProfilePESUAdmin?menuId=660 controllerMode=6407 actionType=8 batchClassId=semId` — attendance (GET + POST fallback, parses `#subjetInfo`)

All requests include `x-csrf-token`, `X-Requested-With`, timestamp `_`, with 403 refresh via `studentProfilePESU`.

## Config

`tauri.conf.json`:

- `frontendDist: ../ui`
- window 1280x800, resizable false, centered, maximized
- CSP: `img-src self data: avatars.githubusercontent.com github.com`, `connect-src self avatars.githubusercontent.com api.github.com github.com`

## Credits

- **Vision2822** — creator & maintainer — [github.com/Vision2822](https://github.com/Vision2822)
- Stack: Tauri + Rust + Vanilla JS
- Scraper: reqwest + scraper + keyring

## License

MIT
