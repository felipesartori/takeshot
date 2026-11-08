//! Renders the brand SVG masters in assets/brand/ into PNGs, takeshot.ico and Takeshot.icns.
//! Run with `cargo run --example brand` after changing any SVG; the outputs are committed.

use std::fs;
use std::path::Path;

const DIR: &str = "assets/brand";
const APP_SIZES: &[u32] = &[16, 20, 24, 32, 40, 48, 64, 128, 256, 512, 1024];
const MARK_SIZES: &[u32] = &[64, 128, 256, 512];
const TRAY_SIZES: &[u32] = &[16, 20, 24, 32, 40, 48, 64];
const ICO_SIZES: &[u32] = &[16, 20, 24, 32, 40, 48, 64, 256];
const ICNS_SIZES: &[u32] = &[16, 32, 64, 128, 256, 512, 1024];

fn render(name: &str, size: u32) -> Vec<u8> {
    let svg = fs::read(Path::new(DIR).join(format!("{name}.svg"))).expect("read svg");
    let tree =
        resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default()).expect("parse svg");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("pixmap");
    let scale = size as f32 / tree.size().width();
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
    println!("brand assets written to {DIR}");
}
