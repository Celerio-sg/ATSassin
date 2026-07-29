//! Scripted, non-interactive verification of the TUI's keybindings.
//!
//! Spawns the real `atsassin.exe tui` binary inside a pseudo-terminal
//! (ConPTY on Windows) and sends it real key bytes exactly as a keyboard
//! would, capturing the rendered screen after each keypress via a `vt100`
//! terminal emulator. This exercises the actual compiled binary end-to-end -
//! real profile parsing, real (Groq) role inference, real network scraping -
//! not a mock. Run with:
//!
//!   cargo run --release --example tui_pty_probe

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const ROWS: u16 = 42;
const COLS: u16 = 130;

fn drain(parser: &mut vt100::Parser, rx: &mpsc::Receiver<Vec<u8>>, wait: Duration) {
    let start = Instant::now();
    while start.elapsed() < wait {
        if let Ok(chunk) = rx.recv_timeout(Duration::from_millis(100)) {
            parser.process(&chunk);
        }
    }
}

fn dump(label: &str, parser: &vt100::Parser) {
    println!("\n===== {label} =====");
    let screen = parser.screen();
    let (rows, cols) = screen.size();
    for row in 0..rows {
        let mut line = String::new();
        for col in 0..cols {
            match screen.cell(row, col) {
                Some(cell) => {
                    let s = cell.contents();
                    if s.is_empty() {
                        line.push(' ');
                    } else {
                        line.push_str(&s);
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

fn main() -> anyhow::Result<()> {
    let repo_root = std::env::current_dir()?;
    let binary = repo_root.join("target/release/atsassin.exe");
    if !binary.exists() {
        anyhow::bail!(
            "Binary not found at {}. Run `cargo build --release` first.",
            binary.display()
        );
    }

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: ROWS,
        cols: COLS,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(&binary);
    cmd.arg("tui");
    cmd.arg("--ascii");
    cmd.cwd(&repo_root);
    cmd.env("DATABASE_PATH", "pty_probe.db");
    cmd.env("PROFILE_PATH", "pty_probe_profile.md");

    std::fs::copy(
        repo_root.join("tests/uat/scenario_1_synthetic_apac_gtm/profile.md"),
        repo_root.join("pty_probe_profile.md"),
    )?;

    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;

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

    let mut parser = vt100::Parser::new(ROWS, COLS, 0);

    drain(&mut parser, &rx, Duration::from_secs(2));
    dump(
        "1. STARTUP (no profile-derived roles yet, jobs table empty)",
        &parser,
    );

    println!("\n>>> sending 'r' (infer roles - real Groq call) <<<");
    writer.write_all(b"r")?;
    drain(&mut parser, &rx, Duration::from_secs(1));
    dump(
        "2. IMMEDIATELY AFTER 'r' (should show 'inferring...')",
        &parser,
    );
    drain(&mut parser, &rx, Duration::from_secs(6));
    dump("3. AFTER ROLE INFERENCE SETTLES", &parser);

    println!("\n>>> sending 'j','j','j' (navigate jobs table down) <<<");
    writer.write_all(b"jjj")?;
    drain(&mut parser, &rx, Duration::from_millis(800));
    dump("4. AFTER 'j','j','j'", &parser);

    println!("\n>>> sending 'k' (navigate up) <<<");
    writer.write_all(b"k")?;
    drain(&mut parser, &rx, Duration::from_millis(500));
    dump("5. AFTER 'k'", &parser);

    println!("\n>>> sending 's' (scan - real network scrape) <<<");
    writer.write_all(b"s")?;
    drain(&mut parser, &rx, Duration::from_secs(2));
    dump(
        "6. IMMEDIATELY AFTER 's' (scan modal should be visible)",
        &parser,
    );
    drain(&mut parser, &rx, Duration::from_secs(10));
    dump("7. WHILE SCAN IN PROGRESS", &parser);
    drain(&mut parser, &rx, Duration::from_secs(10));
    dump(
        "8. AFTER SCAN COMPLETES (modal gone, job table populated)",
        &parser,
    );

    println!("\n>>> sending 'q' (quit) <<<");
    writer.write_all(b"q")?;
    drain(&mut parser, &rx, Duration::from_secs(1));

    let _ = child.kill();
    let _ = std::fs::remove_file(repo_root.join("pty_probe_profile.md"));
    let _ = std::fs::remove_file(repo_root.join("pty_probe.db"));
    let _ = std::fs::remove_file(repo_root.join("pty_probe.llm_telemetry.jsonl"));

    println!("\n===== PROBE COMPLETE =====");
    Ok(())
}
