//! Thumbnails kept on disk, so a capture is decoded once instead of on every visit to Início: one
//! file per capture in the system's cache folder, a header (the picture's size, and the capture's
//! path, size and date it was made from) and then a 320 px PNG. A cache hit never reads the capture
//! itself, only its size and date. A thumbnail goes when its capture is deleted or changed (Delete
//! in Início at once, the rest when the cache is pruned) and past a cap, the oldest first. Without
//! the folder everything still works: the capture is decoded again.

use image::RgbaImage;
use std::path::{Path, PathBuf};

/// How many thumbnails the system cache keeps.
pub const CAP: usize = 4000;

const MAGIC: &[u8; 4] = b"TKT1";

/// Where thumbnails are kept and how many.
#[derive(Clone, Debug)]
pub struct Cache {
    pub dir: PathBuf,
    pub cap: usize,
}

/// A capture as it is now: the file its thumbnail goes to, and what that thumbnail must match.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    file: PathBuf,
    source: String,
    len: u64,
    modified: u128,
}

/// What a cache file says about itself.
struct Header {
    size: [u32; 2],
    source: String,
    len: u64,
    modified: u128,
}

impl Cache {
    /// The system's cache folder (`<cache>/takeshot/thumbs`), when there is one.
    pub fn system() -> Option<Self> {
        dirs::cache_dir().map(|d| Self {
            dir: d.join("takeshot").join("thumbs"),
            cap: CAP,
        })
    }

    /// `source` as it is now (one look at its size and date, never its content); none when it
    /// cannot be read.
    pub fn entry(&self, source: &Path) -> Option<Entry> {
        let (len, modified) = stamp(source)?;
        let source = source.to_string_lossy().into_owned();
        // A stable hash (FNV-1a) of the path, size and date: a changed capture gets a new file.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in source
            .as_bytes()
            .iter()
            .chain(&len.to_le_bytes())
            .chain(&modified.to_le_bytes())
        {
            h = (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
        }
        Some(Entry {
            file: self.dir.join(format!("{h:016x}.thumb")),
            source,
            len,
            modified,
        })
    }

    /// Drops `source`'s thumbnail (a capture sent to the trash).
    pub fn forget(&self, source: &Path) {
        if let Some(e) = self.entry(source) {
            let _ = std::fs::remove_file(e.file);
        }
    }

    /// Drops the thumbnails whose capture is gone or changed, temporary files left behind, and
    /// then the oldest past the cap.
    pub fn prune(&self) {
        let Ok(list) = std::fs::read_dir(&self.dir) else {
            return;
        };
        let hour_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        let mut kept = Vec::new();
        for f in list.flatten() {
            let p = f.path();
            let Ok(when) = f.metadata().and_then(|m| m.modified()) else {
                continue;
            };
            if p.extension().is_some_and(|e| e == "tmp") {
                if when < hour_ago {
                    let _ = std::fs::remove_file(&p);
                }
                continue;
            }
            if p.extension().is_none_or(|e| e != "thumb") {
                continue;
            }
            let current = read_header(&p).is_some_and(|h| {
                stamp(Path::new(&h.source)).is_some_and(|s| s == (h.len, h.modified))
            });
            if current {
                kept.push((when, p));
            } else {
                let _ = std::fs::remove_file(&p);
            }
        }
        if kept.len() > self.cap {
            kept.sort();
            let extra = kept.len() - self.cap;
            for (_, p) in kept.into_iter().take(extra) {
                let _ = std::fs::remove_file(p);
            }
        }
    }
}

impl Entry {
    /// The thumbnail and the capture's size, when the file is there, whole, and made from this
    /// very capture.
    pub fn load(&self) -> Option<(RgbaImage, [u32; 2])> {
        let bytes = std::fs::read(&self.file).ok()?;
        let (h, png) = parse_header(&bytes)?;
        if (h.source.as_str(), h.len, h.modified) != (self.source.as_str(), self.len, self.modified)
        {
            return None;
        }
        let img = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?;
        Some((img.into_rgba8(), h.size))
    }

    /// Keeps `thumb` for this capture of `size`: a temporary file of this process and thread
    /// first, then a rename, so a reader never sees half of it. Failing only means decoding again.
    pub fn store(&self, thumb: &RgbaImage, size: [u32; 2]) {
        use image::ImageEncoder;
        use image::codecs::png::{CompressionType, FilterType, PngEncoder};
        let Some(dir) = self.file.parent() else {
            return;
        };
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
        let mut out = Vec::with_capacity(64 * 1024);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&size[0].to_le_bytes());
        out.extend_from_slice(&size[1].to_le_bytes());
        out.extend_from_slice(&self.len.to_le_bytes());
        out.extend_from_slice(&self.modified.to_le_bytes());
        out.extend_from_slice(&(self.source.len() as u32).to_le_bytes());
        out.extend_from_slice(self.source.as_bytes());
        let written =
            PngEncoder::new_with_quality(&mut out, CompressionType::Fast, FilterType::Adaptive)
                .write_image(
                    thumb.as_raw(),
                    thumb.width(),
                    thumb.height(),
                    image::ExtendedColorType::Rgba8,
                );
        if written.is_err() {
            return;
        }
        let tmp = self.file.with_extension(format!(
            "{}-{:?}.tmp",
            std::process::id(),
            std::thread::current().id()
        ));
        let saved = std::fs::write(&tmp, out).is_ok() && std::fs::rename(&tmp, &self.file).is_ok();
        if !saved {
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

/// A file's size and modified time (nanoseconds since 1970).
fn stamp(path: &Path) -> Option<(u64, u128)> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some((meta.len(), modified))
}

/// The header at the start of `bytes`, and the PNG after it.
fn parse_header(bytes: &[u8]) -> Option<(Header, &[u8])> {
    let rest = bytes.strip_prefix(MAGIC)?;
    let (w, rest) = rest.split_first_chunk::<4>()?;
    let (h, rest) = rest.split_first_chunk::<4>()?;
    let (len, rest) = rest.split_first_chunk::<8>()?;
    let (modified, rest) = rest.split_first_chunk::<16>()?;
    let (n, rest) = rest.split_first_chunk::<4>()?;
    let n = u32::from_le_bytes(*n) as usize;
    let source = std::str::from_utf8(rest.get(..n)?).ok()?.to_string();
    Some((
        Header {
            size: [u32::from_le_bytes(*w), u32::from_le_bytes(*h)],
            source,
            len: u64::from_le_bytes(*len),
            modified: u128::from_le_bytes(*modified),
        },
        rest.get(n..)?,
    ))
}

/// The header of the cache file at `path`, read without the picture after it.
fn read_header(path: &Path) -> Option<Header> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut fixed = [0u8; 40];
    f.read_exact(&mut fixed).ok()?;
    let n = u32::from_le_bytes(fixed[36..40].try_into().ok()?) as usize;
    let mut source = vec![0u8; n.min(64 * 1024)];
    f.read_exact(&mut source).ok()?;
    let mut all = fixed.to_vec();
    all.extend_from_slice(&source);
    parse_header(&all).map(|(h, _)| h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    fn temp(test: &str, cap: usize) -> (PathBuf, Cache) {
        let root =
            std::env::temp_dir().join(format!("takeshot-thumbcache-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let captures = root.join("captures");
        std::fs::create_dir_all(&captures).unwrap();
        (
            captures,
            Cache {
                dir: root.join("cache"),
                cap,
            },
        )
    }

    fn capture(dir: &Path, name: &str, w: u32) -> PathBuf {
        let p = dir.join(name);
        RgbaImage::from_pixel(w, 10, Rgba([10, 20, 30, 255]))
            .save(&p)
            .unwrap();
        p
    }

    fn files(c: &Cache) -> Vec<PathBuf> {
        std::fs::read_dir(&c.dir)
            .map(|l| l.flatten().map(|f| f.path()).collect())
            .unwrap_or_default()
    }

    fn set_modified(p: &Path, t: SystemTime) {
        std::fs::File::options()
            .write(true)
            .open(p)
            .unwrap()
            .set_modified(t)
            .unwrap();
    }

    #[test]
    fn a_cached_thumb_comes_back_for_the_same_file() {
        let (captures, cache) = temp("same", CAP);
        let shot = capture(&captures, "a.png", 40);
        let entry = cache.entry(&shot).unwrap();
        assert!(entry.load().is_none(), "nothing cached yet");
        let thumb = RgbaImage::from_pixel(8, 4, Rgba([1, 2, 3, 255]));
        entry.store(&thumb, [40, 10]);
        assert_eq!(cache.entry(&shot).unwrap().load(), Some((thumb, [40, 10])));
    }

    #[test]
    fn a_changed_file_gets_a_new_thumb() {
        let (captures, cache) = temp("changed", CAP);
        let shot = capture(&captures, "a.png", 40);
        cache
            .entry(&shot)
            .unwrap()
            .store(&RgbaImage::new(8, 4), [40, 10]);
        // Another picture under the same name.
        capture(&captures, "a.png", 90);
        assert!(cache.entry(&shot).unwrap().load().is_none(), "another size");
        // The same size, a new date.
        let e = cache.entry(&shot).unwrap();
        e.store(&RgbaImage::new(8, 4), [90, 10]);
        set_modified(&shot, SystemTime::now() + Duration::from_secs(60));
        assert!(cache.entry(&shot).unwrap().load().is_none(), "another date");
    }

    #[test]
    fn a_broken_cache_file_is_ignored() {
        let (captures, cache) = temp("broken", CAP);
        let shot = capture(&captures, "a.png", 40);
        cache
            .entry(&shot)
            .unwrap()
            .store(&RgbaImage::new(8, 4), [40, 10]);
        for f in files(&cache) {
            let bytes = std::fs::read(&f).unwrap();
            std::fs::write(&f, &bytes[..bytes.len() / 2]).unwrap();
        }
        assert!(cache.entry(&shot).unwrap().load().is_none());
    }

    #[test]
    fn the_cache_keeps_alpha() {
        let (captures, cache) = temp("alpha", CAP);
        let shot = capture(&captures, "a.png", 40);
        let thumb = RgbaImage::from_pixel(8, 4, Rgba([200, 100, 50, 90]));
        cache.entry(&shot).unwrap().store(&thumb, [40, 10]);
        let (img, _) = cache.entry(&shot).unwrap().load().unwrap();
        assert_eq!(img.get_pixel(3, 2).0, [200, 100, 50, 90]);
    }

    #[test]
    fn the_cache_drops_the_oldest_past_its_cap() {
        let (captures, cache) = temp("cap", 3);
        let base = SystemTime::now() - Duration::from_secs(3600);
        let mut kept = Vec::new();
        for i in 0..5u64 {
            let shot = capture(&captures, &format!("{i}.png"), 40);
            let e = cache.entry(&shot).unwrap();
            e.store(&RgbaImage::new(8, 4), [40, 10]);
            set_modified(&e.file, base + Duration::from_secs(i * 60));
            kept.push(e.file);
        }
        cache.prune();
        let left: Vec<bool> = kept.iter().map(|f| f.exists()).collect();
        assert_eq!(
            left,
            [false, false, true, true, true],
            "the two oldest went"
        );
    }

    #[test]
    fn a_deleted_or_changed_capture_s_thumb_goes() {
        let (captures, cache) = temp("orphans", CAP);
        let (a, b, c) = (
            capture(&captures, "a.png", 40),
            capture(&captures, "b.png", 40),
            capture(&captures, "c.png", 40),
        );
        for p in [&a, &b, &c] {
            cache
                .entry(p)
                .unwrap()
                .store(&RgbaImage::new(8, 4), [40, 10]);
        }
        // Delete in Início drops it at once.
        cache.forget(&a);
        assert!(cache.entry(&a).unwrap().load().is_none());
        // Deleted elsewhere, or changed: the prune drops them.
        std::fs::remove_file(&b).unwrap();
        capture(&captures, "c.png", 90);
        cache.prune();
        assert!(files(&cache).is_empty(), "{:?}", files(&cache));
    }
}
