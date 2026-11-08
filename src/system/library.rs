//! The captures folder as a library: the images in it, newest first, filtered by period and
//! name, their thumbnails made off the UI thread, and sending one to the trash. Both themes'
//! galleries and the tray's recent captures read it.

use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveTime};
use image::RgbaImage;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::time::SystemTime;

/// One image in the folder.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub modified: SystemTime,
    pub bytes: u64,
}

const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "bmp"];

/// The images in `dir` (not in sub-folders), newest first, then by name; a missing or unreadable
/// folder has none.
pub fn scan(dir: &Path) -> Vec<Entry> {
    let mut entries: Vec<Entry> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|x| IMAGE_EXTENSIONS.iter().any(|k| x.eq_ignore_ascii_case(k)))
                })
                .filter_map(|e| {
                    let meta = e.metadata().ok()?;
                    Some(Entry {
                        name: e.file_name().to_string_lossy().into_owned(),
                        path: e.path(),
                        modified: meta.modified().ok()?,
                        bytes: meta.len(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.name.cmp(&b.name))
    });
    entries
}

/// The gallery's filters: everything, today, or this week (from Monday 00:00, local time).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Period {
    #[default]
    All,
    Today,
    Week,
}

/// Whether a file changed at `modified` falls in `p`, seen from `now`.
pub fn in_period(modified: SystemTime, p: Period, now: DateTime<Local>) -> bool {
    let start = match p {
        Period::All => return true,
        Period::Today => now.date_naive(),
        Period::Week => {
            let back = now.weekday().num_days_from_monday() as u64;
            now.date_naive() - chrono::Days::new(back)
        }
    };
    let Some(start) = start
        .and_time(NaiveTime::MIN)
        .and_local_timezone(Local)
        .earliest()
    else {
        return true;
    };
    DateTime::<Local>::from(modified) >= start
}

/// The positions of the entries in `p` whose name contains `query` (any case), in order.
pub fn visible(entries: &[Entry], p: Period, query: &str, now: DateTime<Local>) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| in_period(e.modified, p, now))
        .filter(|(_, e)| query.is_empty() || e.name.to_lowercase().contains(&query))
        .map(|(i, _)| i)
        .collect()
}

/// The day a capture falls on, for the words that tell it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Day {
    Today,
    Yesterday,
    Date(NaiveDate),
}

/// The local day `t` falls on, seen from `now`.
pub fn day_of(t: SystemTime, now: DateTime<Local>) -> Day {
    let today = now.date_naive();
    let date = DateTime::<Local>::from(t).date_naive();
    if date == today {
        Day::Today
    } else if Some(date) == today.pred_opt() {
        Day::Yesterday
    } else {
        Day::Date(date)
    }
}

/// How many entries each filter shows: all, today, this week.
pub fn counts(entries: &[Entry], now: DateTime<Local>) -> [usize; 3] {
    let n = |p| {
        entries
            .iter()
            .filter(|e| in_period(e.modified, p, now))
            .count()
    };
    [entries.len(), n(Period::Today), n(Period::Week)]
}

/// A thumbnail: the picture shrunk to 320 px on its longest side, and its original size.
pub type Thumb = Result<(RgbaImage, [u32; 2]), String>;

const THUMB_SIDE: u32 = 320;

/// The thumbnails still to make: the newest request first (what just came on screen), each path
/// once, and requests that left the screen dropped before they are decoded.
#[derive(Default)]
pub struct ThumbQueue {
    items: Vec<PathBuf>,
}

impl ThumbQueue {
    pub fn push(&mut self, path: PathBuf) {
        self.items.retain(|p| *p != path);
        self.items.push(path);
    }

    /// Drops the waiting requests not in `wanted`.
    pub fn keep(&mut self, wanted: &HashSet<PathBuf>) {
        self.items.retain(|p| wanted.contains(p));
    }

    pub fn pop(&mut self) -> Option<PathBuf> {
        self.items.pop()
    }
}

/// The queue and whether the owner went away, shared with the worker.
type Shared = (Mutex<(ThumbQueue, bool)>, Condvar);

/// Makes thumbnails on one background thread, newest request first; each result wakes the app.
pub struct Thumbs {
    jobs: Arc<Shared>,
    rx: Mutex<Receiver<(PathBuf, Thumb)>>,
}

impl Thumbs {
    pub fn new(ctx: &egui::Context) -> Self {
        let jobs: Arc<Shared> =
            Arc::new((Mutex::new((ThumbQueue::default(), false)), Condvar::new()));
        let (done, rx) = channel();
        let ctx = ctx.clone();
        let queue = jobs.clone();
        let spawned = std::thread::Builder::new()
            .name("takeshot-thumbs".into())
            .spawn(move || {
                loop {
                    let path = {
                        let (lock, ready) = &*queue;
                        let Ok(mut guard) = lock.lock() else { return };
                        loop {
                            if guard.1 {
                                return;
                            }
                            if let Some(p) = guard.0.pop() {
                                break p;
                            }
                            guard = match ready.wait(guard) {
                                Ok(g) => g,
                                Err(_) => return,
                            };
                        }
                    };
                    let thumb = make_thumb(&path);
                    if done.send((path, thumb)).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
            });
        if let Err(e) = spawned {
            eprintln!("takeshot: no thumbnails: {e}");
        }
        Self {
            jobs,
            rx: Mutex::new(rx),
        }
    }

    pub fn request(&self, path: PathBuf) {
        let (lock, ready) = &*self.jobs;
        if let Ok(mut g) = lock.lock() {
            g.0.push(path);
            ready.notify_one();
        }
    }

    /// Forgets the waiting requests not in `wanted` (the cards that left the screen).
    pub fn keep(&self, wanted: &HashSet<PathBuf>) {
        if let Ok(mut g) = self.jobs.0.lock() {
            g.0.keep(wanted);
        }
    }

    /// The thumbnails finished since the last call.
    pub fn poll(&self) -> Vec<(PathBuf, Thumb)> {
        self.rx
            .lock()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default()
    }
}

impl Drop for Thumbs {
    fn drop(&mut self) {
        let (lock, ready) = &*self.jobs;
        if let Ok(mut g) = lock.lock() {
            g.1 = true;
            ready.notify_all();
        }
    }
}

/// image's default limits (512 MiB) turn a gigantic picture into an error instead of a stall.
fn make_thumb(path: &Path) -> Thumb {
    let img = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let (w, h) = img.dimensions();
    let k = (THUMB_SIDE as f32 / w.max(h) as f32).min(1.0);
    let small = if k < 1.0 {
        image::imageops::thumbnail(
            &img,
            ((w as f32 * k).round() as u32).max(1),
            ((h as f32 * k).round() as u32).max(1),
        )
    } else {
        img
    };
    Ok((small, [w, h]))
}

/// Sends a file to the system's trash (recoverable), not a permanent delete.
pub fn move_to_trash(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("{} does not exist", path.display()));
    }
    trash::delete(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {

    use super::*;
    use chrono::{Local, TimeZone};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    /// A screen-like capture: bars and panels in flat colours, rows of "text", and a photo-like
    /// block of noise, so it compresses and decodes like a real one.
    fn screen_like(w: u32, h: u32, seed: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            let n = (x.wrapping_mul(73_856_093)
                ^ y.wrapping_mul(19_349_663)
                ^ seed.wrapping_mul(83_492_791))
            .wrapping_mul(2_654_435_761);
            let v = (n >> 24) as u8;
            if y < h / 14 {
                image::Rgba([32, 33, 38, 255])
            } else if x < w / 6 {
                image::Rgba([245, 245, 247, 255])
            } else if x > w / 2 && y > h / 2 {
                image::Rgba([v, v / 2 + 60, 255 - v, 255])
            } else if (y / 9) % 3 == 0 && (x / 7 + seed) % 11 != 0 && v > 90 {
                image::Rgba([40, 40, 46, 255])
            } else {
                image::Rgba([255, 255, 255, 255])
            }
        })
    }

    /// Makes the bench folder: 40 screen captures (PNG, 1920 x 1080 and 2560 x 1440) and 10
    /// phone photos (JPEG, 4032 x 3024).
    fn bench_folder(dir: &Path) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for i in 0..40u32 {
            let (w, h) = if i % 3 == 0 {
                (2560, 1440)
            } else {
                (1920, 1080)
            };
            let p = dir.join(format!("shot-{i:03}.png"));
            screen_like(w, h, i).save(&p).unwrap();
            paths.push(p);
        }
        for i in 0..10u32 {
            let p = dir.join(format!("photo-{i:03}.jpg"));
            image::DynamicImage::ImageRgba8(screen_like(4032, 3024, 100 + i))
                .to_rgb8()
                .save(&p)
                .unwrap();
            paths.push(p);
        }
        paths
    }

    /// Milliseconds per file for `f` over `paths`, by kind (PNG captures, JPEG photos).
    fn per_kind(paths: &[PathBuf], mut f: impl FnMut(&Path)) -> (f64, f64) {
        let mut time = |ext: &str| {
            let some: Vec<_> = paths
                .iter()
                .filter(|p| p.extension().is_some_and(|e| e == ext))
                .collect();
            let t = std::time::Instant::now();
            for p in &some {
                f(p);
            }
            t.elapsed().as_secs_f64() * 1000.0 / some.len() as f64
        };
        (time("png"), time("jpg"))
    }

    #[test]
    #[ignore = "benchmark: cargo test --release thumb_bench -- --ignored --nocapture"]
    fn thumb_bench() {
        let dir = temp_dir("thumb-bench");
        let paths = bench_folder(&dir);
        let (png, jpg) = per_kind(&paths, |p| {
            make_thumb(p).unwrap();
        });
        println!("thumb_bench: one thread, no cache: {png:.1} ms a capture, {jpg:.1} ms a photo");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn temp_dir(test: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("takeshot-test-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn touch(path: &Path, bytes: &[u8], modified: SystemTime) {
        std::fs::write(path, bytes).unwrap();
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(modified)
            .unwrap();
    }

    /// Wednesday 2026-11-04 10:00 local; the week began on Monday the 2nd.
    fn wednesday() -> chrono::DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 4, 10, 0, 0).unwrap()
    }

    fn at(d: u32, h: u32, m: u32) -> SystemTime {
        SystemTime::from(Local.with_ymd_and_hms(2026, 11, d, h, m, 0).unwrap())
    }

    fn entry(name: &str, modified: SystemTime) -> Entry {
        Entry {
            path: PathBuf::from(name),
            name: name.to_string(),
            modified,
            bytes: 1,
        }
    }

    #[test]
    fn scan_lists_images_newest_first() {
        let d = temp_dir("scan");
        let now = SystemTime::now();
        touch(&d.join("a.png"), b"x", now - Duration::from_secs(100));
        touch(&d.join("b.JPG"), b"xy", now);
        touch(&d.join("c.txt"), b"x", now);
        std::fs::create_dir(d.join("sub.png")).unwrap();
        let names: Vec<String> = scan(&d).into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["b.JPG", "a.png"]);
        assert_eq!(scan(&d)[0].bytes, 2);
        assert!(
            scan(&d.join("missing")).is_empty(),
            "a missing folder has none"
        );
    }

    #[test]
    fn periods_cut_at_midnight_and_monday() {
        let now = wednesday();
        assert!(in_period(at(4, 0, 0), Period::Today, now));
        let tuesday_late = at(3, 23, 59);
        assert!(!in_period(tuesday_late, Period::Today, now));
        assert!(in_period(tuesday_late, Period::Week, now));
        assert!(in_period(at(2, 0, 0), Period::Week, now), "Monday 00:00");
        let sunday_late = at(1, 23, 59);
        assert!(!in_period(sunday_late, Period::Week, now));
        assert!(!in_period(sunday_late, Period::Today, now));
        assert!(in_period(sunday_late, Period::All, now));
    }

    #[test]
    fn search_and_period_combine() {
        let entries = vec![
            entry("Takeshot 2026-11-04 09-00-00.png", at(4, 9, 0)),
            entry("Takeshot 2026-11-03 09-00-00.png", at(3, 9, 0)),
            entry("other.png", at(4, 8, 0)),
        ];
        assert_eq!(
            visible(&entries, Period::Today, "TAKESHOT 2026", wednesday()),
            [0]
        );
        assert_eq!(
            visible(&entries, Period::All, "takeshot", wednesday()),
            [0, 1]
        );
        assert_eq!(visible(&entries, Period::Week, "", wednesday()), [0, 1, 2]);
    }

    #[test]
    fn counts_match_the_filters() {
        let entries = vec![
            entry("a.png", at(4, 9, 0)),
            entry("b.png", at(4, 8, 0)),
            entry("c.png", at(3, 9, 0)),
            entry(
                "d.png",
                SystemTime::from(Local.with_ymd_and_hms(2026, 10, 2, 9, 0, 0).unwrap()),
            ),
        ];
        assert_eq!(counts(&entries, wednesday()), [4, 2, 3]);
    }

    #[test]
    fn a_broken_image_fails_alone() {
        let d = temp_dir("thumbs");
        let broken = d.join("broken.png");
        std::fs::write(&broken, b"not an image").unwrap();
        let good = d.join("good.png");
        image::RgbaImage::new(40, 20).save(&good).unwrap();
        let big = d.join("big.png");
        image::RgbaImage::new(1000, 500).save(&big).unwrap();
        let thumbs = Thumbs::new(&egui::Context::default());
        for p in [&broken, &good, &big] {
            thumbs.request(p.clone());
        }
        let mut got = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while got.len() < 3 && std::time::Instant::now() < deadline {
            got.extend(thumbs.poll());
            std::thread::sleep(Duration::from_millis(20));
        }
        let find = |p: &PathBuf| got.iter().find(|(q, _)| q == p).map(|(_, r)| r);
        assert!(matches!(find(&broken), Some(Err(_))));
        let (img, dims) = find(&good).unwrap().as_ref().unwrap();
        assert_eq!(*dims, [40, 20]);
        assert_eq!(img.dimensions(), (40, 20), "small stays small");
        let (img, dims) = find(&big).unwrap().as_ref().unwrap();
        assert_eq!(*dims, [1000, 500], "the original size");
        assert_eq!(img.dimensions(), (320, 160), "longest side 320");
    }

    #[test]
    fn the_newest_request_goes_first_and_dropped_ones_never_decode() {
        let mut q = ThumbQueue::default();
        for p in ["a.png", "b.png", "c.png"] {
            q.push(PathBuf::from(p));
        }
        q.push(PathBuf::from("a.png"));
        let keep: std::collections::HashSet<PathBuf> =
            ["a.png", "c.png"].into_iter().map(PathBuf::from).collect();
        q.keep(&keep);
        assert_eq!(q.pop(), Some(PathBuf::from("a.png")), "asked again: newest");
        assert_eq!(q.pop(), Some(PathBuf::from("c.png")));
        assert_eq!(q.pop(), None, "b left the screen; a is not asked twice");
    }

    #[test]
    fn trash_reports_a_missing_file() {
        let d = temp_dir("trash");
        assert!(move_to_trash(&d.join("nope.png")).is_err());
    }
}
