//! Renders the brand SVG masters in assets/brand/ into PNGs, takeshot.ico, Takeshot.icns and the
//! Windows installer's pictures (packaging/windows/*.bmp) and the macOS dmg window's background
//! (packaging/macos/dmg-background*.png). Run with `cargo run --example brand` after changing any
//! SVG; the outputs are committed.

use std::fs;
use std::path::Path;

const DIR: &str = "assets/brand";
const APP_SIZES: &[u32] = &[16, 20, 24, 32, 40, 48, 64, 128, 256, 512, 1024];
const MARK_SIZES: &[u32] = &[64, 128, 256, 512];
const TRAY_SIZES: &[u32] = &[16, 20, 24, 32, 40, 48, 64];
const ICO_SIZES: &[u32] = &[16, 20, 24, 32, 40, 48, 64, 256];
const ICNS_SIZES: &[u32] = &[16, 32, 64, 128, 256, 512, 1024];

fn render(name: &str, size: u32) -> Vec<u8> {
    render_size(name, size, size)
}

/// `name`'s SVG at `w` x `h` (scaled by the width), straight-alpha RGBA. Text is set in the
/// app's own fonts (Quicksand, Roboto), so the wordmark and captions match the app on any machine.
fn render_size(name: &str, w: u32, h: u32) -> Vec<u8> {
    let svg = fs::read(Path::new(DIR).join(format!("{name}.svg"))).expect("read svg");
    let mut opts = resvg::usvg::Options::default();
    for font in [
        "Quicksand-Medium.ttf",
        "Roboto-Regular.ttf",
        "Roboto-Medium.ttf",
    ] {
        opts.fontdb_mut()
            .load_font_file(Path::new("assets/fonts").join(font))
            .expect("load font");
    }
    let tree = resvg::usvg::Tree::from_data(&svg, &opts).expect("parse svg");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).expect("pixmap");
    let scale = w as f32 / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia stores premultiplied alpha; PNG, ICO and ICNS expect straight alpha.
    pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

/// The installer's pictures: 24-bit BMPs, as NSIS shows them (no alpha), at the size the
/// installer's window has at 100 %.
fn save_bmp(name: &str, w: u32, h: u32) {
    let rgba = render_size(name, w, h);
    let rgb: Vec<u8> = rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    let img = image::RgbImage::from_raw(w, h, rgb).expect("buffer size");
    let out = Path::new("packaging/windows").join(format!("{name}.bmp"));
    img.save(&out).expect("write bmp");
}

/// The dmg window's background at 1x and 2x (CI joins them into one TIFF for Retina screens).
fn save_dmg_background() {
    for (scale, suffix) in [(1, ""), (2, "@2x")] {
        let (w, h) = (660 * scale, 400 * scale);
        let img = image::RgbaImage::from_raw(w, h, render_size("dmg-background", w, h))
            .expect("buffer size");
        img.save(Path::new("packaging/macos").join(format!("dmg-background{suffix}.png")))
            .expect("write png");
    }
}

fn save_png(name: &str, size: u32) {
    let rgba = render(name, size);
    let img = image::RgbaImage::from_raw(size, size, rgba).expect("buffer size");
    let out = Path::new(DIR)
        .join("png")
        .join(format!("{name}-{size}.png"));
    img.save(&out).expect("write png");
}

fn main() {
    fs::create_dir_all(Path::new(DIR).join("png")).expect("png dir");
    for app in ["takeui-app", "coral-app"] {
        for &s in APP_SIZES {
            save_png(app, s);
        }
    }
    for mark in ["takeui-mark-light", "takeui-mark-dark", "coral-mark"] {
        for &s in MARK_SIZES {
            save_png(mark, s);
        }
    }
    for tray in [
        "takeui-tray-dark",
        "takeui-tray-light",
        "takeui-tray-template",
        "coral-tray-dark",
        "coral-tray-light",
        "coral-tray-template",
    ] {
        for &s in TRAY_SIZES {
            save_png(tray, s);
        }
    }

    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for &s in ICO_SIZES {
        let img = ico::IconImage::from_rgba_data(s, s, render("takeui-app", s));
        dir.add_entry(ico::IconDirEntry::encode(&img).expect("encode ico entry"));
    }
    dir.write(fs::File::create(Path::new(DIR).join("takeshot.ico")).expect("create ico"))
        .expect("write ico");

    let mut family = icns::IconFamily::new();
    for &s in ICNS_SIZES {
        let img = icns::Image::from_data(icns::PixelFormat::RGBA, s, s, render("takeui-app", s))
            .expect("icns image");
        family.add_icon(&img).expect("add icns icon");
    }
    family
        .write(fs::File::create(Path::new(DIR).join("Takeshot.icns")).expect("create icns"))
        .expect("write icns");
    save_bmp("installer-sidebar", 164, 314);
    save_bmp("installer-header", 150, 57);
    save_dmg_background();
    println!("brand assets written to {DIR}, packaging/windows and packaging/macos");
}
