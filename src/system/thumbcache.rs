//! Thumbnails kept on disk, so a capture is decoded once instead of on every visit to Início: 320 px
//! PNGs in the system's cache folder, named after the capture's path, size and date (a changed file
//! gets a new one), the oldest dropped past a cap. Without the folder everything still works: the
//! capture is decoded again.

use image::RgbaImage;
use std::path::{Path, PathBuf};

/// How many thumbnails the cache keeps (about 60 KB each).
pub const CAP: usize = 4000;

/// The cache folder, when the system has one.
pub fn dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("takeshot").join("thumbs"))
}

/// The cache file for `path` as it is now: its path, size and date in a stable hash (FNV-1a), so
/// a file changed under the same name gets another; none when the file cannot be read.
fn file_for(dir: &Path, path: &Path) -> Option<PathBuf> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let bytes = path.as_os_str().as_encoded_bytes().iter();
    for b in bytes
        .chain(&meta.len().to_le_bytes())
        .chain(&modified.to_le_bytes())
    {
        h = (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
    }
    Some(dir.join(format!("{h:016x}.png")))
}

/// The thumbnail cached for `path`, when there is one and it reads.
pub fn load(dir: &Path, path: &Path) -> Option<RgbaImage> {
    let f = file_for(dir, path)?;
    image::open(f).ok().map(|i| i.to_rgba8())
}

/// Keeps `thumb` as `path`'s thumbnail: a temporary file of this process's own, then a rename, so
/// a reader never sees half of it. Failing to write only means decoding again next time.
pub fn store(dir: &Path, path: &Path, thumb: &RgbaImage) {
    use image::ImageEncoder;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let Some(f) = file_for(dir, path) else {
        return;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let mut png = Vec::new();
    let written =
        PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Adaptive)
            .write_image(
                thumb.as_raw(),
                thumb.width(),
                thumb.height(),
                image::ExtendedColorType::Rgba8,
            );
    let tmp = f.with_extension(format!("{}.tmp", std::process::id()));
    if written.is_ok() && std::fs::write(&tmp, png).is_ok() && std::fs::rename(&tmp, &f).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// Drops the oldest thumbnails past `cap`.
pub fn prune(dir: &Path, cap: usize) {
    let Ok(list) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = list
        .filter_map(|f| {
            let f = f.ok()?;
            let p = f.path();
            let when = f.metadata().ok()?.modified().ok()?;
            (p.extension()? == "png").then_some((when, p))
        })
        .collect();
    if files.len() <= cap {
        return;
    }
    files.sort();
    let extra = files.len() - cap;
    for (_, p) in files.into_iter().take(extra) {
        let _ = std::fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    fn temp(test: &str) -> (PathBuf, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("takeshot-thumbcache-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (captures, cache) = (root.join("captures"), root.join("cache"));
        std::fs::create_dir_all(&captures).unwrap();
        (captures, cache)
    }

    fn capture(dir: &std::path::Path, name: &str, w: u32) -> PathBuf {
        let p = dir.join(name);
        RgbaImage::from_pixel(w, 10, Rgba([10, 20, 30, 255]))
            .save(&p)
            .unwrap();
        p
    }

    #[test]
    fn a_cached_thumb_comes_back_for_the_same_file() {
        let (captures, cache) = temp("same");
        let shot = capture(&captures, "a.png", 40);
        assert!(load(&cache, &shot).is_none(), "nothing cached yet");
        let thumb = RgbaImage::from_pixel(8, 4, Rgba([1, 2, 3, 255]));
        store(&cache, &shot, &thumb);
        assert_eq!(load(&cache, &shot), Some(thumb));
    }

    #[test]
    fn a_changed_file_gets_a_new_thumb() {
        let (captures, cache) = temp("changed");
        let shot = capture(&captures, "a.png", 40);
        store(&cache, &shot, &RgbaImage::new(8, 4));
        // The same name, another picture.
        capture(&captures, "a.png", 90);
        assert!(load(&cache, &shot).is_none());
    }

    #[test]
    fn a_broken_cache_file_is_ignored() {
        let (captures, cache) = temp("broken");
        let shot = capture(&captures, "a.png", 40);
        store(&cache, &shot, &RgbaImage::new(8, 4));
        for f in std::fs::read_dir(&cache).unwrap() {
            std::fs::write(f.unwrap().path(), b"half a png").unwrap();
        }
        assert!(load(&cache, &shot).is_none());
    }

    #[test]
    fn the_cache_keeps_alpha() {
        let (captures, cache) = temp("alpha");
        let shot = capture(&captures, "a.png", 40);
        let thumb = RgbaImage::from_pixel(8, 4, Rgba([200, 100, 50, 90]));
        store(&cache, &shot, &thumb);
        assert_eq!(
            load(&cache, &shot).unwrap().get_pixel(3, 2).0,
            [200, 100, 50, 90]
        );
    }

    #[test]
    fn the_cache_drops_the_oldest_past_its_cap() {
        let (captures, cache) = temp("cap");
        let base = SystemTime::now() - Duration::from_secs(3600);
        let mut files = Vec::new();
        for i in 0..5u64 {
            let shot = capture(&captures, &format!("{i}.png"), 40 + i as u32);
            store(&cache, &shot, &RgbaImage::new(8, 4));
            let f = std::fs::read_dir(&cache)
                .unwrap()
                .map(|f| f.unwrap().path())
                .find(|f| !files.contains(f))
                .unwrap();
            std::fs::File::options()
                .write(true)
                .open(&f)
                .unwrap()
                .set_modified(base + Duration::from_secs(i * 60))
                .unwrap();
            files.push(f);
        }
        prune(&cache, 3);
        let left: Vec<bool> = files.iter().map(|f| f.exists()).collect();
        assert_eq!(
            left,
            [false, false, true, true, true],
            "the two oldest went"
        );
    }
}
