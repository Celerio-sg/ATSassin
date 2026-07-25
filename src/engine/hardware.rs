use crate::config::ModelTier;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct HardwareProfile {
    pub total_ram_gb: u8,
    pub available_ram_gb: u8,
    pub cpu_cores: u8,
    pub has_gpu: bool,
    pub gpu_vram_gb: Option<u8>,
    pub recommended_tier: &'static str,
    pub recommended_model: &'static str,
    pub quantization: &'static str,
    pub context_tokens: u32,
    pub cpu_threads: u8,
    pub batch_size: usize,
}

static HARDWARE_PROFILE: OnceLock<HardwareProfile> = OnceLock::new();

impl HardwareProfile {
    pub fn detect() -> Self {
        let (total_ram, cpu_cores) = Self::probe_system();
        let (has_gpu, gpu_vram) = Self::probe_gpu();

        let recommended = if has_gpu && gpu_vram.unwrap_or(0) >= 16 {
            "full"
        } else if total_ram >= 8 || (has_gpu && gpu_vram.unwrap_or(0) >= 8) {
            "balanced"
        } else {
            "light"
        };

        let (model, quant, ctx, threads, batch) = match recommended {
            "full" => ("qwen2.5:7b", "Q6_K", 32768, 0, 32),
            "balanced" => ("qwen2.5:3b", "Q4_K_M", 8192, 6, 64),
            _ => ("qwen2.5:0.5b", "Q4_K_M", 4096, 4, 64),
        };

        Self {
            total_ram_gb: total_ram,
            available_ram_gb: total_ram.saturating_sub(2),
            cpu_cores,
            has_gpu,
            gpu_vram_gb: gpu_vram,
            recommended_tier: recommended,
            recommended_model: model,
            quantization: quant,
            context_tokens: ctx,
            cpu_threads: threads,
            batch_size: batch,
        }
    }

    pub fn global() -> &'static Self {
        HARDWARE_PROFILE.get_or_init(Self::detect)
    }

    pub fn cpu_optimization_hints(&self) -> Vec<String> {
        let mut hints = Vec::new();

        if !self.has_gpu && self.total_ram_gb >= 8 {
            hints.push(
                "CPU-only mode detected: use Q4_K_M quantization for best speed/quality tradeoff"
                    .to_string(),
            );
            hints.push(format!(
                "Set OLLAMA_NUM_THREAD={} for CPU inference",
                self.cpu_threads.min(self.cpu_cores)
            ));
            hints.push("Enable OLLAMA_KEEP_ALIVE=30m to avoid model reloads".to_string());
        }

        if self.has_gpu {
            if let Some(vram) = self.gpu_vram_gb {
                if vram < 8 {
                    hints.push(format!(
                        "GPU VRAM {}GB: use Q4_K_M quantization to fit model",
                        vram
                    ));
                } else if vram >= 16 {
                    hints.push(
                        "GPU VRAM >=16GB: use Q6_K quantization for best quality".to_string(),
                    );
                }
            }
        } else {
            hints.push("No GPU detected: ATSassin will run on CPU only".to_string());
            hints.push(
                "Consider qwen2.5:3b for fastest CPU inference if quality is insufficient"
                    .to_string(),
            );
        }

        hints.push(format!(
            "Recommended batch size for embeddings: {}",
            self.batch_size
        ));
        hints.push(
            "For scoring: use 1 prompt at a time; for batch embedding: use recommended batch size"
                .to_string(),
        );

        hints
    }

    pub fn tier_for_hardware(&self, tier: &ModelTier) -> ModelTier {
        let mut adjusted = tier.clone();

        if !self.has_gpu && tier.cpu_ok && tier.quantization == "Q6_K" {
            adjusted.quantization = "Q4_K_M".to_string();
        }

        if self.total_ram_gb < tier.ram_min_gb {
            adjusted.context_tokens = adjusted.context_tokens.min(4096);
        }

        adjusted
    }

    pub fn inference_params(&self) -> InferenceParams {
        InferenceParams {
            batch_size: if self.has_gpu { 32 } else { 64 },
            max_sequence_length: if self.has_gpu { 256 } else { 128 },
            num_threads: self.cpu_threads.min(self.cpu_cores),
            keep_alive_secs: if self.has_gpu { 600 } else { 1800 },
            quantization_validation_threshold: 0.01,
        }
    }

    fn probe_system() -> (u8, u8) {
        let total_ram = std::env::var("ATSASSIN_RAM_GB")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8);

        let cpu_cores = std::env::var("ATSASSIN_CPU_CORES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|n| n.get() as u8)
                    .unwrap_or(4)
            });

        (total_ram, cpu_cores)
    }

    fn probe_gpu() -> (bool, Option<u8>) {
        if let Ok(val) = std::env::var("ATSASSIN_HAS_GPU") {
            let has_gpu = val == "1" || val.to_lowercase() == "true";
            let vram = std::env::var("ATSASSIN_GPU_VRAM_GB")
                .ok()
                .and_then(|v| v.parse().ok());
            return (has_gpu, vram);
        }

        #[cfg(target_os = "windows")]
        {
            let mut has_gpu = false;
            let mut vram_gb: Option<u8> = None;

            if let Ok(output) = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "Get-CimInstance Win32_VideoController | Select-Object Name, AdapterRAM",
                ])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stdout.is_empty() && !stdout.contains("VideoProcessor") {
                    has_gpu = true;
                    for line in stdout.lines() {
                        let trimmed = line.trim();
                        if let Some(bytes_str) = trimmed.split_whitespace().last() {
                            if let Ok(bytes) = bytes_str.parse::<u64>() {
                                if bytes > 0 {
                                    let gb = (bytes as f64 / 1024.0 / 1024.0 / 1024.0).ceil() as u8;
                                    vram_gb = Some(gb.max(1));
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if has_gpu && vram_gb.is_none() {
                vram_gb = Some(2);
            }

            (has_gpu, vram_gb)
        }

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            if std::path::Path::new("/dev/dri").exists() {
                return (true, Some(8));
            }
            return (false, None);
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            return (false, None);
        }
    }
}

#[derive(Debug, Clone)]
pub struct InferenceParams {
    pub batch_size: usize,
    pub max_sequence_length: u32,
    pub num_threads: u8,
    pub keep_alive_secs: u32,
    pub quantization_validation_threshold: f64,
}

impl InferenceParams {
    pub fn as_env_suggestion(&self) -> Vec<String> {
        vec![
            format!("OLLAMA_NUM_THREAD={}", self.num_threads),
            format!("OLLAMA_KEEP_ALIVE={}s", self.keep_alive_secs),
        ]
    }

    pub fn quantization_quality_gate(
        original_accuracy: f64,
        quantized_accuracy: f64,
        threshold: f64,
    ) -> bool {
        let delta = (original_accuracy - quantized_accuracy).abs();
        delta <= threshold
    }
}
