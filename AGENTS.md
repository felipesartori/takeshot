# Takeshot agent guide

Takeshot is a screenshot tool written in Rust with egui: capture with PrtSc, annotate, and style the result with backgrounds and watermarks. The redesign in progress turns it into a tray utility with editable objects. Product context and principles: [PRODUCT.md](PRODUCT.md). Visual source of truth: [design/takeshot-ui.fig](design/takeshot-ui.fig) (open it in OpenPencil or Figma).

## Parity

Takeshot ships two complete themes, **TakeUI** and **Coral UI**, each with light, dark and system modes. Parity means every user-facing function exists in both themes, each in the place its theme's design gives it. A visible change is done only when both themes have it. Use the `implementar-nos-dois-temas` skill for any change a user can see.

The redesign that introduces the two themes is in progress on `feat/redesign`. Until both skins exist, change the current interface and list in the PR what the other theme still needs.

## Layout

| Folder | Holds |
|---|---|
| `src/main.rs` | entry point: arguments, shared state, window modes |
| `src/engine/` | drawing engine: annotations, objects, pointer policy, rendering, background presets |
| `src/system/` | operating system and data: screen capture and monitor work areas, clipboard and files, fonts, settings, and the resident's parts (`resident`, `tray`, `hotkeys`, `prtsc_hook`, `instance`, `autostart`, `recents`, `borderless`) |
| `src/ui/` | design system: `assets` (embedded fonts and the icon font), `tokens` (colours per theme and mode), `i18n` (strings), `icon` (Lucide), `style` (egui visuals), `kit` (components), `brand` (icons and mark), `chrome` (the theme's title bar: drag, double click, resize edges, window buttons, the native fallback), `gallery` (all of it on one screen) |
| `src/screens/` | window state and behaviour shared by both themes (`home`, `editor` and `overlay` hold the main window's, the editor's and the capture's state and carry out their `MainAction`s, `EditorAction`s and `CaptureAction`s; the editor's canvas is `Editor::canvas`), the windows not yet skinned (pin, notice), and the resident's `shell` (main window, tray menu, About and close dialogs) |
| `src/skins/` | how each theme draws: `takeui/` (and `coral/` from plan 6) draw the main window, the editor (bars, panel, object bar) and the capture's bars from their state and return what the user asked for as actions, which `screens/` carries out; `begin_frame` applies the look when it changes. Until Coral UI has its own pages, choosing it colours TakeUI's layout |
| `assets/` | embedded fonts with their licenses, and the brand SVG masters with the PNG, ICO and ICNS files generated from them |

Every `.rs` file starts with a `//!` doc comment (after any crate attributes) stating its responsibility; read those lines to find your way, and give every new file one. Behavior lives in `engine/` and `system/` so both skins share it.

## Interface rules

- Hover and press change color only; every widget keeps its size and position (egui `expansion` stays 0).
- The pointer shows what a click will do: arrow on bars and panels, crosshair only while drawing on the image, move over draggable objects and inside the capture area, resize on handles, text cursor while typing.
- Every user-facing string is a `Msg` key in `src/ui/i18n.rs` with English and Portuguese text; the exhaustive `match` turns a missing translation into a compile error. Show it with `t(Msg::…)`.
- Widgets come from `src/ui/kit.rs` and colours from `Tokens`; a new widget joins the kit with a size that never depends on hover, and the kit test covers it.
- Motion is limited to 80 ms color transitions and the 150 ms "Copied" notice.
- All code, assets and values are original work or come from openly licensed sources listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Objects on the picture

Editor and capture share one behaviour, so both skins get it for free:

- Annotations are objects in `engine::doc::Doc`: ids, selection, transforms and an undo history of snapshots. A whole drag is one undo step (call `begin_change` once, then `translate_selected` or `resize` per frame). Selecting never changes `revision`, which is how the editor knows there is nothing to save.
- Screens route the pointer and keys through `Annotator::handle_canvas`, `handle_object_keys` and `handle_history_keys`, and set the pointer over the picture from `engine::cursor::pick` instead of picking icons by hand.
- A slider drag is one undo step too: a skin sends `Gesture(true)` when it is grabbed and `Gesture(false)` when let go (`Doc::begin_gesture`, `end_gesture`); a frame with no button down closes one left open. Restyles go through `Doc::restyle_selected(&Restyle)`, whose colour and opacity are apart, so a multi-selection keeps each object's other half. Picking from a list goes through `Annotator::pick`, which copies the object's style into the options, as a click on the canvas does.
- Each frame: `bake()` first (it returns annotations to render only when the picture is out of date), then the pointer, then `paint_selection`. During a drag only its start re-bakes, to take the dragged objects out, and `paint_selection` paints `painted_live()` until the drop re-bakes. A big capture never re-renders while a drag moves something; arrow nudges and Ctrl+D re-render once per press.

## The resident

`takeshot` with no arguments starts the resident: one process per user owns the tray icon, the global hotkeys and the main window, and a second launch only shows it. Captures, editors and pinned images run as their own processes (`--capture`, `--edit`, `--pin`).

- Event sources (hotkeys, the PrtSc hook, the tray, a second launch) only send a `ResidentEvent` and request a repaint; `App::logic` drains them, so they act while no window is shown. On Windows and macOS the root viewport is an invisible host and the main window a child viewport; on Linux the root is the main window.
- A capture whose selection is on screen holds `instance::CaptureSlot`; another `--capture` exits before grabbing the screen.
- A borderless, rounded, transparent window on Windows needs `borderless::flatten_borderless`: egui-winit always asks winit for the undecorated shadow, which shows as a 1 px line across the top and a square shadow around the corners.

## Verify before calling work done

1. `cargo fmt --check` prints nothing.
2. `cargo clippy --all-targets -- -D warnings` finishes with zero warnings, pre-existing ones included.
3. `mkdir -p shots` once (the folder is git-ignored), then `cargo test > shots/test.log 2>&1` and read the log. Send test output to a file and run one test command at a time: a pipe holds a hung run open.
4. For visible changes, render the screens and compare them with the design file: `design/screens/` holds the TakeUI references exported from the `.fig`, and the exact measures (sizes, padding, gaps, font sizes) are on the `.fig`'s nodes, read through OpenPencil. `--screenshot out.png` saves the window after ~120 frames and works with every window: `takeshot --screenshot out.png` (main window), `takeshot --edit --screenshot out.png` (editor with a sample picture), `takeshot --capture --screenshot out.png` (capture overlay on a sample). `takeshot` alone starts the resident (close a running one first); debug builds are console programs, so a script that launches one must redirect its output or the new console window takes the focus. `--lang en|pt|auto`, `--theme takeui|coral` and `--mode light|dark|system` override the saved look for this run in every window drawn from the tokens; `--page inicio|estilo|ajustes` opens the main window on that page, `--page editor` the editor and `--page captura` the capture (both on the sample picture, the capture with an area and a mark), and `--page componentes` the component gallery; `--captures-dir <folder>` shows another folder in Início. None of them touches the saved settings. Screenshots for the repo or a PR use a sample folder, never the user's own captures. `scripts/screenshots.sh` renders the gallery, the three pages, the editor and the capture in Portuguese in all four theme/mode combinations into `shots/`, with `design/screens` as Início's captures (build with `cargo build --release` first).
5. Before a push, `scripts/check.sh` runs what CI runs: fmt, clippy and tests here, then the same on Linux in a container (`scripts/ci-linux`, needs Docker) plus clippy for macOS, on the commit being pushed. Code behind `#[cfg(...)]` for another system compiles only there, so steps 1 to 3 on Windows miss it. The pre-push hook runs it once a clone has `git config core.hooksPath .githooks`.

## Tools

- `scripts/dev.sh [takeshot arguments]`: live preview. Rebuilds into `target/dev-watch` and reopens the window whenever `src/`, `assets/` or `Cargo.toml` change (default `--page componentes`).
- `cargo run --example brand`: renders `assets/brand/*.svg` into the PNGs, `takeshot.ico` and `Takeshot.icns`. Run it after changing an SVG and commit the outputs; `build.rs` embeds `takeshot.ico` in the Windows executable.

## Git

- Branch `feat/<topic>` or `fix/<topic>` from `main`, open a PR to `main` and fill in the template.
- Commit messages: English, [Conventional Commits](https://www.conventionalcommits.org/).
