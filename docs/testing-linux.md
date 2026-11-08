# Testing on Linux

Two scripts check the Linux packages in Docker, from Windows, macOS or Linux:

```sh
scripts/linux-packages.sh   # builds the .deb, .AppImage and .tar.gz in the CI image (scripts/ci-linux)
scripts/linux-smoke.sh      # opens them in clean Ubuntu 24.04 containers and saves what they drew
```

`linux-packages.sh` puts the packages in `target/packages-linux` and prints the shared libraries the binary needs (`dpkg-shlibdeps`), which is where the `.deb`'s `depends` in `Cargo.toml` comes from. `linux-smoke.sh` then:

- installs the `.deb` with `apt` (which pulls its dependencies) and runs `takeshot --screenshot` on **X11** (Xvfb) and on **Wayland** (sway with no screen, `WLR_BACKENDS=headless`);
- runs the AppImage on X11 in a container with no GTK installed, so it must carry its own libraries.

The pictures land in `target/packages-linux/smoke`. Look at each one: the window drawn whole, in the theme asked for.

## What the containers cannot check

These need a desktop session and go through by hand before a release (a VM or a Linux machine):

1. The tray icon and its menu (an AppIndicator panel: GNOME with the AppIndicator extension, KDE, XFCE with the StatusNotifier plugin).
2. `PrtSc` and `Shift+PrtSc` on X11; on Wayland, a desktop shortcut running `takeshot --capture`, and the capture through the desktop portal.
3. Settings > General > "Start with the system": log out and in; the entry is `~/.config/autostart/Takeshot.desktop`.
4. Quit from the tray (or Settings > General when there is no tray) closes the editors and pinned pictures too.

## Last run

(filled in by the last run of `scripts/linux-smoke.sh`)
