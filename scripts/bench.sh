#!/bin/bash
# ATSassin Champion-Challenger Benchmark
# Runs comparable tasks across ATSassin and installed competitor repos.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
BENCH_DIR="$REPO_ROOT/benchmarks"
RESULTS_DIR="$REPO_ROOT/benchmarks/results"
mkdir -p "$RESULTS_DIR"

echo "============================================"
echo "ATSassin Champion-Challenger Benchmark"
echo "============================================"

# Sample inputs
SAMPLE_RESUME="$REPO_ROOT/tests/fixtures/sample_resume.txt"
SAMPLE_JD="$REPO_ROOT/tests/fixtures/sample_jd.txt"
if [ ! -f "$SAMPLE_RESUME" ]; then
  echo "This is a sample resume for benchmarking purposes." > "$SAMPLE_RESUME"
  echo "Skills: Rust, Python, LLMs, distributed systems, product management." >> "$SAMPLE_RESUME"
fi
if [ ! -f "$SAMPLE_JD" ]; then
  echo "Job Description: Senior Product Manager, APAC region, SaaS, B2B." > "$SAMPLE_JD"
fi

# 1) ATSassin
echo "[1/3] Benchmarking ATSassin..."
if command -v cargo &> /dev/null; then
  ATSASSIN_START=$(date +%s%N)
  cargo run --release -- evaluate --job-id dummy 2>/dev/null || true
  ATSASSIN_END=$(date +%s%N)
  ATSASSIN_MS=$(( (ATSASSIN_END - ATSASSIN_START) / 1000000 ))
  echo "ATSassin evaluate: ${ATSASSIN_MS}ms" | tee "$RESULTS_DIR/atsassin_timing.txt"
else
  echo "cargo not found, skipping ATSassin benchmark"
  ATSASSIN_MS="N/A"
fi

# 2) Resume-Matcher (Python)
echo "[2/3] Benchmarking Resume-Matcher..."
if [ -d "$BENCH_DIR/Resume-Matcher" ]; then
  pushd "$BENCH_DIR/Resume-Matcher" >/dev/null
  if [ -f "requirements.txt" ]; then
    python -m pip install -q -r requirements.txt 2>/dev/null || true
  fi
  RM_START=$(date +%s%N)
  python -m apps.backend.app.main --help 2>/dev/null || true
  RM_END=$(date +%s%N)
  RM_MS=$(( (RM_END - RM_START) / 1000000 ))
  echo "Resume-Matcher startup: ${RM_MS}ms" | tee "$RESULTS_DIR/resume_matcher_timing.txt"
  popd >/dev/null
else
  echo "Resume-Matcher not found"
  RM_MS="N/A"
fi

# 3) career-ops
echo "[3/3] Benchmarking career-ops..."
if [ -d "$BENCH_DIR/career-ops" ]; then
  pushd "$BENCH_DIR/career-ops" >/dev/null
  if [ -f "package.json" ]; then
    npm install --silent 2>/dev/null || true
  fi
  CO_START=$(date +%s%N)
  node doctor.mjs --json 2>/dev/null || true
  CO_END=$(date +%s%N)
  CO_MS=$(( (CO_END - CO_START) / 1000000 ))
  echo "career-ops doctor: ${CO_MS}ms" | tee "$RESULTS_DIR/career_ops_timing.txt"
  popd >/dev/null
else
  echo "career-ops not found"
  CO_MS="N/A"
fi

# Summary
cat > "$RESULTS_DIR/summary.md" <<EOF
# Champion-Challenger Benchmark Results

| Tool | Startup/Cold | Notes |
|------|-------------|-------|
| ATSassin | ${ATSASSIN_MS}ms | Rust release binary |
| Resume-Matcher | ${RM_MS}ms | Python backend startup |
| career-ops | ${CO_MS}ms | Node.js doctor check |

## Methodology
- Timed cold start / help invocation
- All tools run on same machine
- No warm cache for Rust release binary
EOF

echo ""
echo "============================================"
echo "Benchmark complete. Results in $RESULTS_DIR"
echo "============================================"
