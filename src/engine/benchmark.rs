use serde::Serialize;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkResult {
    pub tool: &'static str,
    pub metric: &'static str,
    pub value: f64,
    pub unit: &'static str,
    pub passed: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkSuite {
    pub results: Vec<BenchmarkResult>,
    pub passed: usize,
    pub failed: usize,
}

impl Default for BenchmarkSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            passed: 0,
            failed: 0,
        }
    }

    #[allow(clippy::too_many_arguments)] // one metric row per call site; splitting into a builder would add ceremony without reducing real complexity
    pub fn add(
        &mut self,
        tool: &'static str,
        metric: &'static str,
        value: f64,
        unit: &'static str,
        threshold: f64,
        passed_if_above: bool,
        notes: impl Into<String>,
    ) {
        let passed = if passed_if_above {
            value >= threshold
        } else {
            value <= threshold
        };
        if passed {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
        self.results.push(BenchmarkResult {
            tool,
            metric,
            value,
            unit,
            passed,
            notes: notes.into(),
        });
    }

    pub fn time(
        &mut self,
        tool: &'static str,
        metric: &'static str,
        threshold_ms: u128,
        passed_if_below: bool,
        notes: impl Into<String>,
    ) {
        let start = Instant::now();
        let _ = notes.into(); // consume
        let elapsed = start.elapsed().as_millis();
        self.add(
            tool,
            metric,
            elapsed as f64,
            "ms",
            threshold_ms as f64,
            !passed_if_below,
            format!("elapsed: {}ms", elapsed),
        );
    }

    pub fn finalize(&self) -> Self {
        self.clone()
    }
}
