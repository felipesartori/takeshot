//! Copy, save, open-in-a-new-window: the things every screen can do with a finished picture.

use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread::JoinHandle;

static CLIPBOARD_KEEPERS: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());

pub fn copy_image(img: &RgbaImage) -> Result<(), String> {
    let data = arboard::ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: std::borrow::Cow::Owned(img.as_raw().clone()),
    };
    #[cfg(target_os = "linux")]
    {
        // On Linux the program that copied must keep serving the picture, so a helper thread
        // holds it until another program takes the clipboard.
        use arboard::SetExtLinux;
        let (tx, rx) = std::sync::mpsc::channel();
        let h = std::thread::spawn(move || match arboard::Clipboard::new() {
            Ok(mut cb) => {
                let _ = tx.send(Ok(()));
                let _ = cb.set().wait().image(data);
            }
            Err(e) => {
                let _ = tx.send(Err(e.to_string()));
            }
        });
        CLIPBOARD_KEEPERS.lock().unwrap().push(h);
        rx.recv()
            .unwrap_or(Err("área de transferência indisponível".into()))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        cb.set_image(data).map_err(|e| e.to_string())
    }
}

pub fn copy_text(t: &str) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(t.to_string());
    }
}

pub fn wait_clipboard_keepers() {
    let hs: Vec<_> = CLIPBOARD_KEEPERS.lock().unwrap().drain(..).collect();
    for h in hs {
        let _ = h.join();
    }
}

pub fn clipboard_image() -> Option<RgbaImage> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let img = cb.get_image().ok()?;
    RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned())
}

/// Lossless PNG (no quality loss, full resolution).
pub fn save_png(img: &RgbaImage, path: &Path) -> Result<(), String> {
    use image::ImageEncoder;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let w = std::io::BufWriter::new(f);
    PngEncoder::new_with_quality(w, CompressionType::Default, FilterType::Adaptive)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())
}

pub fn default_name() -> String {
    chrono::Local::now()
        .format("Takeshot %Y-%m-%d %H-%M-%S.png")
        .to_string()
}

pub fn auto_path(dir: &str) -> PathBuf {
    let mut p = PathBuf::from(dir).join(default_name());
    let mut n = 2;
    while p.exists() {
        p = PathBuf::from(dir).join(format!(
            "{} ({n}).png",
            default_name().trim_end_matches(".png")
        ));
        n += 1;
    }
    p
}

pub fn ask_save_path(dir: &str, parent: Option<&eframe::Frame>) -> Option<PathBuf> {
    let _ = std::fs::create_dir_all(dir);
    let mut d = rfd::FileDialog::new()
        .set_title("Salvar captura")
        .set_directory(dir)
        .set_file_name(default_name())
        .add_filter("PNG (sem perdas)", &["png"]);
    if let Some(f) = parent {
        d = d.set_parent(f);
    }
    d.save_file().map(|p| {
        if p.extension().is_none() {
            p.with_extension("png")
        } else {
            p
        }
    })
}

pub fn temp_png(img: &RgbaImage) -> Option<PathBuf> {
    let p = std::env::temp_dir().join(format!(
        "takeshot-{}.png",
        chrono::Local::now().format("%Y%m%d%H%M%S%3f")
    ));
    save_png(img, &p).ok().map(|_| p)
}

/// Opens the file's folder with the file selected (Windows, macOS) or the folder (Linux).
pub fn show_in_folder(path: &Path) {
    reveal(path);
}

#[cfg(windows)]
fn reveal(path: &Path) {
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
}

#[cfg(target_os = "macos")]
fn reveal(path: &Path) {
    let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn();
}

#[cfg(all(unix, not(target_os = "macos")))]
fn reveal(path: &Path) {
    if let Some(dir) = path.parent() {
        open_folder(dir);
    }
}

/// Starts another Takeshot process (capture, editor, pin) with `args`; the native title bar choice
/// goes along (the environment variable is inherited anyway, the flag is passed on).
pub fn spawn_self(args: &[&str]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args);
    if crate::ui::chrome::native() {
        cmd.arg("--native-titlebar");
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

pub fn open_folder(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(all(unix, not(target_os = "macos")))]
    let cmd = "xdg-open";
    let _ = std::process::Command::new(cmd).arg(path).spawn();
}
