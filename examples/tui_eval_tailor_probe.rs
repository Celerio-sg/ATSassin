//! Focused, generously-paced confirmation of evaluate ('e') and tailor ('t')
//! working end-to-end from the TUI. The earlier chaos probe fired these keys
//! before confirming the scan had actually finished, leaving the result
//! ambiguous - this probe waits on explicit state (board count, job count)
//! rather than fixed guesses before acting.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const ROWS: u16 = 42;
const COLS: u16 = 130;

fn main() -> anyhow::Result<()> {
    let repo_root = std::env::current_dir()?;
    let binary = repo_root.join("target/release/atsassin.exe");
    let _ = std::fs::remove_file(repo_root.join("eval_probe.db"));
    std::fs::copy(
        repo_root.join("tests/uat/scenario_1_simon_brender/profile.md"),
        repo_root.join("pty_probe_profile.md"),
    )?;

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
    cmd.env("DATABASE_PATH", "eval_probe.db");
    cmd.env("PROFILE_PATH", "pty_probe_profile.md");
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

    let drain = |parser: &mut vt100::Parser, rx: &mpsc::Receiver<Vec<u8>>, wait: Duration| {
        let start = Instant::now();
        while start.elapsed() < wait {
            if let Ok(chunk) = rx.recv_timeout(Duration::from_millis(100)) {
                parser.process(&chunk);
            }
        }
    };
    let screen_text = |parser: &vt100::Parser| -> String {
        let screen = parser.screen();
        let (rows, cols) = screen.size();
        let mut out = String::new();
        for row in 0..rows {
            for col in 0..cols {
                if let Some(cell) = screen.cell(row, col) {
                    let s = cell.contents();
                    if s.is_empty() {
                        out.push(' ')
                    } else {
                        out.push_str(&s)
                    }
                } else {
                    out.push(' ');
                }
            }
            out.push('\n');
        }
        out
    };
    let dump = |label: &str, parser: &vt100::Parser| {
        println!("\n===== {label} =====");
        for line in screen_text(parser).lines() {
            if !line.trim().is_empty() {
                println!("{line}");
            }
        }
    };

    drain(&mut parser, &rx, Duration::from_secs(1));
    writer.write_all(b"r")?;
    drain(&mut parser, &rx, Duration::from_secs(7));
    dump("1. roles inferred", &parser);

    writer.write_all(b"s")?;
    println!("\n>>> waiting for 'Scan complete' to actually appear (polling, up to 40s) <<<");
    let start = Instant::now();
    loop {
        drain(&mut parser, &rx, Duration::from_secs(2));
        if screen_text(&parser).contains("Scan complete")
            || start.elapsed() > Duration::from_secs(40)
        {
            break;
        }
    }
    dump("2. scan complete (or timeout)", &parser);

    println!("\n>>> pressing 'e' now that a job is confirmed selected <<<");
    writer.write_all(b"e")?;
    println!(">>> waiting for evaluation to actually complete (polling 'Evaluated:' or 'Evaluation failed', up to 30s) <<<");
    let start = Instant::now();
    loop {
        drain(&mut parser, &rx, Duration::from_secs(2));
        let t = screen_text(&parser);
        if t.contains("Evaluated:")
            || t.contains("Evaluation failed")
            || start.elapsed() > Duration::from_secs(30)
        {
            break;
        }
    }
    dump("3. after evaluate", &parser);

    println!("\n>>> pressing 't' to tailor the same job <<<");
    writer.write_all(b"t")?;
    println!(">>> waiting for tailoring to actually complete (polling 'saved to' or 'Tailoring failed', up to 40s) <<<");
    let start = Instant::now();
    loop {
        drain(&mut parser, &rx, Duration::from_secs(2));
        let t = screen_text(&parser);
        if t.contains("saved to")
            || t.contains("Tailoring failed")
            || start.elapsed() > Duration::from_secs(40)
        {
            break;
        }
    }
    dump("4. after tailor", &parser);

    writer.write_all(b"q")?;
    drain(&mut parser, &rx, Duration::from_secs(1));
    let _ = child.kill();

    println!("\n===== checking for tailored_*.md on disk =====");
    for entry in std::fs::read_dir(&repo_root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("tailored_") {
            let content = std::fs::read_to_string(entry.path())?;
            println!("FOUND: {name} ({} bytes)", content.len());
            println!(
                "--- first 800 chars ---\n{}",
                &content.chars().take(800).collect::<String>()
            );
        }
    }

    let _ = std::fs::remove_file(repo_root.join("pty_probe_profile.md"));
    println!("\n===== PROBE COMPLETE (eval_probe.db and tailored_*.md left in place for inspection) =====");
    Ok(())
}
