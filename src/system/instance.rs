//! Single instance: the first Takeshot process owns a local socket; a second launch asks it to show a page and exits.

// The resident process wires it in plan 4, Task 6; until then only the tests use it.
#![allow(dead_code)]

#[cfg(target_os = "macos")]
use interprocess::local_socket::GenericFilePath;
#[cfg(not(target_os = "macos"))]
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::{ListenerOptions, Name, Stream, prelude::*};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::sync::Arc;
use std::time::Duration;

/// The main window pages a second launch or the tray can ask for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Start,
    Style,
    Settings,
    About,
}

impl Page {
    pub fn as_str(self) -> &'static str {
        match self {
            Page::Start => "start",
            Page::Style => "style",
            Page::Settings => "settings",
            Page::About => "about",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "start" => Some(Page::Start),
            "style" => Some(Page::Style),
            "settings" => Some(Page::Settings),
            "about" => Some(Page::About),
            _ => None,
        }
    }
}

/// The socket name for this user: one resident per user session.
pub fn endpoint() -> String {
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_default();
    let safe: String = user
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    format!("takeshot-{}", if safe.is_empty() { "user" } else { &safe })
}

/// A named pipe on Windows and an abstract socket on Linux; on macOS a file in the per-user temp
/// folder, since the shared `/tmp` would let other users take the name.
#[cfg(not(target_os = "macos"))]
fn socket_name(name: &str) -> io::Result<Name<'static>> {
    format!("{name}.sock").to_ns_name::<GenericNamespaced>()
}

#[cfg(target_os = "macos")]
fn socket_name(name: &str) -> io::Result<Name<'static>> {
    std::env::temp_dir()
        .join(format!("{name}.sock"))
        .to_fs_name::<GenericFilePath>()
}

/// The name is taken: Unix says "address in use", Windows says "access denied" for an existing pipe.
fn in_use(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::AddrInUse | io::ErrorKind::PermissionDenied
    )
}

fn connect(name: &str) -> io::Result<Stream> {
    Stream::connect(socket_name(name)?)
}

fn create(name: &str, overwrite: bool) -> io::Result<interprocess::local_socket::Listener> {
    ListenerOptions::new()
        .name(socket_name(name)?)
        .try_overwrite(overwrite)
        .create_sync()
}

/// The first process's end of the socket.
pub struct Listener(interprocess::local_socket::Listener);

pub enum Claim {
    /// No other Takeshot runs: this process is the resident.
    Primary(Listener),
    /// A resident answers: ask it to show itself and exit.
    Secondary,
    /// The socket cannot be used at all: run anyway, without single instance.
    Alone,
}

/// Becomes the resident, or finds the one that runs. A resident that crashed and left its socket
/// file behind (file-path names) does not block the next launch.
pub fn claim(name: &str) -> Claim {
    for _ in 0..5 {
        if connect(name).is_ok() {
            return Claim::Secondary;
        }
        match create(name, false) {
            Ok(l) => return Claim::Primary(Listener(l)),
            // Another launch is starting right now, or a dead resident left its name behind.
            Err(e) if in_use(&e) => std::thread::sleep(Duration::from_millis(60)),
            Err(e) => return alone(e),
        }
    }
    // Still nobody answers: the name is a dead resident's leftover file, which can be reclaimed.
    if connect(name).is_ok() {
        return Claim::Secondary;
    }
    match create(name, true) {
        Ok(l) => Claim::Primary(Listener(l)),
        Err(e) => alone(e),
    }
}

fn alone(e: io::Error) -> Claim {
    eprintln!("takeshot: running without single instance: {e}");
    Claim::Alone
}

/// Asks the resident to show `page`. Retries briefly: the resident may be between connections.
pub fn send_show(name: &str, page: Page) -> io::Result<()> {
    let mut last = io::Error::other("no attempt");
    for _ in 0..10 {
        match connect(name) {
            Ok(mut s) => {
                s.write_all(format!("show {}\n", page.as_str()).as_bytes())?;
                return s.flush();
            }
            Err(e) => {
                last = e;
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    Err(last)
}

impl Listener {
    /// Answers second launches on a background thread for the rest of the process. Each
    /// connection gets its own short thread and at most 64 bytes, so a client that never writes
    /// cannot hold up the next launch.
    pub fn spawn(self, on_show: impl Fn(Page) + Send + Sync + 'static) {
        let listener = self.0;
        let on_show = Arc::new(on_show);
        let spawned = std::thread::Builder::new()
            .name("takeshot-instance".into())
            .spawn(move || {
                for conn in listener.incoming() {
                    let Ok(conn) = conn else {
                        std::thread::sleep(Duration::from_millis(50));
                        continue;
                    };
                    let on_show = on_show.clone();
                    std::thread::spawn(move || {
                        let mut line = String::new();
                        if BufReader::new(conn.take(64)).read_line(&mut line).is_ok()
                            && let Some(page) =
                                line.trim().strip_prefix("show ").and_then(Page::parse)
                        {
                            on_show(page);
                        }
                    });
                }
            });
        if let Err(e) = spawned {
            eprintln!("takeshot: cannot answer second launches: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use std::time::Duration;

    fn unique(test: &str) -> String {
        format!("takeshot-test-{}-{test}", std::process::id())
    }

    #[test]
    fn second_claim_is_secondary_and_its_show_reaches_the_first() {
        let name = unique("second");
        let Claim::Primary(listener) = claim(&name) else {
            panic!("the first claim must be primary");
        };
        let (tx, rx) = channel();
        listener.spawn(move |page| {
            let _ = tx.send(page);
        });
        assert!(matches!(claim(&name), Claim::Secondary));
        send_show(&name, Page::Settings).unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)), Ok(Page::Settings));
    }

    #[test]
    fn a_dead_primary_does_not_block_the_next_claim() {
        let name = unique("dead");
        let Claim::Primary(listener) = claim(&name) else {
            panic!("the first claim must be primary");
        };
        drop(listener);
        assert!(matches!(claim(&name), Claim::Primary(_)));
    }

    #[test]
    fn launches_racing_for_the_name_leave_exactly_one_primary() {
        let name = unique("race");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let (name, barrier) = (name.clone(), barrier.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    match claim(&name) {
                        Claim::Primary(l) => {
                            l.spawn(|_| {});
                            "primary"
                        }
                        Claim::Secondary => "secondary",
                        Claim::Alone => "alone",
                    }
                })
            })
            // Collect first: joining inside the same lazy chain would wait at the barrier forever.
            .collect::<Vec<_>>()
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<Vec<_>>();
        let count = |k: &str| threads.iter().filter(|r| **r == k).count();
        assert_eq!(count("primary"), 1, "{threads:?}");
        assert_eq!(count("alone"), 0, "{threads:?}");
    }

    #[test]
    fn a_silent_client_does_not_block_the_next_show() {
        let name = unique("silent");
        let Claim::Primary(listener) = claim(&name) else {
            panic!("the first claim must be primary");
        };
        let (tx, rx) = channel();
        listener.spawn(move |page| {
            let _ = tx.send(page);
        });
        // A client that connects and never writes a line.
        let _mute = connect(&name).unwrap();
        send_show(&name, Page::Style).unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)), Ok(Page::Style));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_stale_socket_file_is_reclaimed() {
        let name = unique("stale");
        let path = std::env::temp_dir().join(format!("{name}.sock"));
        // std leaves the file behind when its listener drops, like a crashed resident.
        drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
        assert!(path.exists());
        assert!(matches!(claim(&name), Claim::Primary(_)));
    }

    #[test]
    fn page_round_trips_through_the_wire_format() {
        for page in [Page::Start, Page::Style, Page::Settings, Page::About] {
            assert_eq!(Page::parse(page.as_str()), Some(page));
        }
        assert_eq!(Page::parse("nope"), None);
    }

    #[test]
    fn the_endpoint_is_per_user_and_safe() {
        let e = endpoint();
        assert!(e.starts_with("takeshot-"), "{e}");
        assert!(
            e.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "{e}"
        );
    }
}
