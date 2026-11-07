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
        if let Some(i) = self.ui {
            let key = format!("tk-{}", self.list[i].name);
            if let Some(p) = defs.families.get_mut(&egui::FontFamily::Proportional) {
                p.insert(0, key);
            }
        }
        let bold_key = format!("tk-{}", self.list[self.bold].name);
        let mut b = vec![bold_key];
        b.extend(
            defs.families
                .get(&egui::FontFamily::Proportional)
                .cloned()
                .unwrap_or_default(),
        );
        defs.families
            .insert(egui::FontFamily::Name("bold".into()), b);
        ctx.set_fonts(defs);
    }
}

/// ab_glyph scale for a font size in pixels (the font's full height, as egui sizes fonts).
pub fn px_scale(_font: &FontArc, size: f32) -> PxScale {
    PxScale::from(size)
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
