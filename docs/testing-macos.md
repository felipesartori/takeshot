# Testing on macOS

CI builds and lints the macOS app but cannot click through it. Before a release, someone with a Mac runs this list on the `takeshot-X.Y.Z-macos-universal.dmg` from the release (or from the `macos-universal` artifact of a workflow run). Note the macOS version and the chip (Apple Silicon or Intel) in the PR or the release discussion, with what failed.

## Install

1. Open the `.dmg` and drag Takeshot to Applications.
2. The app is not signed by Apple yet. The first time, right-click Takeshot in Applications > Open, then Open again. If macOS says the app is damaged, run `xattr -cr /Applications/Takeshot.app` and open it again.
3. Expect the Takeshot icon in the menu bar and the main window (Início).

## Screen recording

4. Press `Ctrl+Shift+2` (area). macOS asks for Screen Recording: allow it in System Settings > Privacy & Security > Screen & System Audio Recording, then quit Takeshot from the menu bar icon and open it again.
5. `Ctrl+Shift+2` again: the screen dims, drag an area. Expect the tools in a column beside the area, the options next to them and Pin, Save, Edit and Copy under the area. Draw a rectangle and an arrow, press Copy, paste in Preview (File > New from Clipboard).
6. `Ctrl+Shift+1` (full screen) opens the editor with the whole screen.
7. On a second monitor, if there is one: the area capture works there and the bars stay on that monitor.

## Windows and title bar

8. The main window and the editor draw their own 44 px title bar with the traffic lights inside it, vertically centred with the bar's buttons. Drag the window by the bar, double-click it to zoom, resize it from the edges.
9. Switch the theme in Ajustes (TakeUI and Coral UI, light and dark): every window follows, the menu bar icon included.

## Menu bar icon

10. Click the icon: the menu shows capture area, full screen and in 3 seconds, open Takeshot, the recent captures, style, settings, theme, about and quit.
11. A recent capture copies on click and opens in the editor with Shift+click.

## Pin and editor

12. From a capture, Pin: the picture floats over other windows. Drag the middle to move it, a corner to resize it (it keeps its proportions), scroll to zoom, right-click for the menu, double-click or Esc to close.
13. In the editor, add a text, change its size with the wheel, undo and redo, save with `Cmd+S`, and check the PNG in Finder.

## Start at login and quit

14. Ajustes > Geral > "Iniciar com o sistema": log out and in. Takeshot starts in the menu bar only. The entry is `~/Library/LaunchAgents/Takeshot.plist`.
15. Open an editor, draw something, then quit from the menu bar icon: the editor asks (Discard, Cancel, Save and close). Cancel keeps everything open; Discard closes everything.

## Uninstall

16. Quit Takeshot, drag it from Applications to the Trash. Its data: settings in `~/Library/Application Support/takeshot`, thumbnails in `~/Library/Caches/takeshot`, the login entry above.
