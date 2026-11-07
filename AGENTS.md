# Takeshot agent guide

Takeshot is a tray-resident screenshot tool written in Rust with egui: capture with PrtSc, annotate with editable objects, style with backgrounds and watermarks. Product context and principles: [PRODUCT.md](PRODUCT.md). Visual source of truth: [design/takeshot-ui.fig](design/takeshot-ui.fig) (open it in OpenPencil or Figma).

## Parity

Takeshot ships two complete themes, **TakeUI** and **Coral UI**, each with light, dark and system modes. Parity means every user-facing function exists in both themes, each in the place its theme's design gives it. A visible change is done only when both themes have it. Use the `implementar-nos-dois-temas` skill for any change a user can see.

The redesign that introduces the two themes is in progress on `feat/redesign`. Until both skins exist, change the current interface and list in the PR what the other theme still needs.

## Layout

- Every `src/*.rs` file starts with a `//!` doc comment (after any crate attributes) stating its responsibility; read those lines to find your way, and give every new file one.
- Target architecture: a theme-agnostic core (state, settings, objects, capture, render, actions, tray, hotkeys) plus two skins, `takeui` and `coral`, that only draw and call core actions. Keep behavior in the core so both skins share it.

## Interface rules

- Hover and press change color only; every widget keeps its size and position (egui `expansion` stays 0).
- The pointer shows what a click will do: arrow on bars and panels, crosshair only while drawing on the image, move over draggable objects and inside the capture area, resize on handles, text cursor while typing.
- Every user-facing string exists in English and Portuguese, added in the same change.
- Motion is limited to 80 ms color transitions and the 150 ms "Copied" notice.
- All code, assets and values are original work or come from openly licensed sources listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Verify before calling work done

1. `cargo fmt --check` prints nothing.
2. `cargo clippy --all-targets -- -D warnings` finishes with zero warnings, pre-existing ones included.
3. `cargo test > shots/test.log 2>&1`, then read the log. Write test output to a file and read it; run one test command at a time.
4. For visible changes, `takeshot --screenshot out.png` renders the window and saves it; compare it with the design file. Theme, mode, language and page flags arrive with the redesign.

## Git

- Branch `feat/<topic>` or `fix/<topic>` from `main`, open a PR to `main` and fill in the template.
- Commit messages: English, [Conventional Commits](https://www.conventionalcommits.org/).
