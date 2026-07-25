# ATSassin Continuous Loop Test
# Runs until all assertions pass or max iterations reached.
$ErrorActionPreference = 'Stop'
$MAX_ITERATIONS = 3
$PASSED = 0
$FAILED = 0
$ITER = 0

Write-Host "============================================"
Write-Host "ATSassin Continuous Loop Test"
Write-Host "Max iterations: $MAX_ITERATIONS"
Write-Host "============================================"

while ($ITER -lt $MAX_ITERATIONS) {
    $ITER++
    Write-Host ""
    Write-Host "--- Iteration $ITER/$MAX_ITERATIONS ---"
    $FAILED_THIS_ROUND = 0

    # 1) Build
    $buildOutput = cmd /c "cargo build --release 2>&1" | Out-String
    if (-not (Test-Path "target/release/atsassin.exe")) {
        Write-Host "FAIL: cargo build --release"
        $FAILED++
        $FAILED_THIS_ROUND++
        continue
    }
    Write-Host "PASS: cargo build --release"

    # 2) Tests
    $testOutput = cmd /c "cargo test 2>&1" | Out-String
    if ($testOutput -match "test result: FAILED") {
        Write-Host "FAIL: cargo test"
        $FAILED++
        $FAILED_THIS_ROUND++
        continue
    }
    Write-Host "PASS: cargo test"

    # 3) Integration checks
    if (Test-Path "verify.bat") {
        $verifyExit = 0
        $verifyOutput = cmd /c "verify.bat 2>&1" | Out-String
        $verifyExit = $LASTEXITCODE
        if ($verifyExit -ne 0) {
            Write-Host "FAIL: verify.bat (exit $verifyExit)"
            $FAILED++
            $FAILED_THIS_ROUND++
            continue
        }
        Write-Host "PASS: verify.bat"
    }

    if ($FAILED_THIS_ROUND -eq 0) {
        $PASSED++
        Write-Host "Iteration ${ITER}: ALL PASSED"
    }
}

Write-Host ""
Write-Host "============================================"
Write-Host "Loop Test Complete"
Write-Host "Passed: $PASSED / $ITER"
Write-Host "Failed: $FAILED / $ITER"
Write-Host "============================================"

if ($FAILED -eq 0) {
    Write-Host "STATUS: GREEN - All iterations passed"
    exit 0
} else {
    Write-Host "STATUS: RED - $FAILED iterations failed"
    exit 1
}
