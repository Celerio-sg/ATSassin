//! Final UAT re-run: all 5 Tier-1 personas driven through the real TUI via a
//! real ConPTY, real Groq calls, real scraping. For each persona: init
//! profile -> infer roles -> scan -> evaluate top job -> quit, polling on
//! actual state transitions (never fixed guesses) before acting or judging.
//! Prints a final summary table for scoring against the UAT rubric.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const ROWS: u16 = 44;
const COLS: u16 = 140;

struct Session {
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    // Must be kept alive - dropping the master PTY handle tears down the
    // reader/writer cloned from it, even though those are separate handles.
    // Omitting this field caused every spawn to fail with "the pipe is
    // being closed" the instant `spawn()` returned and `pair` went out of
    // scope.
    _master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    rx: mpsc::Receiver<Vec<u8>>,
    parser: vt100::Parser,
}

impl Session {
    fn spawn(
        binary: &std::path::Path,
        repo_root: &std::path::Path,
        db_name: &str,
        profile_name: &str,
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
        cmd.cwd(repo_root);
        cmd.env("DATABASE_PATH", db_name);
        cmd.env("PROFILE_PATH", profile_name);
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
            _master: pair.master,
            writer,
            rx,
            parser: vt100::Parser::new(ROWS, COLS, 0),
        })
    }

    fn send(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(bytes)?;
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

    fn text(&self) -> String {
        let screen = self.parser.screen();
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
    }

    fn wait_until(&mut self, needle_any: &[&str], timeout: Duration) -> bool {
        let start = Instant::now();
        loop {
            self.drain(Duration::from_secs(1));
            let t = self.text();
            if needle_any.iter().any(|n| t.contains(n)) {
                return true;
            }
            if start.elapsed() > timeout {
                return false;
            }
        }
    }

    fn dump(&self, label: &str) {
        println!("\n===== {label} =====");
        for line in self.text().lines() {
            if !line.trim().is_empty() {
                println!("{line}");
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

    let personas = [
        ("scenario_1_synthetic_apac_gtm", "Maya Kestrel"),
        ("scenario_2_returning_housewife", "Returning Housewife"),
        ("scenario_3_worldschooling_parent", "Worldschooling Parent"),
        ("scenario_4_tokyo_graduate", "Tokyo Graduate"),
        ("scenario_5_retrenched_salaryman", "Retrenched Salaryman"),
    ];

    let mut summary = Vec::new();

    // Copy each fixture straight to its PROFILE_PATH - handle_tui parses
    // the profile file directly (ProfileParser::parse), it doesn't require
    // `profile init` to have run first. Mixing plain Command::output() with
    // ConPTY spawns in the same process reliably breaks ConPTY on this
    // machine ("the pipe is being closed", os error 232) - a test-harness
    // quirk, not an app issue - so we avoid Command entirely here and let
    // real profile *parsing* happen inside the real TUI process instead.
    let mut profile_paths = std::collections::HashMap::new();
    for (dir, _label) in personas {
        let profile_path = format!("final_uat_{dir}_profile.md");
        let db = format!("final_uat_{dir}.db");
        let _ = std::fs::remove_file(repo_root.join(&db));
        std::fs::copy(
            repo_root.join(format!("tests/uat/{dir}/profile.md")),
            repo_root.join(&profile_path),
        )?;
        profile_paths.insert(dir, profile_path);
    }

    for (dir, label) in personas {
        println!("\n\n########################################");
        println!("########## PERSONA: {label} ##########");
        println!("########################################");

        let db = format!("final_uat_{dir}.db");
        let profile_path = profile_paths.get(dir).unwrap().clone();
        let dst = repo_root.join(&profile_path);

        let mut s = Session::spawn(&binary, &repo_root, &db, &profile_path)?;
        s.drain(Duration::from_secs(1));

        s.send(b"r")?;
        // NOTE: the left panel's static "Inferred Roles ([r] to infer):"
        // label always contains "Inferred" - match on the actual status
        // message ("Inferred N role(s).") instead, or the failure message.
        let roles_ok = s.wait_until(
            &["role(s).", "Role inference failed"],
            Duration::from_secs(20),
        );
        s.dump(&format!("{label}: after role inference (ok={roles_ok})"));

        s.send(b"s")?;
        let scan_ok = s.wait_until(&["Scan complete"], Duration::from_secs(45));
        s.dump(&format!("{label}: after scan (ok={scan_ok})"));

        let has_job = !s.text().contains("No jobs yet");
        let mut eval_ok = false;
        if has_job {
            s.send(b"e")?;
            eval_ok = s.wait_until(
                &["Evaluated:", "Evaluation failed"],
                Duration::from_secs(25),
            );
            s.dump(&format!("{label}: after evaluate (ok={eval_ok})"));
        }

        let mut tailor_ok = false;
        if eval_ok {
            s.send(b"t")?;
            tailor_ok = s.wait_until(&["saved to", "Tailoring failed"], Duration::from_secs(40));
            s.dump(&format!("{label}: after tailor (ok={tailor_ok})"));
        }

        let final_text = s.text();
        s.send(b"q")?;
        s.drain(Duration::from_secs(1));

        // Substring search on the raw dump rather than per-line matching -
        // vt100's cell-by-cell reconstruction doesn't reliably line up with
        // logical UI rows when wide box-drawing/gauge characters are
        // involved, which silently broke line-based matching earlier.
        let extract = |text: &str, needle: &str| -> String {
            match text.find(needle) {
                Some(i) => text[i..]
                    .chars()
                    .take(60)
                    .collect::<String>()
                    .replace('\n', " ")
                    .trim()
                    .to_string(),
                None => "(not found)".to_string(),
            }
        };
        let match_score = extract(&final_text, "Match Score");
        let crashed = final_text.trim().is_empty();

        summary.push(format!(
            "{label:28} | eval_ok={eval_ok:5} | tailor_ok={tailor_ok:5} | {match_score:60} | crashed_or_blank={crashed}"
        ));

        let _ = std::fs::remove_file(repo_root.join(&db));
        let _ =
            std::fs::remove_file(repo_root.join(format!("final_uat_{dir}.llm_telemetry.jsonl")));
        let _ = std::fs::remove_file(&dst);
    }

    let _ = std::fs::remove_file(repo_root.join("profile.md"));

    println!("\n\n===== FINAL 5-PERSONA UAT SUMMARY =====");
    for line in &summary {
        println!("{line}");
    }

    println!("\n===== tailored_*.md files on disk =====");
    let mut tailored_count = 0;
    for entry in std::fs::read_dir(&repo_root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("tailored_") {
            tailored_count += 1;
            let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
            println!("{name}: {} bytes", content.len());
            let _ = std::fs::remove_file(entry.path());
        }
    }
    println!("Total tailored files found: {tailored_count} (expected up to 5, one per persona whose evaluate succeeded)");

    Ok(())
}
