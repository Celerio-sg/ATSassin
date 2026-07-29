#!/usr/bin/env python3
"""Mechanical consistency gate for the documentation set.

Catches the class of error that repeatedly slipped past human and LLM review
during the 2026-07-29 architecture work: pointers that resolve to the wrong
thing, or to nothing.

Every check here corresponds to a defect that actually shipped:

  - a design doc claimed "#119 survives only as the prior table" while #119 was
    entirely a salary dataset          -> CHECK: cited issues exist and are open
  - CONTRIBUTING routed contributors to a plan for rejected work
                                       -> CHECK: no active doc cites a rejected issue
  - the board review pointed at four closed issues as if actionable
                                       -> CHECK: closed issues carry closure context
  - binary size was claimed as 8.14 MB and ~9.5 MB in the same document
                                       -> CHECK: measured facts match reality
  - relative links broke when files moved into docs/design/
                                       -> CHECK: every relative link resolves

Run offline (no network) by default; pass --with-tracker to validate issue
references against GitHub, which requires `gh` to be authenticated.
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Docs that describe current direction. A stale pointer in one of these sends a
# contributor down a dead end, so they are held to a higher standard than the
# point-in-time reports.
ACTIVE = [
    "README.md",
    "CONTRIBUTING.md",
    "docs/INFLECTION_ARCHITECTURE.md",
    "docs/DECISIONS.md",
    "docs/ROADMAP.md",
    "docs/TEST_STRATEGY.md",
    "docs/VENTURE_BOARD_REVIEW.md",
    "docs/design/EVIDENCE_LAYER.md",
    "docs/design/CALIBRATION_LAYER.md",
    "docs/design/ALLOCATION_LAYER.md",
]

# Issues closed as rejected. Citing one as live work is always a bug.
REJECTED = {49, 105}

# Phrases that make a reference to a closed issue legitimate.
CLOSURE_WORDS = (
    "closed", "reject", "duplicate", "supersed", "prematurely",
    "history", "historical", "rej-", "deferred", "moot", "stale",
)

failures: list[str] = []
warnings: list[str] = []


def fail(msg: str) -> None:
    failures.append(msg)


def warn(msg: str) -> None:
    warnings.append(msg)


def check_links() -> None:
    """Every relative markdown link must resolve to a file that exists."""
    for rel in ACTIVE:
        path = ROOT / rel
        if not path.exists():
            fail(f"{rel}: listed as an active doc but does not exist")
            continue
        text = path.read_text(encoding="utf-8")
        for match in re.finditer(r"\[[^\]]*\]\(([^)#][^)]*\.md)(#[^)]*)?\)", text):
            target = (path.parent / match.group(1)).resolve()
            if not target.exists():
                fail(f"{rel}: broken link -> {match.group(1)}")


def check_measured_facts() -> None:
    """Numbers asserted in docs must match what the repo actually contains.

    These drifted silently before: the binary was documented at two different
    sizes, neither correct, and the company count was understated by nine.
    """
    directory = ROOT / "src/pipeline/company_directory.rs"
    if directory.exists():
        src = directory.read_text(encoding="utf-8", errors="replace")
        actual = len(re.findall(r'\(\s*"[^"]+"\s*,\s*"[^"]+"\s*\)', src))
        if actual:
            for rel in ("docs/ROADMAP.md", "docs/VENTURE_BOARD_REVIEW.md"):
                text = (ROOT / rel).read_text(encoding="utf-8")
                for claim in re.findall(r"(?:sweep across|swept|directory of|×)\s*~?(\d+)\s+(?:real\s+)?compan(?:y|ies)", text):
                    if abs(int(claim) - actual) > 2:
                        warn(f"{rel}: claims {claim} companies; directory has ~{actual}")

    # The binary size claim must match the built artifact when one exists.
    binary = ROOT / "target/release/atsassin.exe"
    if not binary.exists():
        binary = ROOT / "target/release/atsassin"
    if binary.exists():
        mb = binary.stat().st_size / 1048576
        for rel in ACTIVE:
            path = ROOT / rel
            if not path.exists():
                continue
            for line in path.read_text(encoding="utf-8").splitlines():
                low = line.lower()
                # Lines that document a corrected/superseded figure are not claims.
                if any(w in low for w in ("was ", "claimed", "previously", "stale",
                                          "corrected", "incorrect", "wrong")):
                    continue
                # Only lines actually talking about the binary are size claims.
                if not any(w in low for w in ("binary", "release build", "artifact")):
                    continue
                for claim in re.findall(r"(\d+\.\d+)\s*MB", line):
                    if abs(float(claim) - mb) > 0.75:
                        warn(f"{rel}: claims {claim} MB; built binary is {mb:.2f} MB")


def check_no_fabrication_patterns() -> None:
    """ADR-002: no timestamp may be used as a substitute for missing data.

    The existing violations are tracked in #144. This gate exists to stop the
    count *growing* while that issue is open -- a new one is a regression even
    though the old ones are known.
    """
    hits: list[str] = []
    for src in (ROOT / "src").rglob("*.rs"):
        text = src.read_text(encoding="utf-8", errors="replace")
        for num, line in enumerate(text.splitlines(), 1):
            if "#[cfg(test)]" in line or line.strip().startswith("//"):
                continue
            if re.search(r"posted_at:\s*Some\(\s*(chrono::)?Utc::now\(\)", line):
                hits.append(f"{src.relative_to(ROOT).as_posix()}:{num}")
    _finish_fabrication(hits)


BASELINE = ROOT / "scripts/.docs_baseline.json"


def _finish_fabrication(hits: list[str]) -> None:
    known = set()
    if BASELINE.exists():
        known = set(json.loads(BASELINE.read_text())["fabricated_posted_at"])
    for h in sorted(set(hits) - known):
        fail(f"{h}: NEW fabricated posting date (ADR-002) -- use None")
    stale = known - set(hits)
    if stale:
        warn(f"{len(stale)} baselined fabrication site(s) now fixed -- "
             f"regenerate scripts/.docs_baseline.json to tighten the gate")
    if hits:
        warn(f"{len(hits)} known fabricated posting date(s) remain (tracked in #144)")


def check_tracker(issues: dict[int, dict]) -> None:
    """Issue references in active docs must resolve, and must not be stale."""
    for rel in ACTIVE:
        path = ROOT / rel
        if not path.exists():
            continue
        for num, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            # Skip markdown anchors like (#6-design-dimensions) and code spans.
            scrubbed = re.sub(r"\(#[\w-]+\)", "", line)          # markdown anchors
            scrubbed = re.sub(r"`[^`]*`", "", scrubbed)            # code spans
            scrubbed = re.sub(r'"[^"]*"', "", scrubbed)           # quoted examples
            if re.search(r"e\.g\.|for example|such as", line, re.I):
                continue
            for match in re.finditer(r"#(\d{1,4})\b", scrubbed):
                n = int(match.group(1))
                if n < 40:  # older numbers collide with section and finding numbering
                    continue
                issue = issues.get(n)
                if issue is None:
                    fail(f"{rel}:{num}: cites #{n}, which does not exist")
                    continue
                contextual = any(w in line.lower() for w in CLOSURE_WORDS)
                if n in REJECTED and not contextual:
                    fail(
                        f"{rel}:{num}: cites rejected issue #{n} as live work "
                        f"(see REJ-001)"
                    )
                elif issue["state"] == "CLOSED" and not contextual:
                    fail(f"{rel}:{num}: cites closed #{n} without closure context")


def load_issues() -> dict[int, dict]:
    out = subprocess.run(
        ["gh", "issue", "list", "--state", "all", "--limit", "500",
         "--json", "number,state,title"],
        capture_output=True, text=True, cwd=ROOT, check=True,
    )
    return {i["number"]: i for i in json.loads(out.stdout)}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--with-tracker", action="store_true",
                    help="validate issue references against GitHub (needs gh)")
    args = ap.parse_args()

    check_links()
    check_measured_facts()
    check_no_fabrication_patterns()

    if args.with_tracker:
        try:
            check_tracker(load_issues())
        except Exception as exc:  # noqa: BLE001 - network/auth failures are not doc bugs
            warn(f"tracker check skipped: {exc}")

    for w in warnings:
        print(f"WARN  {w}")
    for f in failures:
        print(f"FAIL  {f}")

    if failures:
        print(f"\n{len(failures)} failure(s), {len(warnings)} warning(s)")
        return 1
    print(f"docs consistency: OK ({len(warnings)} warning(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
