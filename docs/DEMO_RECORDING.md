# Recording the ATSassin demo

The `assets/demo.gif` in this repo is a **real screen recording** of the Windows binary running the commands shown in the README. It is not a synthetic animation.

## How the checked-in demo was made

We used **ffmpeg** to capture the desktop while a PowerShell script drove the CLI through:

```
atsassin profile init → scan → evaluate → tailor → tui
```

- **Recorder:** ffmpeg (`gdigrab` on Windows)
- **Driver script:** `scripts/capture_demo.ps1`
- **Command script:** `scripts/demo.ps1`
- **Profile/job data:** `demo_profile.md` and `demo_jd.txt` (fully synthetic, anonymised)
- **LLM:** local Ollama (`qwen2.5:0.5b`)

## Reproduce the demo

### Prerequisites

- Windows host with PowerShell
- `ffmpeg` in PATH
- A release build of `atsassin.exe` (`target/release/atsassin.exe`)
- Ollama running with `qwen2.5:0.5b` (or edit `scripts/demo.ps1` to set a different `OLLAMA_MODEL`)

### Record

```powershell
powershell -File scripts/capture_demo.ps1
```

This will:

1. Start ffmpeg capturing the desktop to `demo_capture.mkv`.
2. Open a maximised PowerShell window and run `scripts/demo.ps1`.
3. Stop ffmpeg after the demo finishes.
4. Convert the captured video to `assets/demo.gif`.

The real-time recording is about 70–75 seconds. If you need a shorter README-style clip, you can speed it up with ffmpeg (e.g. `setpts=0.25*PTS`) or trim to the final TUI section, but the checked-in demo runs at real speed so the LLM calls are honest.

## PII policy

- Only use synthetic data in public demos.
- The included `demo_profile.md` uses a fictional candidate and fake email/company/location.
- Do not upload recordings that contain your own resume, email, or job history.

## Alternative: VHS (Linux/WSL/macOS)

If you prefer a Linux-style terminal recording, `scripts/demo_vhs.tape` and `scripts/Dockerfile.vhs` are kept as references. They require VHS, ttyd, a Linux binary, and Ollama to be reachable from the recording container.
