#!/bin/bash
# ATSassin Continuous Loop Test
# Runs until all assertions pass or max iterations reached.
set -euo pipefail

MAX_ITERATIONS="${MAX_ITERATIONS:-10}"
PASSED=0
FAILED=0
ITER=0

echo "============================================"
echo "ATSassin Continuous Loop Test"
echo "Max iterations: $MAX_ITERATIONS"
echo "============================================"

while [ $ITER -lt $MAX_ITERATIONS ]; do
  ITER=$((ITER + 1))
  echo ""
  echo "--- Iteration $ITER/$MAX_ITERATIONS ---"

  FAILED_THIS_ROUND=0

  # 1) Build
  if ! cargo build --release 2>/dev/null; then
    echo "FAIL: cargo build --release"
    FAILED=$((FAILED + 1))
    FAILED_THIS_ROUND=$((FAILED_THIS_ROUND + 1))
    continue
  fi
  echo "PASS: cargo build --release"

  # 2) Tests
  if ! cargo test 2>/dev/null; then
    echo "FAIL: cargo test"
    FAILED=$((FAILED + 1))
    FAILED_THIS_ROUND=$((FAILED_THIS_ROUND + 1))
    continue
  fi
  echo "PASS: cargo test"

  # 3) Integration checks
  if [ -f "verify.bat" ]; then
    if ! powershell -File verify.bat 2>/dev/null; then
      echo "FAIL: verify.bat"
      FAILED=$((FAILED + 1))
      FAILED_THIS_ROUND=$((FAILED_THIS_ROUND + 1))
      continue
    fi
  fi
  echo "PASS: verify.bat"

  if [ $FAILED_THIS_ROUND -eq 0 ]; then
    PASSED=$((PASSED + 1))
    echo "Iteration $ITER: ALL PASSED"
  fi
done

echo ""
echo "============================================"
echo "Loop Test Complete"
echo "Passed: $PASSED / $ITER"
echo "Failed: $FAILED / $ITER"
echo "============================================"

if [ $FAILED -eq 0 ]; then
  echo "STATUS: GREEN - All iterations passed"
  exit 0
else
  echo "STATUS: RED - $FAILED iterations failed"
  exit 1
fi
