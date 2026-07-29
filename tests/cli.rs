// CLI-level (black-box) tests exercising the actual `atsassin` binary and
// public library functions. These close the coverage gap that let the
// pipeline-wiring, tailor-data-flow, and UTF-8 crash bugs ship undetected -
// the pre-existing test suite only ever exercised `engine`/`pipeline`
// internals directly, never the CLI layer in `src/cli.rs`.
//
// Every subprocess test runs in an isolated temp dir with an unreachable
// Ollama endpoint so LLM calls fail fast and deterministically, with no
// network dependency and no reliance on real API keys.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn uat_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/uat")
        .join(name)
}

fn atsassin_cmd(tempdir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("atsassin").expect("atsassin binary should build");
    cmd.current_dir(tempdir)
        .env("DATABASE_PATH", tempdir.join("test.db"))
        .env("PROFILE_PATH", tempdir.join("profile.md"))
        .env("LLM_PROVIDER", "ollama")
        .env("OLLAMA_BASE_URL", "http://127.0.0.1:9") // unreachable port -> fast, deterministic LLM failure
        .env_remove("GROQ_API_KEY")
        .env_remove("KIMI_API_KEY")
        .env_remove("LIGHTNING_API_KEY")
        .env_remove("GLM_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENROUTER_API_KEY");
    cmd
}

#[test]
fn pipeline_add_list_update_export_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();

    atsassin_cmd(tmp.path())
        .args([
            "pipeline",
            "add",
            "--job-id",
            "uat-demo-1",
            "--status",
            "new",
        ])
        .assert()
        .success();

    atsassin_cmd(tmp.path())
        .args(["pipeline", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uat-demo-1"))
        .stdout(predicate::str::contains("New"));

    atsassin_cmd(tmp.path())
        .args([
            "pipeline",
            "update",
            "--job-id",
            "uat-demo-1",
            "--status",
            "applied",
        ])
        .assert()
        .success();

    atsassin_cmd(tmp.path())
        .args(["pipeline", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Applied"));

    let csv_path = tmp.path().join("pipeline.csv");
    atsassin_cmd(tmp.path())
        .args(["pipeline", "export", "--output", csv_path.to_str().unwrap()])
        .assert()
        .success();

    let csv = std::fs::read_to_string(&csv_path).unwrap();
    assert!(csv.contains("uat-demo-1"));
    assert!(csv.contains("Applied"));
}

#[test]
fn pipeline_invalid_status_is_helpful() {
    let tmp = tempfile::tempdir().unwrap();
    atsassin_cmd(tmp.path())
        .args([
            "pipeline",
            "add",
            "--job-id",
            "uat-invalid",
            "--status",
            "banana",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Valid statuses"));
}

#[test]
fn pipeline_update_missing_job_fails() {
    let tmp = tempfile::tempdir().unwrap();
    atsassin_cmd(tmp.path())
        .args([
            "pipeline",
            "update",
            "--job-id",
            "does-not-exist",
            "--status",
            "applied",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No pipeline entry"));
}

#[test]
fn evaluate_utf16_file_does_not_utf8_crash() {
    let tmp = tempfile::tempdir().unwrap();
    let jd = fixture("sample_jd_utf16.txt");

    // The LLM call itself will fail (unreachable Ollama endpoint) so the
    // command may exit non-zero overall - what matters is that the file read
    // and job persistence succeed BEFORE that failure, proving the UTF-16
    // crash is gone and persistence happens ahead of the LLM call.
    atsassin_cmd(tmp.path())
        .args(["evaluate", "--file", jd.to_str().unwrap()])
        .assert()
        .stderr(predicate::str::contains("stream did not contain valid UTF-8").not())
        .stdout(predicate::str::contains("Job saved"));
}

#[test]
fn utf16_decode_helper() {
    use atsassin::engine::profile_parser::decode_text_lossy;

    let bytes = [0xFFu8, 0xFE, b'H', 0, b'i', 0];
    assert_eq!(decode_text_lossy(&bytes), "Hi");

    let raw = std::fs::read(fixture("sample_jd_utf16.txt")).unwrap();
    let decoded = decode_text_lossy(&raw);
    assert!(
        decoded.starts_with("Job Description"),
        "decoded content was: {decoded:?}"
    );
}

#[test]
fn parser_handles_uat_profile() {
    use atsassin::engine::profile_parser::ProfileParser;

    let text =
        std::fs::read_to_string(uat_fixture("scenario_1_synthetic_apac_gtm/profile.md")).unwrap();
    let profile = ProfileParser::profile_from_text(&text).unwrap();

    assert_eq!(
        profile.name, "Maya Kestrel",
        "name label should be stripped"
    );
    assert!(
        profile.skills.iter().any(|s| s.name == "Solution-oriented"),
        "hyphenated skill should survive intact, got: {:?}",
        profile.skills.iter().map(|s| &s.name).collect::<Vec<_>>()
    );
    assert!(profile
        .skills
        .iter()
        .any(|s| s.name == "Start-up Leadership"));
    assert_eq!(
        profile.experience.len(),
        16,
        "all 16 Experience: headers should be parsed, no cap"
    );
    assert!(
        profile
            .experience
            .iter()
            .any(|e| e.title == "Founder" && e.company == "Meridian GTM Studio"),
        "structured header parsing should split title/company correctly, got: {:?}",
        profile
            .experience
            .iter()
            .map(|e| (&e.title, &e.company))
            .collect::<Vec<_>>()
    );
}
