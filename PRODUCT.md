# Product

<!-- impeccable:product-schema 1 -->

## Platform

desktop

## Users

The TakeFlow team first (support, documentation, sales), and anyone who downloads it from GitHub Releases. They take many screenshots a day, annotate them quickly (arrows, boxes, numbered steps, blur) and paste them into chats, tickets and docs, sometimes with a styled background and a watermark for posts.

## Product Purpose

Capture an area of the screen in one keystroke (`PrtSc`), mark it up, and copy or save it without friction. A second step, the editor, turns the capture into a presentable image (gradient or solid background, margin, rounded corners, shadow, window bar, aspect ratio, logo or text watermark). Success: from key press to pasted image in a few seconds, with no stutter and no hunting for controls.

## Positioning

Instant capture with in-place annotation combined with a presentation editor (backgrounds, frame, watermark), local only (no cloud upload, no image search), lossless PNG at native resolution, on Windows, macOS and Linux.

## Operating Context

- Lives in the system tray; starts with the OS (optional), always minimized. `PrtSc` opens area capture fast from anywhere.
- Tray menu: capture area, open Start, Styles, Settings, About, Quit (quit only exists there).
- Main window: Start (capture actions plus a gallery of recent captures and images in the default folder), Styles (the single default style and watermark), Settings (grouped), About.
- Closing the main window asks once whether to minimize to tray or quit, remembers the answer, and the choice can be changed in Settings.
- Light, dark and follow-system themes across the whole app, including the capture overlay, the editor and (where the OS allows) the tray menu.

## Capabilities and Constraints

- Rust + egui/eframe 0.37, immediate-mode UI; single binary `takeshot`. Global hotkeys via `global-hotkey`, capture via `xcap`, rendering via `tiny-skia`.
- Tools: pen, line, arrow, rectangle, ellipse, highlighter, text, blur/pixelate, numbered steps. Planned: every annotation is an object that can be selected, moved, resized, restyled, duplicated and deleted after it is drawn.
- One default style and one watermark (not a list of saved presets). Copy/save straight from capture can apply the full style, rounded corners only, or nothing.
- Output is always lossless PNG at the screen's native resolution.
- On Linux the tray menu is drawn by the desktop (tray-icon 0.26 delivers no right-click events there) and follows the system theme; on Windows and macOS the app draws it in its own theme.

## Brand Commitments

- Product name: Takeshot. Needs a logo and app icon in every format (Windows .ico, macOS .icns, Linux PNG set, tray icons for light and dark).
- Design system based on TakeFlow's frontend: shadcn/ui neutral tokens, Roboto, 8px radius. Two complete, switchable themes, each with light, dark and system modes: **TakeUI** (TakeFlow family: purple `#8D43B1` actions, green `#3FCC59` for active, pill top navigation, own title bar) and **Coral UI** (own identity: coral `#FF6B47` actions with dark text, sidebar, native title bar). A theme changes palette, brand mark and layout; every feature must exist in both.
- The executable carries the TakeUI icon; window, taskbar and tray icons follow the active theme.
- Public repository `takeflow-oficial/takeshot`, AGPL-3.0-or-later, copyright Takeflow Sistemas. No "inspired by" credits in the app or README.
- Professional and modern, smooth controls without stutter, very little animation. Hover changes color only, never size.

## Evidence on Hand

- Current UI screenshots: `docs/home.png`, `docs/editor.png`, `docs/capture.png` (anti-reference for the look; the flow is kept).
- TakeFlow logo: `D:\Joseph\Desktop\DEV\takeflow\frontend\public\logo-takeflow.png`.
- No Takeshot logo exists yet; no testimonials, user counts or benchmarks exist and none may be invented.

## Product Principles

1. Speed first: the capture path never waits on the main window, a dialog or an animation.
2. Everything drawn stays editable until the image leaves the app.
3. The pointer always tells the truth about what a click will do.
4. Quiet chrome, loud content: the screenshot is the hero; controls recede.
5. Same app on every OS: one design system, native tray and startup behavior.
