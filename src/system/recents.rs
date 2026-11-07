//! The latest captures in the save folder, for the tray menu and the Home gallery.

use std::path::{Path, PathBuf};

/// The `n` newest PNG files in `dir` (by modification time, newest first); a missing or unreadable
/// folder has none.
pub fn recent_captures(dir: &Path, n: usize) -> Vec<PathBuf> {
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|x| x.eq_ignore_ascii_case("png"))
                })
                .filter_map(|e| Some((e.path(), e.metadata().ok()?.modified().ok()?)))
                .collect()
        })
        .unwrap_or_default();
    files.sort_by_key(|f| std::cmp::Reverse(f.1));
    files.into_iter().take(n).map(|(p, _)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn scratch(test: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("takeshot-recents-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn file_at(dir: &Path, name: &str, age_s: u64) -> PathBuf {
        let p = dir.join(name);
        let f = std::fs::File::create(&p).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(age_s))
            .unwrap();
        p
    }

    #[test]
    fn recents_are_the_newest_pngs_and_survive_odd_folders() {
        let dir = scratch("odd");
        // Seven PNGs, the higher the number the newer, plus noise that is not a capture.
        let pngs: Vec<PathBuf> = (0..7)
            .map(|i| file_at(&dir, &format!("shot-{i}.png"), 100 - i * 10))
            .collect();
        file_at(&dir, "notes.txt", 0);
        file_at(&dir, "LOUD.PNG", 5);
        std::fs::create_dir_all(dir.join("folder.png")).unwrap();
        let got = recent_captures(&dir, 5);
        let want = vec![
            dir.join("LOUD.PNG"),
            pngs[6].clone(),
            pngs[5].clone(),
            pngs[4].clone(),
            pngs[3].clone(),
        ];
        assert_eq!(got, want);
        assert!(recent_captures(&dir.join("missing"), 5).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
