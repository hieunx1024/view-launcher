# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`view-launcher` is a cross-platform (Linux + Windows) desktop app launcher written in Rust, using the [Slint](https://slint.dev) UI toolkit with the FemtoVG/OpenGL renderer (software renderer fallback on Windows). It provides an instant search overlay for applications, files/folders, emoji, an inline calculator/unit converter, window switching, and clipboard history. The core design goal is sub-5ms toggle latency via a persistent single-instance daemon.

## Build & Test Commands

```bash
# Build (debug / release)
cargo build
cargo build --release

# Run all tests — on Linux this REQUIRES a virtual display because Slint
# creates a real window even in unit tests; CI uses `xvfb-run`.
xvfb-run cargo test --verbose      # Linux
cargo test --verbose               # Windows (no xvfb needed)

# Run a single test by name
xvfb-run cargo test test_search_and_icon_performance

# Run the app
cargo run --release
```

Linux build dependencies (Debian/Ubuntu): `libxkbcommon-dev libfontconfig1-dev libwayland-dev libegl1-mesa-dev libgl1-mesa-dev libx11-dev`.

Almost all unit tests live inline as a `#[cfg(test)] mod tests` block at the bottom of [src/launcher.rs](src/launcher.rs) (~line 1600+) and exercise `LauncherEngine::search()` end-to-end across every search mode (apps, files, calculator, emoji, windows, clipboard, plugins, theme). When changing ranking/matching behavior, that's the block to check and extend.

CI (`.github/workflows/ci.yml`) builds both a Linux `.deb` (via manual `dpkg-deb`) and a Windows installer (via Inno Setup, `installer/windows/setup.iss`) on every push/PR to `main`.

## Architecture

### Process model: single persistent daemon, not a normal launch-per-invocation app

[src/main.rs](src/main.rs) `main()` is intentionally structured so the *hot path* — invoking the launcher when it's already running — does the least possible work:

1. Before creating any window or GPU context, it tries to reach an already-running instance over IPC (Unix domain socket at `$XDG_RUNTIME_DIR/view-launcher.sock`, or a TCP loopback on `127.0.0.1:42425` on Windows) and just sends a `"toggle"` message. If that succeeds, the process exits immediately — this is the documented "<0.2ms" path.
2. If no instance is running, this process *becomes* the daemon: it creates the Slint window once, starts a background thread listening on that same socket/port (`start_daemon_listener`), and then keeps running with `ui.run()`. Subsequent invocations of the binary just hit step 1 and toggle this same window.
3. `--quit` sends a `"quit"` message over the same channel to stop the daemon; `--dmenu` bypasses the daemon entirely and runs a one-shot dmenu-style picker to stdout.

Because of this, "app is slow to open" reports usually mean one of two different things and the fix differs accordingly:
- **First launch (cold daemon start)** — dominated by `LauncherEngine::new()` / `IconResolver::new()` work below.
- **Toggle of an already-running daemon** — should be near-instant (IPC round trip only); if this is slow, look at `start_daemon_listener` and the Slint `show()`/focus path in `main.rs`, not the indexers.

### Startup indexing (cold-start critical path)

`LauncherEngine::new()` ([src/launcher.rs](src/launcher.rs)) runs `index_apps()` **synchronously** on the main thread before the window is usable, then kicks off filesystem file/folder indexing (`index_files_impl`) on a background thread (results land in `shallow_files: Arc<RwLock<Vec<_>>>`, so search degrades gracefully until that finishes).

`index_apps()` has separate Linux/Windows implementations behind `#[cfg]`:
- Linux: reads `.desktop` files from `/usr/share/applications`, `~/.local/share/applications`, flatpak/snap dirs, and `$XDG_DATA_DIRS`.
- Windows: uses `walkdir::WalkDir` to recursively scan the Start Menu (`%APPDATA%\Microsoft\Windows\Start Menu\Programs`), `C:\ProgramData\...\Start Menu\Programs`, and the user/public Desktop for `.lnk`/`.url` files — this directory tree tends to be much larger than Linux's flat `applications` dirs, so it's the primary Windows-specific cold-start cost.

`IconResolver` ([src/icon_resolver.rs](src/icon_resolver.rs)) is a separate cache (`image_cache`, `file_type_cache`, `icon_path_index`, all `Arc<RwLock<...>>`) keyed by icon hint / exec path / app name. On Windows, `IconResolver::extract_windows_icon()` pulls native icons out of `.lnk`/`.exe` files via `windows-sys` Shell APIs (`SHGetFileInfoW`/`PrivateExtractIconsW`, which internally resolves `.lnk` targets via COM) — real per-file Win32 work, not a cheap lookup, and for a full Start Menu it used to be the dominant cold-start cost: `main()` called `IconResolver::preload_icons()` synchronously for every indexed app *before* `ui.run()`, so the window couldn't paint or accept input until every icon had been extracted.

**This was fixed**: on Windows, `main()` now calls `warm_icon_cache_chunked()` ([src/main.rs](src/main.rs)) instead, which resolves icons a small batch at a time via repeated `slint::Timer::single_shot` calls, so the event loop gets to run (window paints, accepts input) between batches rather than blocking on the whole list. Note `slint::Image` is not `Send`/`Sync` (verified via `cargo check --target x86_64-pc-windows-gnu`; the type contains a GPU-backend-owned variant), so this work could *not* simply be moved to a background `std::thread` — that's why it's chunked cooperatively on the UI thread instead of parallelized. If you revisit this, `index_apps()`'s Windows branch ([src/launcher.rs:260](src/launcher.rs:260)) — a synchronous `WalkDir` scan of the Start Menu — is the next-largest remaining synchronous cold-start cost, though it's just directory listing (no per-file Win32 calls) and much cheaper than icon extraction was.

### Module layout (`src/lib.rs`)

- `launcher` — `LauncherEngine`: app/file indexing, fuzzy search & ranking (`fuzzy-matcher` SkimMatcherV2 + custom tiered scoring), launching items, Vietnamese accent-insensitive matching (`remove_vietnamese_accents`).
- `icon_resolver` — native icon extraction/caching (`.desktop` icon themes on Linux, Shell icon extraction on Windows), plus SVG rasterization (`resvg`/`usvg`/`tiny-skia`) for bundled UI glyphs.
- `config` — `Config` struct (serde + `toml`), loaded from/saved to `~/.config/view-launcher/config.toml` (Linux) or `%APPDATA%\view-launcher\config.toml` (Windows); also owns autostart registration and global-hotkey setup.
- `calc` — inline expression parser/evaluator (arithmetic, hex/binary literals, unit & currency conversion).
- `emoji` — offline emoji picker data + EN/VI keyword search.
- `plugins` — Rofi-style external script plugin system, scripts loaded from `~/.config/view-launcher/plugins/`.
- `window_switcher` — enumerates and focuses open windows (platform-specific).
- `clipboard` — clipboard history manager (`arboard`-backed).
- `history` — recently-launched item tracking, persisted to disk.
- `system_actions` — built-in commands (e.g. lock/shutdown-style actions) surfaced as search results.

The UI itself is a single Slint file, [ui/app_window.slint](ui/app_window.slint), compiled by `slint-build` in [build.rs](build.rs) into generated Rust exposed via `slint::include_modules!()` in `lib.rs`. Rust ↔ UI communication is through generated setters/getters and callbacks on the `AppWindow` component (see the `ui.set_*` / `ui.on_*` calls in `main.rs`).

### Note on `.agents/`

The `.agents/` directory is a vendored generic multi-agent toolkit ("AG Kit") scoped to Gemini CLI / Google Antigravity, not project-specific documentation for view-launcher — its own conventions explicitly say not to claim compatibility with Claude Code. Don't treat its contents as guidance for this repo.
