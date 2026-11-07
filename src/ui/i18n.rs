//! Interface language: automatic (follows the system), English or Portuguese, and the translated strings.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Pt,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum LangPref {
    #[default]
    Auto,
    En,
    Pt,
}

/// Automatic picks Portuguese for any `pt` locale (`pt`, `pt-BR`, `pt_BR.UTF-8`) and English otherwise.
pub fn resolve(pref: LangPref, system_locale: Option<&str>) -> Lang {
    match pref {
        LangPref::En => Lang::En,
        LangPref::Pt => Lang::Pt,
        LangPref::Auto => {
            let l = system_locale.unwrap_or("").to_ascii_lowercase();
            let pt = l == "pt" || l.starts_with("pt-") || l.starts_with("pt_");
            if pt { Lang::Pt } else { Lang::En }
        }
    }
}

pub fn system_locale() -> Option<String> {
    sys_locale::get_locale()
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn lang() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::Pt,
        _ => Lang::En,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Msg {
    Theme,
    ThemeTakeUi,
    ThemeCoral,
    Mode,
    ModeLight,
    ModeDark,
    ModeSystem,
    Language,
    LangAuto,
    LangEn,
    LangPt,
    Copy,
    Save,
    Open,
    Delete,
    Search,
    AskOnClose,
    PrtScReady,
    PrtScBusy,
    GalleryTitle,
}

pub fn t(m: Msg) -> &'static str {
    match lang() {
        Lang::En => en(m),
        Lang::Pt => pt(m),
    }
}

/// `t` with `{0}`, `{1}`… replaced by `args` in order.
pub fn tf(m: Msg, args: &[&str]) -> String {
    let mut s = t(m).to_string();
    for (i, a) in args.iter().enumerate() {
        s = s.replace(&format!("{{{i}}}"), a);
    }
    s
}

fn en(m: Msg) -> &'static str {
    match m {
        Msg::Theme => "Theme",
        Msg::ThemeTakeUi => "TakeUI",
        Msg::ThemeCoral => "Coral UI",
        Msg::Mode => "Mode",
        Msg::ModeLight => "Light",
        Msg::ModeDark => "Dark",
        Msg::ModeSystem => "System",
        Msg::Language => "Language",
        Msg::LangAuto => "Automatic",
        Msg::LangEn => "English",
        Msg::LangPt => "Português",
        Msg::Copy => "Copy",
        Msg::Save => "Save",
        Msg::Open => "Open folder",
        Msg::Delete => "Delete",
        Msg::Search => "Search captures",
        Msg::AskOnClose => "Ask",
        Msg::PrtScReady => "PrtSc ready",
        Msg::PrtScBusy => "PrtSc in use by another app",
        Msg::GalleryTitle => "Components",
    }
}

fn pt(m: Msg) -> &'static str {
    match m {
        Msg::Theme => "Tema",
        Msg::ThemeTakeUi => "TakeUI",
        Msg::ThemeCoral => "Coral UI",
        Msg::Mode => "Modo",
        Msg::ModeLight => "Claro",
        Msg::ModeDark => "Escuro",
        Msg::ModeSystem => "Sistema",
        Msg::Language => "Idioma",
        Msg::LangAuto => "Automático",
        Msg::LangEn => "English",
        Msg::LangPt => "Português",
        Msg::Copy => "Copiar",
        Msg::Save => "Salvar",
        Msg::Open => "Abrir pasta",
        Msg::Delete => "Excluir",
        Msg::Search => "Buscar capturas",
        Msg::AskOnClose => "Perguntar",
        Msg::PrtScReady => "PrtSc pronto",
        Msg::PrtScBusy => "PrtSc em uso por outro app",
        Msg::GalleryTitle => "Componentes",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_follows_portuguese_locales() {
        for l in ["pt-BR", "pt_BR.UTF-8", "pt", "PT-pt"] {
            assert_eq!(resolve(LangPref::Auto, Some(l)), Lang::Pt, "{l}");
        }
    }

    #[test]
    fn auto_falls_back_to_english() {
        for l in [Some("en-US"), Some("es-ES"), Some("ptx"), Some(""), None] {
            assert_eq!(resolve(LangPref::Auto, l), Lang::En, "{l:?}");
        }
    }

    #[test]
    fn explicit_choice_wins() {
        assert_eq!(resolve(LangPref::En, Some("pt-BR")), Lang::En);
        assert_eq!(resolve(LangPref::Pt, Some("en-US")), Lang::Pt);
    }

    #[test]
    fn translates_and_formats() {
        set_lang(Lang::Pt);
        assert_eq!(t(Msg::ModeSystem), "Sistema");
        set_lang(Lang::En);
        assert_eq!(t(Msg::ModeSystem), "System");
        assert_eq!(tf(Msg::GalleryTitle, &[]), "Components");
    }
}
