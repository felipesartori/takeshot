# Third-party notices

Takeshot is licensed under the GNU Affero General Public License v3.0 or later (see [LICENSE](LICENSE)). The binary embeds the third-party resources below, each under its own license.

## Fonts embedded through `epaint_default_fonts` 0.37.0

Takeshot enables egui's `default_fonts` feature, which embeds the three fonts below; their license files ship in the crate's `fonts/` directory (<https://crates.io/crates/epaint_default_fonts>). The crate's emoji fonts (Noto Emoji and the full emoji-icon-font) sit behind its `monochrome_emoji_fonts` feature, which Takeshot does not enable, so they are not in the binary.

### Ubuntu Light

- **File:** `Ubuntu-Light.ttf`
- **License:** Ubuntu Font Licence 1.0 (`UFL.txt`)
- **Source:** <https://design.ubuntu.com/font>
- **Copyright:** Copyright 2011 Canonical Ltd.

### Hack Regular

- **File:** `Hack-Regular.ttf`
- **License:** MIT License; the Bitstream Vera Sans Mono portions under the Bitstream Vera License; the DejaVu portions in the public domain (`Hack-Regular.txt`)
- **Source:** <https://github.com/source-foundry/Hack>
- **Copyright:** Copyright (c) 2018 Source Foundry Authors; Copyright (c) 2003 by Bitstream, Inc.

### egui-icons

- **File:** `egui-icons.ttf`, a five-glyph subset of emoji-icon-font (the logos egui exposes as `egui::special_emojis`)
- **License:** MIT License (`emoji-icon-font-mit-license.txt`)
- **Source:** <https://github.com/jslegers/emoji-icon-font>
- **Copyright:** Copyright (c) 2014 John Slegers

## Rust crates

Takeshot is built from open-source Rust crates. The full list of crates with their license texts is generated for each release.
