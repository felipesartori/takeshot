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
    GalleryHeading,
    SecButtons,
    SecTools,
    SecControls,
    SecMenu,
    SecSelection,
    SecIcons,
    SecType,
    SecColors,
    SecBrand,
    Home,
    Style,
    Settings,
    CaptureArea,
    FullScreen,
    Disabled,
    Margin,
    ShowNotice,
    StartWithSystem,
    Recent,
    About,
    Quit,
    TypeTitle,
    TypeSection,
    TypeBody,
    TypeLabel,
    TypeCaption,
    DiscardTitle,
    DiscardBody,
    Discard,
    CopyAndClose,
    Cancel,
    CaptureIn3s,
    OpenTakeshot,
    StyleAndWatermark,
    OpenCapturesFolder,
    Today,
    Yesterday,
    RecentHint,
    CloseAppTitle,
    CloseAppBody,
    RememberChoice,
    MinimizeToTray,
    OnCloseLabel,
    TrayClickLabel,
    OpenHome,
    QuitTakeshot,
    GeneralSection,
    PrtScPriority,
    PrtScPriorityHint,
    OpenWindowsSettings,
    PrtScConflictHelp,
    ChangeKeys,
    RestoreDefault,
    PressKeys,
    WaylandHotkeyHelp,
    AboutPhrase,
    MadeBy,
    LicenseAgpl,
    ViewOnGitHub,
    VersionN,
    CloseWord,
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
        Msg::GalleryHeading => "Components · {0}",
        Msg::SecButtons => "Buttons",
        Msg::SecTools => "Tools and navigation",
        Msg::SecControls => "Controls",
        Msg::SecMenu => "Menu",
        Msg::SecSelection => "Selection",
        Msg::SecIcons => "Icons",
        Msg::SecType => "Typography",
        Msg::SecColors => "Colours",
        Msg::SecBrand => "Brand",
        Msg::Home => "Home",
        Msg::Style => "Style",
        Msg::Settings => "Settings",
        Msg::CaptureArea => "Capture area",
        Msg::FullScreen => "Full screen",
        Msg::Disabled => "Disabled",
        Msg::Margin => "Margin",
        Msg::ShowNotice => "Show a notice after copying",
        Msg::StartWithSystem => "Start with the system",
        Msg::Recent => "Recent captures",
        Msg::About => "About Takeshot",
        Msg::Quit => "Quit",
        Msg::TypeTitle => "Page title",
        Msg::TypeSection => "Section title",
        Msg::TypeBody => "Body text and fields",
        Msg::TypeLabel => "Label and button",
        Msg::TypeCaption => "Caption and metadata",
        Msg::DiscardTitle => "Discard changes?",
        Msg::DiscardBody => "This picture has edits that were not copied or saved.",
        Msg::Discard => "Discard",
        Msg::CopyAndClose => "Copy and close",
        Msg::Cancel => "Cancel",
        Msg::CaptureIn3s => "Capture in 3 seconds",
        Msg::OpenTakeshot => "Open Takeshot",
        Msg::StyleAndWatermark => "Style and watermark",
        Msg::OpenCapturesFolder => "Open captures folder",
        Msg::Today => "Today",
        Msg::Yesterday => "Yesterday",
        Msg::RecentHint => "Click copies · Shift+click opens",
        Msg::CloseAppTitle => "Close Takeshot?",
        Msg::CloseAppBody => {
            "Minimized, it keeps running in the tray and PrtSc keeps working. To quit for good, use Quit in the tray menu."
        }
        Msg::RememberChoice => "Remember my choice",
        Msg::MinimizeToTray => "Minimize to tray",
        Msg::OnCloseLabel => "When clicking X",
        Msg::TrayClickLabel => "Clicking the tray icon",
        Msg::OpenHome => "Open Home",
        Msg::QuitTakeshot => "Quit Takeshot",
        Msg::GeneralSection => "General",
        Msg::PrtScPriority => "Use Takeshot priority",
        Msg::PrtScPriorityHint => "Some antivirus software may flag this.",
        Msg::OpenWindowsSettings => "Open Windows settings",
        Msg::PrtScConflictHelp => {
            "Turn off \"Use the Print Screen key to open screen capture\" in Settings > Accessibility > Keyboard."
        }
        Msg::ChangeKeys => "Change",
        Msg::RestoreDefault => "Restore default",
        Msg::PressKeys => "Press the new keys. Esc cancels.",
        Msg::WaylandHotkeyHelp => "On Wayland, create a desktop shortcut that runs: {0}",
        Msg::AboutPhrase => {
            "Screen capture, annotation and finishing. All local, no cloud, lossless PNG at the original resolution."
        }
        Msg::MadeBy => "Made by TakeFlow",
        Msg::LicenseAgpl => "License AGPL-3.0",
        Msg::ViewOnGitHub => "View on GitHub",
        Msg::VersionN => "Version {0}",
        Msg::CloseWord => "Close",
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
        Msg::GalleryHeading => "Componentes · {0}",
        Msg::SecButtons => "Botões",
        Msg::SecTools => "Ferramentas e navegação",
        Msg::SecControls => "Controles",
        Msg::SecMenu => "Menu",
        Msg::SecSelection => "Seleção",
        Msg::SecIcons => "Ícones",
        Msg::SecType => "Tipografia",
        Msg::SecColors => "Cores",
        Msg::SecBrand => "Marca",
        Msg::Home => "Início",
        Msg::Style => "Estilo",
        Msg::Settings => "Ajustes",
        Msg::CaptureArea => "Capturar área",
        Msg::FullScreen => "Tela inteira",
        Msg::Disabled => "Desativado",
        Msg::Margin => "Margem",
        Msg::ShowNotice => "Mostrar aviso ao copiar",
        Msg::StartWithSystem => "Iniciar com o sistema",
        Msg::Recent => "Capturas recentes",
        Msg::About => "Sobre o Takeshot",
        Msg::Quit => "Sair",
        Msg::TypeTitle => "Título de página",
        Msg::TypeSection => "Título de seção",
        Msg::TypeBody => "Corpo de texto e campos",
        Msg::TypeLabel => "Rótulo e botão",
        Msg::TypeCaption => "Legenda e metadados",
        Msg::DiscardTitle => "Descartar as mudanças?",
        Msg::DiscardBody => "Esta imagem tem alterações que não foram copiadas nem salvas.",
        Msg::Discard => "Descartar",
        Msg::CopyAndClose => "Copiar e fechar",
        Msg::Cancel => "Cancelar",
        Msg::CaptureIn3s => "Capturar em 3 segundos",
        Msg::OpenTakeshot => "Abrir o Takeshot",
        Msg::StyleAndWatermark => "Estilo e marca d'água",
        Msg::OpenCapturesFolder => "Abrir pasta das capturas",
        Msg::Today => "Hoje",
        Msg::Yesterday => "Ontem",
        Msg::RecentHint => "Clique copia · Shift+clique abre",
        Msg::CloseAppTitle => "Fechar o Takeshot?",
        Msg::CloseAppBody => {
            "Minimizado, ele continua na bandeja e o PrtSc segue funcionando. Para sair de vez, use Sair no menu da bandeja."
        }
        Msg::RememberChoice => "Lembrar minha escolha",
        Msg::MinimizeToTray => "Minimizar para a bandeja",
        Msg::OnCloseLabel => "Ao clicar no X",
        Msg::TrayClickLabel => "Clique no ícone da bandeja",
        Msg::OpenHome => "Abrir o Início",
        Msg::QuitTakeshot => "Sair do Takeshot",
        Msg::GeneralSection => "Geral",
        Msg::PrtScPriority => "Usar prioridade do Takeshot",
        Msg::PrtScPriorityHint => "Alguns antivírus podem estranhar.",
        Msg::OpenWindowsSettings => "Abrir configuração do Windows",
        Msg::PrtScConflictHelp => {
            "Desligue \"Usar a tecla Print Screen para abrir a captura de tela\" em Configurações > Acessibilidade > Teclado."
        }
        Msg::ChangeKeys => "Alterar",
        Msg::RestoreDefault => "Restaurar padrão",
        Msg::PressKeys => "Pressione as novas teclas. Esc cancela.",
        Msg::WaylandHotkeyHelp => "No Wayland, crie um atalho no desktop que rode: {0}",
        Msg::AboutPhrase => {
            "Captura, anotação e acabamento de telas. Tudo local, sem nuvem, PNG sem perdas na resolução original."
        }
        Msg::MadeBy => "Feito por TakeFlow",
        Msg::LicenseAgpl => "Licença AGPL-3.0",
        Msg::ViewOnGitHub => "Ver no GitHub",
        Msg::VersionN => "Versão {0}",
        Msg::CloseWord => "Fechar",
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
