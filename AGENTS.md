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
| `src/system/` | operating system and data: screen capture, clipboard and files, fonts, settings (tray, hotkeys and autostart join here) |
| `src/ui/` | design system: tokens, translations, icons, style, component kit, brand |
| `src/screens/` | current windows; the skins replace them screen by screen |
| `src/skins/` | `takeui/` and `coral/`: they only draw and call engine and system code |

Every `.rs` file starts with a `//!` doc comment (after any crate attributes) stating its responsibility; read those lines to find your way, and give every new file one. Behavior lives in `engine/` and `system/` so both skins share it.

## Interface rules

- Hover and press change color only; every widget keeps its size and position (egui `expansion` stays 0).
- The pointer shows what a click will do: arrow on bars and panels, crosshair only while drawing on the image, move over draggable objects and inside the capture area, resize on handles, text cursor while typing.
- Every user-facing string exists in English and Portuguese, added in the same change.
- Motion is limited to 80 ms color transitions and the 150 ms "Copied" notice.
- All code, assets and values are original work or come from openly licensed sources listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Verify before calling work done

1. `cargo fmt --check` prints nothing.
2. `cargo clippy --all-targets -- -D warnings` finishes with zero warnings, pre-existing ones included.
3. `mkdir -p shots` once (the folder is git-ignored), then `cargo test > shots/test.log 2>&1` and read the log. Send test output to a file and run one test command at a time: a pipe holds a hung run open.
4. For visible changes, `takeshot --screenshot out.png` renders the window and saves it; compare it with the design file. Theme, mode, language and page flags arrive with the redesign.

## Git

- Branch `feat/<topic>` or `fix/<topic>` from `main`, open a PR to `main` and fill in the template.
- Commit messages: English, [Conventional Commits](https://www.conventionalcommits.org/).
