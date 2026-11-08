//! Fonts for the text tool: a few distinct system fonts when present, plus two built in.

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct FontEntry {
    pub name: String,
    pub family: egui::FontFamily,
    pub font: FontArc,
    pub bytes: Arc<Vec<u8>>,
}

pub struct FontLib {
    pub list: Vec<FontEntry>,
    /// Index of a bold sans font (counters, labels).
    pub bold: usize,
    /// System sans used for the interface, if found.
    pub ui: Option<usize>,
}

const CANDIDATES: &[(&str, &[&str])] = &[
    (
        "Sans",
        &[
            "segoeui.ttf",
            "sfns.ttf",
            "inter-regular.ttf",
            "notosans-regular.ttf",
            "dejavusans.ttf",
            "ubuntu-r.ttf",
            "liberationsans-regular.ttf",
        ],
    ),
    (
        "Sans Negrito",
        &[
            "segoeuib.ttf",
            "arialbd.ttf",
            "arial bold.ttf",
            "notosans-bold.ttf",
            "dejavusans-bold.ttf",
            "liberationsans-bold.ttf",
            "ubuntu-b.ttf",
        ],
    ),
    (
        "Arial",
        &[
            "arial.ttf",
            "liberationsans-regular.ttf",
            "arimo-regular.ttf",
        ],
    ),
    (
        "Serifada",
        &[
            "georgia.ttf",
            "notoserif-regular.ttf",
            "dejavuserif.ttf",
            "liberationserif-regular.ttf",
            "times.ttf",
            "times new roman.ttf",
        ],
    ),
    (
        "Elegante",
        &[
            "pala.ttf",
            "palatino linotype.ttf",
            "constan.ttf",
            "gabriola.ttf",
            "didot.ttf",
            "notoserif-italic.ttf",
            "dejavuserif-italic.ttf",
        ],
    ),
    (
        "Mono",
        &[
            "consola.ttf",
            "cascadiamono.ttf",
            "courier new.ttf",
            "cour.ttf",
            "dejavusansmono.ttf",
            "liberationmono-regular.ttf",
            "notosansmono-regular.ttf",
        ],
    ),
    (
        "Impacto",
        &[
            "impact.ttf",
            "anton-regular.ttf",
            "bebasneue-regular.ttf",
            "dejavusans-bold.ttf",
        ],
    ),
    (
        "Condensada",
        &[
            "bahnschrift.ttf",
            "arial narrow.ttf",
            "arialn.ttf",
            "dejavusanscondensed.ttf",
            "notosans-condensed.ttf",
        ],
    ),
    (
        "Manuscrita",
        &[
            "inkfree.ttf",
            "segoepr.ttf",
            "comic.ttf",
            "comic sans ms.ttf",
            "chalkduster.ttf",
            "bradley hand bold.ttf",
            "caveat-regular.ttf",
            "notosans-italic.ttf",
        ],
    ),
];

fn font_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if cfg!(target_os = "windows") {
        let win = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        v.push(Path::new(&win).join("Fonts"));
        if let Some(local) = dirs::data_local_dir() {
            v.push(local.join("Microsoft").join("Windows").join("Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        v.push("/System/Library/Fonts".into());
        v.push("/System/Library/Fonts/Supplemental".into());
        v.push("/Library/Fonts".into());
        if let Some(h) = dirs::home_dir() {
            v.push(h.join("Library/Fonts"));
        }
    } else {
        v.push("/usr/share/fonts".into());
        v.push("/usr/local/share/fonts".into());
        if let Some(h) = dirs::home_dir() {
            v.push(h.join(".local/share/fonts"));
            v.push(h.join(".fonts"));
        }
    }
    v
}

fn index_dir(dir: &Path, depth: u32, out: &mut HashMap<String, PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth > 0 {
                index_dir(&p, depth - 1, out);
            }
        } else if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            if lower.ends_with(".ttf") || lower.ends_with(".otf") {
                out.entry(lower).or_insert(p);
            }
        }
    }
}

impl FontLib {
    pub fn load() -> Self {
        let mut files = HashMap::new();
        for d in font_dirs() {
            index_dir(&d, 4, &mut files);
        }
        let mut list: Vec<FontEntry> = Vec::new();
        let mut used = std::collections::HashSet::new();
        for (name, cands) in CANDIDATES {
            for c in *cands {
                let Some(path) = files.get(*c) else { continue };
                if used.contains(path) {
                    continue;
                }
                let Ok(bytes) = std::fs::read(path) else {
                    continue;
                };
                if let Ok(font) = FontArc::try_from_vec(bytes.clone()) {
                    used.insert(path.clone());
                    list.push(FontEntry {
                        name: name.to_string(),
                        family: egui::FontFamily::Name(format!("tk-{name}").into()),
                        font,
                        bytes: Arc::new(bytes),
                    });
                    break;
                }
            }
        }
        for (name, data) in [
            ("Ubuntu", epaint_default_fonts::UBUNTU_LIGHT),
            ("Hack", epaint_default_fonts::HACK_REGULAR),
        ] {
            if let Ok(font) = FontArc::try_from_slice(data) {
                list.push(FontEntry {
                    name: name.to_string(),
                    family: egui::FontFamily::Name(format!("tk-{name}").into()),
                    font,
                    bytes: Arc::new(data.to_vec()),
                });
            }
        }
        let ui = list.iter().position(|f| f.name == "Sans");
        let bold = list
            .iter()
            .position(|f| f.name == "Sans Negrito")
            .or(list.iter().position(|f| f.name == "Impacto"))
            .or(ui)
            .unwrap_or(0);
        Self { list, bold, ui }
    }

    pub fn get(&self, i: usize) -> &FontEntry {
        &self.list[i.min(self.list.len() - 1)]
    }

    /// Register every font with egui so previews use the same faces.
    pub fn install(&self, ctx: &egui::Context) {
        let mut defs = egui::FontDefinitions::default();
        for f in &self.list {
            let key = format!("tk-{}", f.name);
            defs.font_data.insert(
                key.clone(),
                Arc::new(egui::FontData::from_owned((*f.bytes).clone())),
            );
            let mut fam = vec![key.clone()];
            fam.extend(
                defs.families
                    .get(&egui::FontFamily::Proportional)
                    .cloned()
                    .unwrap_or_default(),
            );
            defs.families.insert(f.family.clone(), fam);
        }
        // The system sans stays right behind Roboto as a fallback for characters Roboto lacks.
        let mut fallback = defs
            .families
            .get(&egui::FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();
        if let Some(i) = self.ui {
            fallback.insert(0, format!("tk-{}", self.list[i].name));
        }
        use crate::ui::assets;
        for (key, bytes) in [
            (UI_REGULAR, assets::ROBOTO_REGULAR),
            (UI_MEDIUM, assets::ROBOTO_MEDIUM),
            (UI_BOLD, assets::ROBOTO_BOLD),
            (UI_MONO, assets::ROBOTO_MONO_MEDIUM),
            (UI_ICONS, assets::LUCIDE),
            (UI_BRAND, assets::QUICKSAND_MEDIUM),
        ] {
            defs.font_data.insert(
                key.to_string(),
                Arc::new(egui::FontData::from_static(bytes)),
            );
        }
        let with_fallback = |first: &[&str]| -> Vec<String> {
            let mut v: Vec<String> = first.iter().map(|s| s.to_string()).collect();
            v.extend(fallback.iter().cloned());
            v
        };
        let system_bold = format!("tk-{}", self.list[self.bold].name);
        let fams = [
            (egui::FontFamily::Proportional, with_fallback(&[UI_REGULAR])),
            (egui::FontFamily::Monospace, with_fallback(&[UI_MONO])),
            (
                egui::FontFamily::Name("medium".into()),
                with_fallback(&[UI_MEDIUM]),
            ),
            (
                egui::FontFamily::Name("bold".into()),
                with_fallback(&[UI_BOLD, &system_bold]),
            ),
            (
                egui::FontFamily::Name("mono".into()),
                with_fallback(&[UI_MONO]),
            ),
            (
                egui::FontFamily::Name("icons".into()),
                vec![UI_ICONS.to_string()],
            ),
            (
                egui::FontFamily::Name("brand".into()),
                with_fallback(&[UI_BRAND]),
            ),
        ];
        for (fam, list) in fams {
            defs.families.insert(fam, list);
        }
        ctx.set_fonts(defs);
    }
}

/// Font keys of the interface faces registered by `FontLib::install`.
pub const UI_REGULAR: &str = "ui-regular";
const UI_MEDIUM: &str = "ui-medium";
const UI_BOLD: &str = "ui-bold";
const UI_MONO: &str = "ui-mono";
const UI_ICONS: &str = "ui-icons";
const UI_BRAND: &str = "ui-brand";

/// ab_glyph scale for a font size in pixels. The size is the em, as egui (epaint 0.37) sizes
/// fonts; ab_glyph's own scale is the full line height, so it is converted, or the picture's text
/// comes out smaller than the text drawn live while it is typed or dragged.
pub fn px_scale(font: &FontArc, size: f32) -> PxScale {
    let em = font.units_per_em().unwrap_or(1000.0);
    PxScale::from(size * font.height_unscaled() / em)
}

/// The em size whose full line height is `height`, for text sized by its height (step numbers,
/// the watermark), so that `px_scale` gives it that height.
pub fn em_for_height(font: &FontArc, height: f32) -> f32 {
    height * font.units_per_em().unwrap_or(1000.0) / font.height_unscaled()
}

/// Width and height of (possibly multi-line) text.
pub fn measure(font: &FontArc, size: f32, text: &str) -> (f32, f32) {
    let sf = font.as_scaled(px_scale(font, size));
    let line_h = sf.height() + sf.line_gap();
    let mut w: f32 = 0.0;
    let mut lines = 0;
    for line in text.split('\n') {
        lines += 1;
        let mut x = 0.0;
        let mut prev = None;
        for ch in line.chars() {
            let id = sf.glyph_id(ch);
            if let Some(p) = prev {
                x += sf.kern(p, id);
            }
            x += sf.h_advance(id);
            prev = Some(id);
        }
        w = w.max(x);
    }
    (w, sf.height() + line_h * (lines.max(1) - 1) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_and_final_text_have_the_same_size() {
        // Text is painted by egui while it is typed or dragged and by ab_glyph in the picture;
        // the two must agree, or the text jumps in size when it lands.
        let lib = FontLib::load();
        let ctx = egui::Context::default();
        lib.install(&ctx);
        for f in &lib.list {
            let mut live = 0.0;
            for _ in 0..2 {
                let mut out = ctx.run_ui(Default::default(), |ui| {
                    let g = ui.painter().layout_no_wrap(
                        "Pico em outubro".into(),
                        egui::FontId::new(40.0, f.family.clone()),
                        egui::Color32::WHITE,
                    );
                    live = g.size().x;
                });
                out.textures_delta.clear();
            }
            let (w, _) = measure(&f.font, 40.0, "Pico em outubro");
            assert!(
                (w - live).abs() / live < 0.02,
                "{}: picture {w} vs live {live}",
                f.name
            );
        }
    }

    #[test]
    fn installs_ui_families() {
        let ctx = egui::Context::default();
        FontLib::load().install(&ctx);
        let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
        out.textures_delta.clear();
        let defs = ctx.fonts(|f| f.definitions().clone());
        assert_eq!(
            defs.families[&egui::FontFamily::Proportional]
                .first()
                .map(String::as_str),
            Some(UI_REGULAR)
        );
        let lib = FontLib::load();
        let bold = &defs.families[&egui::FontFamily::Name("bold".into())];
        assert_eq!(bold[0], "ui-bold");
        assert_eq!(bold[1], format!("tk-{}", lib.list[lib.bold].name));
        if let Some(i) = lib.ui {
            assert_eq!(
                defs.families[&egui::FontFamily::Proportional][1],
                format!("tk-{}", lib.list[i].name)
            );
        }
        for f in &lib.list {
            assert!(
                defs.families.contains_key(&f.family),
                "fonte do texto {} sumiu",
                f.name
            );
        }
        for fam in ["medium", "bold", "mono", "icons", "brand"] {
            assert!(
                defs.families
                    .contains_key(&egui::FontFamily::Name(fam.into())),
                "família {fam} ausente"
            );
        }
    }
}
