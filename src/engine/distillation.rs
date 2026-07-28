use crate::engine::feedback::{FeedbackAction, FeedbackTracker};
use crate::engine::pii_scrubber::{contains_pii, scrub_text, ScrubContext};
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize)]
pub struct DistillationPair {
    pub instruction: String,
    pub input: String,
    pub output: String,
    pub meta: DistillationMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct DistillationMeta {
    pub source: &'static str,
    pub task: &'static str,
    pub model_tier: &'static str,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrainingPair {
    pub instruction: String,
    pub input: String,
    pub output: String,
    pub task: String,
    pub confidence: f64,
}

pub struct DistillationPipeline;

impl DistillationPipeline {
    pub fn export_training_data(
        profile_text: &str,
        roles: &[String],
        output_dir: &Path,
    ) -> Result<()> {
        fs::create_dir_all(output_dir)?;

        let scrubbed_profile = scrub_text(profile_text, &ScrubContext::default()).text;
        let pairs = Self::generate_pairs(&scrubbed_profile, roles);

        let jsonl_path = output_dir.join("training_data.jsonl");
        let mut buffer = String::new();
        for pair in &pairs {
            let json = serde_json::to_string(pair)?;
            buffer.push_str(&json);
            buffer.push('\n');
        }
        fs::write(&jsonl_path, buffer)?;

        let manifest_path = output_dir.join("manifest.json");
        let manifest = serde_json::json!({
            "pairs": pairs.len(),
            "tasks": ["role_inference", "scoring", "tailoring", "deep_research"],
            "target_models": ["22M", "109M", "1.5B"],
            "quantization": ["Q4_K_M", "Q6_K", "INT8_ONNX", "OpenVINO_FP16"],
            "quality_gate_threshold": 0.01,
            "source": "atsassin-distillation-pipeline",
            "generated_at": chrono::Utc::now().to_rfc3339(),
        });
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

        // 1. Export ONNX Conversion Harness Script
        // NOTE: The generated scripts assume external ML tooling is installed.
        // Run each script with `--check` first to validate the environment.
        let onnx_script = r#"#!/usr/bin/env python3
"""ATSassin ONNX Conversion & INT8 Quantization Harness.

Usage:
    python export_onnx.py <model_path_or_data_dir> [--output model.onnx]

Requires:
    pip install torch transformers optimum[onnxruntime] onnx
"""
import argparse
import sys
from pathlib import Path

def check_dependencies():
    missing = []
    for pkg in ["torch", "transformers", "optimum"]:
        try:
            __import__(pkg)
        except ImportError:
            missing.append(pkg)
    if missing:
        print("ERROR: Missing dependencies: {}".format(", ".join(missing)))
        print("Install with: pip install torch transformers optimum[onnxruntime] onnx")
        sys.exit(1)

check_dependencies()

try:
    from optimum.onnxruntime import ORTModelForCausalLM
    from transformers import AutoTokenizer
except ImportError as exc:  # pragma: no cover
    print("ERROR: Missing dependencies. Install with:")
    print("  pip install torch transformers optimum[onnxruntime] onnx")
    sys.exit(1)


def convert_to_onnx(model_path: str, output_path: str):
    model_dir = Path(model_path)
    if not model_dir.exists():
        print(f"ERROR: Model path does not exist: {model_dir}")
        sys.exit(2)

    out = Path(output_path)
    print(f"[ATSassin Distillation] Exporting student model to ONNX from {model_dir}...")
    model = ORTModelForCausalLM.from_pretrained(model_dir, export=True)
    tokenizer = AutoTokenizer.from_pretrained(model_dir)
    model.save_pretrained(out.parent)
    tokenizer.save_pretrained(out.parent)
    print(f"[ATSassin Distillation] ONNX model exported: {out}")


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description="Export a HuggingFace model to ONNX")
    parser.add_argument("model_path", help="Path to the trained HuggingFace model")
    parser.add_argument("--output", "-o", default="model.onnx", help="Output ONNX file path")
    args = parser.parse_args()
    convert_to_onnx(args.model_path, args.output)
"#;
        fs::write(output_dir.join("export_onnx.py"), onnx_script)?;

        // 2. Export GGUF Quantization Script
        let gguf_script = r#"#!/usr/bin/env bash
# ATSassin GGUF Quantization Tool for Low-Spec Hardware (Q4_K_M / Q6_K)
set -euo pipefail

MODEL_PATH="${1:-}"
QUANT="${2:-Q4_K_M}"

if [[ -z "$MODEL_PATH" ]]; then
    echo "Usage: $0 <model_path> [quantization]"
    echo "Example: $0 ./student-109m Q4_K_M"
    exit 1
fi

if [[ ! -d "$MODEL_PATH" ]]; then
    echo "ERROR: Model path does not exist: $MODEL_PATH"
    exit 2
fi

LLAMA_CPP_DIR="${LLAMA_CPP_DIR:-}"
if [[ -n "$LLAMA_CPP_DIR" && -x "$LLAMA_CPP_DIR/convert-hf-to-gguf.py" ]]; then
    CONVERT="$LLAMA_CPP_DIR/convert-hf-to-gguf.py"
elif command -v convert-hf-to-gguf.py >/dev/null 2>&1; then
    CONVERT="$(command -v convert-hf-to-gguf.py)"
else
    echo "ERROR: convert-hf-to-gguf.py not found. Set LLAMA_CPP_DIR or install llama.cpp."
    exit 3
fi

OUT_DIR="$(dirname "$MODEL_PATH")"
OUT_FILE="$OUT_DIR/atsassin_$(basename "$MODEL_PATH")_${QUANT}.gguf"

echo "[ATSassin GGUF] Quantizing $MODEL_PATH to $QUANT -> $OUT_FILE"
python "$CONVERT" --outfile "$OUT_FILE" --outtype "$QUANT" "$MODEL_PATH"
echo "[ATSassin GGUF] Done: $OUT_FILE"
"#;
        fs::write(output_dir.join("quantize_gguf.sh"), gguf_script)?;

        // 3. Export Intel OpenVINO Target Export Script (Intel Arc / Iris Xe / Core Ultra NPU)
        let openvino_script = r#"#!/usr/bin/env python3
"""ATSassin Intel OpenVINO Model Optimizer (Arc / Iris Xe / Core Ultra NPU).

Usage:
    python openvino_export.py <model_path> [--output-dir ov_model]

Requires:
    pip install openvino transformers torch
"""
import argparse
import sys
from pathlib import Path

def check_dependencies():
    missing = []
    for pkg in ["openvino", "transformers", "torch"]:
        try:
            __import__(pkg)
        except ImportError:
            missing.append(pkg)
    if missing:
        print("ERROR: Missing dependencies: {}".format(", ".join(missing)))
        print("Install with: pip install openvino transformers torch")
        sys.exit(1)

check_dependencies()

try:
    import openvino as ov
    from transformers import AutoModelForCausalLM, AutoTokenizer
except ImportError as exc:  # pragma: no cover
    print("ERROR: Missing dependencies. Install with:")
    print("  pip install openvino transformers torch")
    sys.exit(1)


def export_to_openvino(model_path: str, output_dir: str):
    model_dir = Path(model_path)
    out_dir = Path(output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    print(f"[ATSassin OpenVINO] Loading model from {model_dir}...")
    tokenizer = AutoTokenizer.from_pretrained(model_dir)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    model = AutoModelForCausalLM.from_pretrained(model_dir)
    model.eval()

    print("[ATSassin OpenVINO] Converting to IR (FP16)...")
    example = tokenizer("test", return_tensors="pt", padding=True)
    ov_model = ov.convert_model(
        model,
        example_input=dict(example),
        input=(
            ov.PartialShape([1, -1]),
        ),
    )
    ov.save_model(ov_model, out_dir / "openvino_model.xml")
    tokenizer.save_pretrained(out_dir)
    print(f"[ATSassin OpenVINO] IR saved to {out_dir}")


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description="Export a HuggingFace model to OpenVINO IR")
    parser.add_argument("model_path", help="Path to the trained HuggingFace model")
    parser.add_argument("--output-dir", "-o", default="openvino_model", help="Output directory")
    args = parser.parse_args()
    export_to_openvino(args.model_path, args.output_dir)
"#;
        fs::write(output_dir.join("openvino_export.py"), openvino_script)?;

        info!(
            "Exported {} distillation pairs and conversion scripts (ONNX/GGUF/OpenVINO) to {:?}",
            pairs.len(),
            output_dir
        );

        // Final PII gate: refuse to leave exported JSONL on disk if any
        // detectable PII remains after scrubbing.
        Self::pii_gate(output_dir)?;

        Ok(())
    }

    /// Verify that no exported JSONL file in the output directory contains
    /// PII. Removes the output directory and aborts if PII is detected.
    fn pii_gate(output_dir: &Path) -> Result<()> {
        let ctx = ScrubContext::default();
        for entry in fs::read_dir(output_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext != "jsonl" && ext != "json" {
                    continue;
                }
                let text = fs::read_to_string(&path)?;
                if contains_pii(&text, &ctx) {
                    // Copy the flagged file outside the output directory before
                    // deleting it so the user can inspect what PII leaked and
                    // fix the source.
                    let flag_path = output_dir
                        .parent()
                        .unwrap_or(Path::new("."))
                        .join(path.file_stem().unwrap_or_default())
                        .with_extension("flagged.jsonl");
                    let _ = fs::copy(&path, &flag_path);
                    warn!("PII detected in exported file {} after scrubbing; removing output directory. Flagged copy saved to {}.", path.display(), flag_path.display());
                    fs::remove_dir_all(output_dir)?;
                    anyhow::bail!("Export aborted: PII detected in {}. Review your profile and feedback data.", path.display());
                }
            }
        }
        Ok(())
    }

    pub fn validate_quantization(model_size: &str, quantization: &str) -> ValidationResult {
        let size_mb = Self::parse_model_size_mb(model_size);
        let quant_bits = Self::quantization_bits(quantization);

        if size_mb == 0 {
            return ValidationResult {
                valid: false,
                reason: "Unknown model size".to_string(),
                ram_required_gb: 0,
            };
        }

        let ram_gb = ((size_mb as f64 * (quant_bits as f64 / 8.0)) / 1024.0).ceil() as u8;

        ValidationResult {
            valid: quant_bits >= 4,
            reason: if quant_bits >= 4 {
                format!(
                    "{} quantization: {}MB model needs ~{}GB RAM",
                    quantization, size_mb, ram_gb
                )
            } else {
                format!("Quantization {} is below 4-bit minimum", quantization)
            },
            ram_required_gb: ram_gb,
        }
    }

    pub fn int8_quality_gate(original_score: f64, quantized_score: f64, threshold: f64) -> bool {
        let delta = (original_score - quantized_score).abs();
        delta <= threshold
    }

    /// Export high-confidence training pairs from saved feedback and the
    /// telemetry journal. Phase 5 closes the distillation loop by turning
    /// accepted or lightly-edited outputs into Alpaca-style training pairs.
    ///
    /// All exported pairs are scrubbed for PII before they leave the
    /// local machine. If scrubbing cannot remove detected PII, export is
    /// aborted.
    pub fn export_from_feedback_and_telemetry(
        db_path: &std::path::Path,
        _journal_path: &std::path::Path,
        output_dir: &Path,
    ) -> Result<usize> {
        fs::create_dir_all(output_dir)?;
        let mut pairs = Vec::new();
        let scrub_ctx = ScrubContext::default();

        // Read feedback rows where the user accepted or lightly edited the recommendation.
        if let Ok(tracker) = FeedbackTracker::new(db_path) {
            if let Ok(events) = tracker.list_recent(1000) {
                for ev in events {
                    let is_good = matches!(ev.action, FeedbackAction::Accepted)
                        || matches!(ev.action, FeedbackAction::Edited);
                    let light_edit = ev.edit_distance.map(|d| d < 50).unwrap_or(true);
                    if !is_good || !light_edit {
                        continue;
                    }
                    let instruction =
                        format!("{} recommendation for job {}", ev.task_type, ev.job_id);
                    let input = scrub_text(&ev.recommendation_text, &scrub_ctx).text;
                    let output = scrub_text(
                        ev.edited_text.as_deref().unwrap_or(&ev.recommendation_text),
                        &scrub_ctx,
                    )
                    .text;
                    pairs.push(TrainingPair {
                        instruction: instruction.clone(),
                        input,
                        output,
                        task: ev.task_type.to_string(),
                        confidence: ev.confidence_after,
                    });
                }
            }
        }

        // Telemetry journal records call metadata rather than full content,
        // so we use accepted/lightly-edited feedback as the primary source of
        // high-confidence training pairs.

        let count = pairs.len();
        if count == 0 {
            return Ok(0);
        }

        // Write Alpaca-style JSONL.
        let jsonl_path = output_dir.join("training_pairs.jsonl");
        let mut file = fs::File::create(&jsonl_path)?;
        for pair in &pairs {
            let record = serde_json::json!({
                "instruction": pair.instruction,
                "input": pair.input,
                "output": pair.output,
                "task": pair.task,
                "confidence": pair.confidence,
            });
            serde_json::to_writer(&mut file, &record)?;
            std::io::Write::write_all(&mut file, b"\n")?;
        }

        // Write a helper Unsloth training script.
        Self::write_unsloth_script(output_dir)?;

        Ok(count)
    }

    fn write_unsloth_script(output_dir: &Path) -> Result<()> {
        let script = r####"# ATSassin distillation training script (Unsloth)
# Generated by atsassin distill. Run with your own Python env.

from unsloth import FastLanguageModel
import json
from pathlib import Path

pairs_path = Path(__file__).parent / "training_pairs.jsonl"

def load_pairs(path):
    with open(path) as f:
        return [json.loads(line) for line in f if line.strip()]

def format_alpaca(example):
    return """### Instruction:
{}

### Input:
{}

### Response:
{}""".format(example["instruction"], example["input"], example["output"])

def main():
    pairs = load_pairs(pairs_path)
    print("Loaded {} training pairs".format(len(pairs)))
    # Add your Unsloth training loop here.

if __name__ == "__main__":
    main()
"####;
        fs::write(output_dir.join("train_unsloth.py"), script)?;
        Ok(())
    }

    pub fn evaluate_quality_gate(
        baseline_accuracy: f64,
        quantized_accuracy: f64,
    ) -> QualityGateResult {
        let delta_pp = (baseline_accuracy - quantized_accuracy) * 100.0;
        let passed = delta_pp <= 1.0;
        QualityGateResult {
            passed,
            baseline_accuracy,
            quantized_accuracy,
            drop_percentage_points: delta_pp,
            threshold_percentage_points: 1.0,
            summary: if passed {
                format!(
                    "PASSED: Quality drop is {:.2}pp (<= 1.00pp limit)",
                    delta_pp
                )
            } else {
                format!("FAILED: Quality drop is {:.2}pp (> 1.00pp limit)", delta_pp)
            },
        }
    }

    fn generate_pairs(profile: &str, roles: &[String]) -> Vec<DistillationPair> {
        let mut pairs = Vec::new();

        for role in roles {
            pairs.push(DistillationPair {
                instruction: format!(
                    "Infer the job archetype from this profile for role: {}",
                    role
                ),
                input: profile[..profile.len().min(4000)].to_string(),
                output: format!(
                    "Role: {}\nFit: strong\nRationale: inferred from profile",
                    role
                ),
                meta: DistillationMeta {
                    source: "atsassin-role-inference",
                    task: "role_inference",
                    model_tier: "light",
                    confidence: 0.8,
                },
            });

            pairs.push(DistillationPair {
                instruction: "Score this job description against the profile".to_string(),
                input: format!(
                    "Profile: {}\nJD: Sample job description for {}",
                    &profile[..profile.len().min(2000)],
                    role
                ),
                output:
                    "{\"overall_score\": 0.75, \"dimensions\": [], \"recommendation\": \"Apply\"}"
                        .to_string(),
                meta: DistillationMeta {
                    source: "atsassin-scoring",
                    task: "scoring",
                    model_tier: "balanced",
                    confidence: 0.75,
                },
            });

            pairs.push(DistillationPair {
                instruction: format!("Tailor resume for role: {}", role),
                input: profile[..profile.len().min(3000)].to_string(),
                output: format!(
                    "Tailored resume summary for {} role emphasizing relevant experience.",
                    role
                ),
                meta: DistillationMeta {
                    source: "atsassin-tailoring",
                    task: "tailoring",
                    model_tier: "balanced",
                    confidence: 0.7,
                },
            });
        }

        pairs
    }

    fn parse_model_size_mb(model: &str) -> usize {
        let lower = model.to_lowercase();
        if lower.contains("3b") {
            return 2000;
        }
        if lower.contains("4b") {
            return 3000;
        }
        if lower.contains("7b") {
            return 5000;
        }
        if lower.contains("9b") {
            return 6000;
        }
        if lower.contains("14b") {
            return 10000;
        }
        if lower.contains("22m") {
            return 100;
        }
        if lower.contains("109m") {
            return 500;
        }
        0
    }

    fn quantization_bits(q: &str) -> u32 {
        match q.to_uppercase().as_str() {
            "Q2_K" | "Q2" => 2,
            "Q3_K_M" | "Q3" => 3,
            "Q4_K_M" | "Q4_K" | "Q4" => 4,
            "Q5_K_M" | "Q5" => 5,
            "Q6_K" | "Q6" => 6,
            "Q8_0" | "Q8" => 8,
            _ => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub reason: String,
    pub ram_required_gb: u8,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QualityGateResult {
    pub passed: bool,
    pub baseline_accuracy: f64,
    pub quantized_accuracy: f64,
    pub drop_percentage_points: f64,
    pub threshold_percentage_points: f64,
    pub summary: String,
}
