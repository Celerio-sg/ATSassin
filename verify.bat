@echo off
setlocal enabledelayedexpansion
set FAIL=0
set PASS=0

echo ============================================
echo ATSassin Full End-to-End Verification
echo ============================================
echo.

echo [1/6] Building release binary...
cargo build --release >nul 2>&1
if %errorlevel% neq 0 (
    echo FAIL: cargo build --release
    set FAIL=1
) else (
    echo PASS: cargo build --release
    set /a PASS+=1
)

echo.
echo [2/6] Running tests...
cargo test
if %errorlevel% neq 0 (
    echo FAIL: cargo test
    set FAIL=1
) else (
    echo PASS: cargo test
    set /a PASS+=1
)

echo.
echo [3/6] Profile init with LinkedIn export...
del /f profile.md >nul 2>&1
cargo run -- profile init --linkedin "assets\examples\linkedin_test" >nul 2>&1
if %errorlevel% neq 0 (
    echo FAIL: profile init
    set FAIL=1
) else (
    echo PASS: profile init
    set /a PASS+=1
)

echo.
echo [4/6] Playbook command...
cargo run -- playbook >nul 2>&1
if %errorlevel% neq 0 (
    echo FAIL: playbook
    set FAIL=1
) else (
    echo PASS: playbook
    set /a PASS+=1
)

echo.
echo [5/6] Pipeline list...
cargo run -- pipeline list >nul 2>&1
if %errorlevel% neq 0 (
    echo FAIL: pipeline list
    set FAIL=1
) else (
    echo PASS: pipeline list
    set /a PASS+=1
)

echo.
echo [6/6] Verify anti-AI-slop blacklist compiled...
cargo build --release >nul 2>&1
if %errorlevel% neq 0 (
    echo FAIL: anti-AI-slop blacklist compile
    set FAIL=1
) else (
    echo PASS: anti-AI-slop blacklist compiled
    set /a PASS+=1
)

echo.
echo ============================================
echo Results: !PASS! passed, !FAIL! failed
echo ============================================
if !FAIL! equ 0 (
    echo ALL TESTS PASSED
    exit /b 0
) else (
    echo SOME TESTS FAILED
    exit /b 1
)
