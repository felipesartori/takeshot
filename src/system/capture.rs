//! Grabbing the screen and the usable area of a monitor, plus a made-up desktop picture used for demos and checks.

use image::RgbaImage;
use tiny_skia as sk;

pub struct Shot {
    pub image: RgbaImage,
    /// Monitor position and scale, used to place the overlay window.
    pub x: i32,
    pub y: i32,
    pub scale: f32,
}

fn cursor() -> Option<(i32, i32)> {
    match mouse_position::mouse_position::Mouse::get_mouse_position() {
        mouse_position::mouse_position::Mouse::Position { x, y } => Some((x, y)),
        mouse_position::mouse_position::Mouse::Error => None,
    }
}

/// Capture the monitor under the mouse (or the primary one).
pub fn grab() -> Result<Shot, String> {
    let monitors = xcap::Monitor::all().map_err(|e| e.to_string())?;
    let mon = cursor_monitor(&monitors)
        .or_else(|| monitors.into_iter().next())
        .ok_or_else(|| "nenhum monitor encontrado".to_string())?;
    shot_of(&mon)
}

/// Capture every monitor, the one under the mouse (or the primary one) first.
pub fn grab_all() -> Result<Vec<Shot>, String> {
    let mut monitors = xcap::Monitor::all().map_err(|e| e.to_string())?;
    if let Some(first) = cursor_monitor(&monitors)
        .and_then(|m| monitors.iter().position(|o| o.id().ok() == m.id().ok()))
    {
        monitors.swap(0, first);
    }
    if monitors.is_empty() {
        return Err("nenhum monitor encontrado".to_string());
    }
    monitors.iter().map(shot_of).collect()
}

/// The monitor under the mouse, else the primary one.
fn cursor_monitor(monitors: &[xcap::Monitor]) -> Option<xcap::Monitor> {
    cursor()
        .and_then(|(x, y)| xcap::Monitor::from_point(x, y).ok())
        .or_else(|| {
            monitors
                .iter()
                .find(|m| m.is_primary().unwrap_or(false))
                .cloned()
        })
}

fn shot_of(mon: &xcap::Monitor) -> Result<Shot, String> {
    let image = mon.capture_image().map_err(|e| e.to_string())?;
    Ok(Shot {
        image,
        x: mon.x().unwrap_or(0),
        y: mon.y().unwrap_or(0),
        scale: mon.scale_factor().unwrap_or(1.0).max(0.5),
    })
}

fn fill(pm: &mut sk::Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, c: [u8; 4]) {
    let mut p = sk::Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], c[3]);
    p.anti_alias = true;
    if let Some(path) = crate::engine::render::rounded_rect(x, y, w, h, [r; 4]) {
        pm.fill_path(
            &path,
            &p,
            sk::FillRule::Winding,
            sk::Transform::identity(),
            None,
        );
    }
}

/// A neutral, made-up app window (no real brands), for previews and the self-check.
pub fn sample_window(w: u32, h: u32) -> RgbaImage {
    let mut pm = sk::Pixmap::new(w, h).unwrap();
    let (wf, hf) = (w as f32, h as f32);
    let u = wf / 1000.0;
    fill(&mut pm, 0.0, 0.0, wf, hf, 0.0, [250, 250, 251, 255]);
    // sidebar
    fill(&mut pm, 0.0, 0.0, 210.0 * u, hf, 0.0, [242, 242, 245, 255]);
    for i in 0..7 {
        let y = 70.0 * u + i as f32 * 42.0 * u;
        let c = if i == 1 {
            [255, 228, 214, 255]
        } else {
            [0, 0, 0, 0]
        };
        fill(
            &mut pm,
            14.0 * u,
            y - 10.0 * u,
            182.0 * u,
            34.0 * u,
            8.0 * u,
            c,
        );
        fill(
            &mut pm,
            28.0 * u,
            y,
            16.0 * u,
            14.0 * u,
            4.0 * u,
            [190, 192, 200, 255],
        );
        fill(
            &mut pm,
            54.0 * u,
            y + 2.0 * u,
            (70.0 + (i * 23 % 60) as f32) * u,
            10.0 * u,
            5.0 * u,
            [170, 172, 180, 255],
        );
    }
    // header
    fill(
        &mut pm,
        240.0 * u,
        34.0 * u,
        260.0 * u,
        22.0 * u,
        6.0 * u,
        [40, 42, 48, 255],
    );
    fill(
        &mut pm,
        240.0 * u,
        68.0 * u,
        380.0 * u,
        12.0 * u,
        6.0 * u,
        [180, 182, 190, 255],
    );
    fill(
        &mut pm,
        wf - 170.0 * u,
        34.0 * u,
        130.0 * u,
        34.0 * u,
        9.0 * u,
        [255, 94, 58, 255],
    );
    // cards
    let cards = [[255, 94, 58, 255], [63, 160, 201, 255], [86, 171, 122, 255]];
    for (i, c) in cards.iter().enumerate() {
        let x = 240.0 * u + i as f32 * 245.0 * u;
        fill(
            &mut pm,
            x,
            110.0 * u,
            225.0 * u,
            120.0 * u,
            12.0 * u,
            [255, 255, 255, 255],
        );
        fill(
            &mut pm,
            x + 18.0 * u,
            130.0 * u,
            70.0 * u,
            10.0 * u,
            5.0 * u,
            [175, 177, 186, 255],
        );
        fill(
            &mut pm,
            x + 18.0 * u,
            156.0 * u,
            120.0 * u,
            26.0 * u,
            6.0 * u,
            [36, 38, 44, 255],
        );
        fill(
            &mut pm,
            x + 18.0 * u,
            198.0 * u,
            160.0 * u,
            8.0 * u,
            4.0 * u,
            [235, 236, 240, 255],
        );
        fill(
            &mut pm,
            x + 18.0 * u,
            198.0 * u,
            (60.0 + i as f32 * 40.0) * u,
            8.0 * u,
            4.0 * u,
            *c,
        );
    }
    // chart
    let top = 255.0 * u;
    fill(
        &mut pm,
        240.0 * u,
        top,
        735.0 * u,
        hf - top - 24.0 * u,
        12.0 * u,
        [255, 255, 255, 255],
    );
    let bars = [
        0.45, 0.62, 0.38, 0.8, 0.55, 0.92, 0.7, 0.5, 0.66, 0.84, 0.58, 0.74,
    ];
    let base = hf - 50.0 * u;
    let avail = base - top - 60.0 * u;
    for (i, b) in bars.iter().enumerate() {
        let bh = avail * b;
        let c = if i == 5 {
            [255, 94, 58, 255]
        } else {
            [214, 216, 224, 255]
        };
        fill(
            &mut pm,
            270.0 * u + i as f32 * 58.0 * u,
            base - bh,
            34.0 * u,
            bh,
            6.0 * u,
            c,
        );
    }
    crate::engine::render::from_pixmap(&pm)
}

/// A fake desktop: wallpaper with the sample window on it.
pub fn sample_desktop(w: u32, h: u32) -> RgbaImage {
    let mut pm = sk::Pixmap::new(w, h).unwrap();
    let stops = vec![
        sk::GradientStop::new(0.0, sk::Color::from_rgba8(28, 44, 70, 255)),
        sk::GradientStop::new(1.0, sk::Color::from_rgba8(92, 120, 140, 255)),
    ];
    if let Some(sh) = sk::LinearGradient::new(
        sk::Point::from_xy(0.0, 0.0),
        sk::Point::from_xy(w as f32, h as f32),
        stops,
        sk::SpreadMode::Pad,
        sk::Transform::identity(),
    ) {
        let p = sk::Paint {
            shader: sh,
            ..Default::default()
        };
        pm.fill_rect(
            sk::Rect::from_xywh(0.0, 0.0, w as f32, h as f32).unwrap(),
            &p,
            sk::Transform::identity(),
            None,
        );
    }
    let win = sample_window((w as f32 * 0.62) as u32, (h as f32 * 0.6) as u32);
    let wpm = crate::engine::render::to_pixmap(&win);
    pm.draw_pixmap(
        (w as f32 * 0.19) as i32,
        (h as f32 * 0.17) as i32,
        wpm.as_ref(),
        &sk::PixmapPaint::default(),
        sk::Transform::identity(),
        None,
    );
    // taskbar
    fill(
        &mut pm,
        0.0,
        h as f32 - 44.0,
        w as f32,
        44.0,
        0.0,
        [18, 20, 26, 235],
    );
    crate::engine::render::from_pixmap(&pm)
}

pub fn sample_shot() -> Shot {
    Shot {
        image: sample_desktop(1600, 900),
        x: 0,
        y: 0,
        scale: 1.0,
    }
}

/// The area without the taskbar of the monitor nearest to `p`, in physical pixels: the tray menus
/// and the notice stay inside it. `None` where the system does not say (callers use the whole monitor).
#[cfg(windows)]
pub fn work_area(p: egui::Pos2) -> Option<egui::Rect> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    };
    let pt = POINT {
        x: p.x as i32,
        y: p.y as i32,
    };
    // SAFETY: plain Win32 queries; MONITORINFO is zeroed and its size set as the API requires.
    unsafe {
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return None;
        }
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }
        let r = info.rcWork;
        Some(egui::Rect::from_min_max(
            egui::pos2(r.left as f32, r.top as f32),
            egui::pos2(r.right as f32, r.bottom as f32),
        ))
    }
}

#[cfg(not(windows))]
pub fn work_area(_p: egui::Pos2) -> Option<egui::Rect> {
    None
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn the_work_area_is_part_of_the_nearest_monitor() {
        use super::work_area;
        // Any point, even far off screen, gets the nearest monitor and its area without the taskbar.
        for p in [egui::pos2(0.0, 0.0), egui::pos2(-10000.0, -10000.0)] {
            let r = work_area(p).expect("a monitor");
            assert!(r.width() > 0.0 && r.height() > 0.0, "{r:?}");
        }
    }

    /// Needs a real display: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn grabs_the_screen() {
        let s = super::grab().expect("capture");
        assert!(s.image.width() > 100 && s.image.height() > 100);
        crate::system::actions::save_png(&s.image, std::path::Path::new("shots/real-capture.png"))
            .unwrap();
    }
}
