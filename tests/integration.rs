use anyhow::Result;

#[tokio::test]
async fn test_llm_client_circuit_breaker() -> Result<()> {
    use atsassin::engine::llm::LlmClient;
    use atsassin::engine::llm::LlmProvider;
    use atsassin::engine::llm::LlmRequest;

    let client = LlmClient::new("http://localhost:11434", None, LlmProvider::Ollama, 5, 1);

    let request = LlmRequest {
        model: "qwen3.5:4b".to_string(),
        messages: vec![],
        temperature: 0.7,
        max_tokens: 100,
        stream: false,
    };

    let result = client.chat(request).await;
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_cost_calculator_free_tier() -> Result<()> {
    use atsassin::engine::cost::CostCalculator;

    let calc = CostCalculator::new();
    let cost = calc.calculate("ollama", "qwen3.5:4b", 1000, 500);
    assert_eq!(cost.total_usd, 0.0);
    assert_eq!(cost.input_cost, 0.0);
    Ok(())
}

#[test]
fn test_cost_calculator_metered() -> Result<()> {
    use atsassin::engine::cost::CostCalculator;

    let calc = CostCalculator::new();
    let cost = calc.calculate("openai", "gpt-4", 1_000_000, 2_000_000);
    assert!(cost.total_usd > 0.0);
    Ok(())
}

#[test]
fn test_hardware_profile_detection() -> Result<()> {
    use atsassin::engine::hardware::HardwareProfile;

    let profile = HardwareProfile::detect();
    assert!(profile.total_ram_gb > 0);
    assert!(profile.cpu_cores > 0);
    assert!(!profile.recommended_tier.is_empty());
    Ok(())
}

#[test]
fn test_router_from_config() -> Result<()> {
    use atsassin::config::LlmConfig;
    use atsassin::config::ModelTier;
    use atsassin::engine::router::ModelRouter;

    let llm = LlmConfig::default();
    let light = ModelTier {
        model: "qwen3.5:4b".into(),
        quantization: "Q4_K_M".into(),
        context_tokens: 4096,
        cpu_ok: true,
        cpu_threads: Some(4),
        ram_min_gb: 4,
        score_threshold: 0.5,
        passes: 1,
        recommended_batch: 1,
    };
    let balanced = ModelTier {
        model: "qwen3.5:9b".into(),
        quantization: "Q6_K".into(),
        context_tokens: 8192,
        cpu_ok: true,
        cpu_threads: Some(8),
        ram_min_gb: 8,
        score_threshold: 0.6,
        passes: 2,
        recommended_batch: 2,
    };
    let full = ModelTier {
        model: "qwen3.5:32b".into(),
        quantization: "Q8_0".into(),
        context_tokens: 32768,
        cpu_ok: false,
        cpu_threads: None,
        ram_min_gb: 16,
        score_threshold: 0.7,
        passes: 3,
        recommended_batch: 4,
    };

    let router = ModelRouter::from_llm_config(&llm, light, balanced, full, None);
    assert!(!router.fallback_chain.is_empty());
    Ok(())
}

#[test]
fn test_quality_stats_empty() -> Result<()> {
    use atsassin::engine::quality::QualityStats;

    let stats = QualityStats::empty();
    assert_eq!(stats.total_calls, 0);
    assert_eq!(stats.acceptance_rate, 0.0);
    Ok(())
}

#[test]
fn test_distillation_pair_structure() -> Result<()> {
    use atsassin::engine::distillation::DistillationMeta;
    use atsassin::engine::distillation::DistillationPair;

    let pair = DistillationPair {
        instruction: "Test instruction".to_string(),
        input: "Test input".to_string(),
        output: "Test output".to_string(),
        meta: DistillationMeta {
            source: "test",
            task: "test",
            model_tier: "light",
            confidence: 0.9,
        },
    };

    assert_eq!(pair.instruction, "Test instruction");
    assert_eq!(pair.meta.confidence, 0.9);
    Ok(())
}

#[test]
fn test_social_leads_to_jobs() -> Result<()> {
    use atsassin::pipeline::social_scraper::{SocialJobLead, SocialScraper};

    let leads = vec![SocialJobLead {
        title: "Rust Engineer".to_string(),
        company: "Acme".to_string(),
        location: "Remote".to_string(),
        url: "https://example.com/1".to_string(),
        source_platform: "Reddit".to_string(),
        author: "u/test".to_string(),
        posted_at: None,
        snippet: "Hiring Rust engineer".to_string(),
    }];

    let jobs = SocialScraper::social_leads_to_jobs(leads);
    assert_eq!(jobs.len(), 1);
    assert!(jobs[0].title.contains("Reddit"));
    assert_eq!(jobs[0].company, "Acme");
    Ok(())
}

#[test]
fn test_distillation_pipeline_script_generation() -> Result<()> {
    use atsassin::engine::distillation::DistillationPipeline;
    use atsassin::engine::pii_scrubber::ScrubContext;
    let temp_dir = tempfile::tempdir()?;
    let roles = vec!["Software Architect".to_string(), "AI Engineer".to_string()];
    let mut scrub_context = ScrubContext::default();
    scrub_context.add_identity_term("Synthetic Candidate");
    DistillationPipeline::export_training_data(
        "# Profile Text\nRust Specialist",
        &roles,
        temp_dir.path(),
        &scrub_context,
    )?;

    assert!(temp_dir.path().join("training_data.jsonl").exists());
    assert!(temp_dir.path().join("manifest.json").exists());
    assert!(temp_dir.path().join("export_onnx.py").exists());
    assert!(temp_dir.path().join("quantize_gguf.sh").exists());
    assert!(temp_dir.path().join("openvino_export.py").exists());

    let gate_res = DistillationPipeline::evaluate_quality_gate(0.95, 0.945);
    assert!(gate_res.passed);
    assert!(gate_res.drop_percentage_points <= 1.0);
    Ok(())
}

#[test]
fn test_fast_hardware_probe_override() -> Result<()> {
    use atsassin::engine::hardware::HardwareProfile;
    std::env::set_var("ATSASSIN_HAS_GPU", "true");
    std::env::set_var("ATSASSIN_GPU_VRAM_GB", "12");

    let profile = HardwareProfile::detect();
    assert!(profile.has_gpu);
    assert_eq!(profile.gpu_vram_gb, Some(12));

    std::env::remove_var("ATSASSIN_HAS_GPU");
    std::env::remove_var("ATSASSIN_GPU_VRAM_GB");
    Ok(())
}

#[test]
fn test_feedback_tracker_full_cycle() -> Result<()> {
    use atsassin::engine::feedback::{FeedbackAction, FeedbackTask, FeedbackTracker};
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("test_feedback.db");
    let tracker = FeedbackTracker::new(&db_path)?;

    let id = tracker.record_feedback(
        "job-123",
        FeedbackTask::Scoring,
        FeedbackAction::Accepted,
        "Original score",
        None,
        0.8,
        0.8,
    )?;
    assert!(id > 0);

    let rate = tracker.get_acceptance_rate(FeedbackTask::Scoring, 30)?;
    assert_eq!(rate, 1.0);

    let recent = tracker.list_recent(5)?;
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].job_id, "job-123");
    Ok(())
}
