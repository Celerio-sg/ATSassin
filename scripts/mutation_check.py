#!/usr/bin/env python3
"""Mutation check for the mathematical harness.

A harness that only passes proves nothing. This reintroduces each defect that
has actually shipped in the Layer 2/3 specs and asserts the tests catch it.
Every mutant below corresponds to a bug that was live in this repository.

Round 3 found the harness itself had two blind spots: an alpha<->beta
transposition passed all nine calibration tests while reporting a callback rate
of 80-97% instead of 3-20%, and a bad decay fit returning 1.0 passed all fifteen
allocation tests while making the allocator age-blind. An earlier
"mutation-verified" claim in ALLOCATION_LAYER.md was not reproducible, because
the mutants existed nowhere. They exist here now.

Usage:  python scripts/mutation_check.py
Exit 0 = every mutant caught. Exit 1 = a mutant survived, so the harness has a
blind spot and any "mutation-verified" claim in the docs is false.
"""

from __future__ import annotations

import io
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOC = ROOT / "src/engine/allocation.rs"
CALIB = ROOT / "src/engine/calibration.rs"

# (file, cargo test filter, label, old, new)
MUTANTS: list[tuple[Path, str, str, str, str]] = [
    (
        ALLOC, "allocation", "cost inversion (age direction)",
        "    Some(1.0 - p_callback.clamp(0.0, 1.0) * decay.clamp(0.0, 1.0))",
        "    Some(-(p_callback.max(1e-9).ln()) * decay)",
    ),
    (
        ALLOC, "allocation", "p_min=0 tie selects worthless postings",
        "if value <= params.p_min {",
        "if value < params.p_min {",
    ),
    (
        ALLOC, "allocation", "bad decay fit treated as maximally fresh",
        "    if half_life_days.is_nan() || half_life_days <= 0.0 {\n        return None;\n    }",
        "    if half_life_days.is_nan() || half_life_days <= 0.0 {\n        return Some(1.0);\n    }",
    ),
    (
        ALLOC, "allocation", "family cap not enforced",
        "if used >= params.family_cap {",
        "if false {",
    ),
    (
        ALLOC, "allocation", "non-deterministic tie-break",
        ".then_with(|| a.0.id.cmp(&b.0.id))",
        ".then(std::cmp::Ordering::Equal)",
    ),
    (
        ALLOC, "allocation", "objective maximises product not sum",
        "        expected += value;",
        "        expected += value.max(1e-12).ln();",
    ),
    (
        ALLOC, "allocation", "NaN inputs no longer rejected",
        "    if p_callback.is_nan() || decay.is_nan() {",
        "    if false && (p_callback.is_nan() || decay.is_nan()) {",
    ),
    (
        CALIB, "calibration", "alpha<->beta transposition (reports 80% not 3%)",
        "pub fn beta_cdf(a: f64, b: f64, x: f64) -> f64 {",
        "pub fn beta_cdf(b: f64, a: f64, x: f64) -> f64 {",
    ),
    (
        CALIB, "calibration", "shrinkage direction inverted",
        "    strength / (strength + n as f64)",
        "    n as f64 / (strength + n as f64)",
    ),
    (
        CALIB, "calibration", "prior_dominated threshold flipped",
        "    w > 0.5\n}",
        "    w < 0.5\n}",
    ),
    (
        CALIB, "calibration", "zero data reported as personal",
        "    if n == 0 {\n        return true;\n    }",
        "    if n == 0 {\n        return false;\n    }",
    ),
]


def failures_for(test_filter: str) -> int:
    """Run the suite and return how many tests failed."""
    proc = subprocess.run(
        ["cargo", "test", "--lib", test_filter],
        cwd=ROOT, capture_output=True, text=True,
        encoding="utf-8", errors="replace",
    )
    for line in proc.stdout.splitlines():
        if line.startswith("test result:"):
            for part in line.split(";"):
                if "failed" in part:
                    return int(part.strip().split()[0])
    return 0


def main() -> int:
    tmp = Path(tempfile.mkdtemp())
    originals = {p: tmp / p.name for p in (ALLOC, CALIB)}
    for src, dst in originals.items():
        shutil.copy(src, dst)

    survived: list[str] = []
    skipped: list[str] = []
    try:
        # Baseline must be green, or "caught" means nothing.
        for f in ("allocation", "calibration"):
            if failures_for(f) != 0:
                print(f"BASELINE IS RED for {f} - fix that before mutating.")
                return 1
        print("Baseline green.\n")

        current = None
        for path, test_filter, label, old, new in MUTANTS:
            if current != test_filter:
                current = test_filter
                print(f"{test_filter.capitalize()} mutants:")
            text = io.open(path, encoding="utf-8").read()
            if old not in text:
                print(f"  {label:<52} SKIPPED (pattern drifted)")
                skipped.append(label)
                continue
            io.open(path, "w", encoding="utf-8").write(text.replace(old, new, 1))
            n = failures_for(test_filter)
            shutil.copy(originals[path], path)
            if n == 0:
                print(f"  {label:<52} SURVIVED  <-- blind spot")
                survived.append(label)
            else:
                print(f"  {label:<52} caught ({n} test{'s' if n != 1 else ''})")
    finally:
        for src, dst in originals.items():
            shutil.copy(dst, src)
        shutil.rmtree(tmp, ignore_errors=True)

    print()
    if survived:
        print(f"{len(survived)} mutant(s) SURVIVED - the harness has a blind spot:")
        for s in survived:
            print(f"  - {s}")
        print("Any 'mutation-verified' claim in the design docs is false until fixed.")
        return 1
    if skipped:
        print(f"{len(skipped)} mutant pattern(s) drifted from the source - update this")
        print("script so the claim stays reproducible:")
        for s in skipped:
            print(f"  - {s}")
        return 1
    print(f"All {len(MUTANTS)} mutants caught. The harness has no known blind spot.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
