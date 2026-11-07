//! Design system shared by both skins: embedded assets, tokens, translations, icons, style and components.

// The design system is built in steps before its first caller exists; the component
// gallery (plan 2, Task 7) uses every item, and this allow is removed there.
#![allow(dead_code)]

pub mod assets;

pub mod tokens;
