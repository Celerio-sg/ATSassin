//! Chaos-engineering pass on the TUI: role persistence across separate
//! process launches, evaluate/tailor actions, rapid keypresses, and a
//! terminal resize mid-session. Spawns the real binary in a real ConPTY and
//! sends real key bytes - not a mock of the app.
//!
//! Run with: cargo run --release --example tui_chaos_probe

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const ROWS: u16 = 42;
const COLS: u16 = 130;

struct Session {
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    rx: mpsc::Receiver<Vec<u8>>,
    parser: vt100::Parser,
}

impl Session {
    fn spawn(
        binary: &std::path::Path,
        repo_root: &std::path::Path,
        db_name: &str,
        extra_args: &[&str],
    ) -> anyhow::Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: ROWS,
            cols: COLS,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(binary);
        cmd.arg("tui");
        cmd.arg("--ascii");
        for a in extra_args {
            cmd.arg(a);
        }
        cmd.cwd(repo_root);
        cmd.env("DATABASE_PATH", db_name);
        cmd.env("PROFILE_PATH", "pty_probe_profile.md");

        let child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut buf = [0u8; 16384];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            _child: child,
            master: pair.master,
            writer,
            rx,
            parser: vt100::Parser::new(ROWS, COLS, 0),
        })
    }

    /// Resizes the REAL pty (delivers a real resize event to the child, the
    /// same as a user dragging their terminal window edge) - not just the
    /// local capture buffer.
    fn resize(&mut self, rows: u16, cols: u16) -> anyhow::Result<()> {
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        self.parser.set_size(rows, cols);
        Ok(())
    }

    fn drain(&mut self, wait: Duration) {
        let start = Instant::now();
        while start.elapsed() < wait {
            if let Ok(chunk) = self.rx.recv_timeout(Duration::from_millis(100)) {
                self.parser.process(&chunk);
            }
        }
    }

    fn send(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(bytes)?;
        Ok(())
    }

    fn dump(&self, label: &str) {
        println!("\n===== {label} =====");
        let screen = self.parser.screen();
        let (rows, cols) = screen.size();
        for row in 0..rows {
            let mut line = String::new();
            for col in 0..cols {
                match screen.cell(row, col) {
                    Some(cell) => {
                        let s = cell.contents();
                        if s.is_empty() {
                            line.push(' ')
                        } else {
                            line.push_str(&s)
                        }
                    }
                    None => line.push(' '),
                }
            }
            let trimmed = line.trim_end();
            if !trimmed.is_empty() {
                println!("{trimmed}");
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let repo_root = std::env::current_dir()?;
    let binary = repo_root.join("target/release/atsassin.exe");
    if !binary.exists() {
        anyhow::bail!("Binary not found. Run `cargo build --release` first.");
    }

    let _ = std::fs::remove_file(repo_root.join("chaos.db"));
    let _ = std::fs::remove_file(repo_root.join("chaos.llm_telemetry.jsonl"));
    std::fs::copy(
        repo_root.join("tests/uat/scenario_1_simon_brender/profile.md"),
        repo_root.join("pty_probe_profile.md"),
    )?;

    // ---- Session A: fresh DB, infer roles, quit ----
    println!("\n########## SESSION A: fresh DB, infer roles, then quit ##########");
    let mut a = Session::spawn(&binary, &repo_root, "chaos.db", &[])?;
    a.drain(Duration::from_secs(1));
    a.dump("A1. startup - roles should be empty (fresh db)");

    a.send(b"r")?;
    a.drain(Duration::from_secs(7));
    a.dump("A2. after 'r' - real Groq role inference should have completed");

    a.send(b"q")?;
    a.drain(Duration::from_millis(500));
    drop(a);

    // ---- Session B: same DB, brand new process - roles should load WITHOUT re-inferring ----
    println!("\n########## SESSION B: relaunch against same DB - roles should persist ##########");
    let mut b = Session::spawn(&binary, &repo_root, "chaos.db", &[])?;
    b.drain(Duration::from_secs(1));
    b.dump("B1. startup - roles should ALREADY be populated (loaded from DB, no LLM call)");

    println!("\n>>> chaos: mashing 'jjjjjjjjjjkkkkkkkkkk' rapidly with no jobs yet <<<");
    b.send(b"jjjjjjjjjjkkkkkkkkkk")?;
    b.drain(Duration::from_millis(500));
    b.dump("B2. after rapid nav-mash on empty job table (must not crash/hang)");

    println!("\n>>> chaos: 'e' and 't' with NO job selected (empty table) <<<");
    b.send(b"e")?;
    b.drain(Duration::from_millis(300));
    b.send(b"t")?;
    b.drain(Duration::from_millis(300));
    b.dump("B3. after 'e' and 't' with no jobs (should show a clear hint, not crash)");

    println!("\n>>> chaos: resize the REAL pty to 60x20 mid-session <<<");
    b.resize(20, 60)?;
    b.drain(Duration::from_millis(500));
    b.dump("B4. after resize to 60x20 (must not panic)");

    println!("\n>>> resizing back to full size <<<");
    b.resize(ROWS, COLS)?;
    b.drain(Duration::from_millis(300));

    println!("\n>>> sending 's' to scan for real jobs <<<");
    b.send(b"s")?;
    b.drain(Duration::from_secs(14));
    b.dump("B5. after scan settles - jobs table should be populated");

    println!("\n>>> chaos: double-tapping 's' immediately (must not double-scan/crash) <<<");
    b.send(b"ss")?;
    b.drain(Duration::from_secs(2));
    b.dump("B6. after double 's' tap");
    b.drain(Duration::from_secs(6));

    println!("\n>>> selecting first job and pressing 'e' to evaluate <<<");
    b.send(b"e")?;
    b.drain(Duration::from_secs(10));
    b.dump("B7. after 'e' - real evaluation with dimensions/strengths/gaps should render");

    println!("\n>>> pressing 't' to tailor the same job <<<");
    b.send(b"t")?;
    b.drain(Duration::from_secs(14));
    b.dump("B8. after 't' - tailored resume+cover letter should be saved to disk");

    println!("\n>>> chaos: cycling region filter 'g' four times (wraps around) <<<");
    b.send(b"gggg")?;
    b.drain(Duration::from_millis(500));
    b.dump("B9. after 4x 'g' (region filter should be back to Global)");

    b.send(b"q")?;
    b.drain(Duration::from_secs(1));

    let _ = std::fs::remove_file(repo_root.join("pty_probe_profile.md"));
    let _ = std::fs::remove_file(repo_root.join("chaos.db"));
    let _ = std::fs::remove_file(repo_root.join("chaos.llm_telemetry.jsonl"));
    for entry in std::fs::read_dir(&repo_root)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with("tailored_") {
            let _ = std::fs::remove_file(entry.path());
        }
    }

    println!("\n===== CHAOS PROBE COMPLETE =====");
    Ok(())
}
