//! Validation for data that is about to leave the local machine.
//!
//! Egress checks operate on the exact bytes that will be transmitted. Callers
//! cannot upload an arbitrary path through the Lightning client: they must
//! first construct an opaque [`ValidatedTrainingPayload`] here.

use crate::engine::pii_scrubber::{contains_pii, ScrubContext};
use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ValidatedTrainingPayload {
    file_name: String,
    bytes: Vec<u8>,
}

impl ValidatedTrainingPayload {
    pub fn from_jsonl(path: &Path, context: &ScrubContext) -> Result<Self> {
        if !context.has_identity_context() {
            anyhow::bail!(
                "Training upload blocked: candidate identity context is unavailable, so free-text data cannot be checked safely"
            );
        }

        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Training upload blocked: cannot read {}", path.display()))?;
        if !metadata.is_file() {
            anyhow::bail!(
                "Training upload blocked: {} is not a regular file",
                path.display()
            );
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            anyhow::bail!(
                "Training upload blocked: {} is not a JSONL file",
                path.display()
            );
        }

        let bytes = std::fs::read(path)
            .with_context(|| format!("Training upload blocked: cannot read {}", path.display()))?;
        if bytes.is_empty() {
            anyhow::bail!("Training upload blocked: {} is empty", path.display());
        }

        let text = std::str::from_utf8(&bytes).with_context(|| {
            format!(
                "Training upload blocked: {} is not valid UTF-8",
                path.display()
            )
        })?;
        let mut record_count = 0;
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let record = serde_json::from_str::<serde_json::Value>(line).with_context(|| {
                format!(
                    "Training upload blocked: {} line {} is not valid JSON",
                    path.display(),
                    index + 1
                )
            })?;
            if !record.is_object() {
                anyhow::bail!(
                    "Training upload blocked: {} line {} is not a JSON object",
                    path.display(),
                    index + 1
                );
            }
            record_count += 1;
        }
        if record_count == 0 {
            anyhow::bail!(
                "Training upload blocked: {} contains no JSON records",
                path.display()
            );
        }

        if contains_pii(text, context) {
            anyhow::bail!(
                "Training upload blocked: detectable personal data remains in {}",
                path.display()
            );
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("Training upload blocked: training file name is not valid UTF-8")?
            .to_string();

        Ok(Self { file_name, bytes })
    }

    pub(crate) fn file_name(&self) -> &str {
        &self.file_name
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_context() -> ScrubContext {
        let mut context = ScrubContext::default();
        context.add_identity_term("Synthetic Candidate");
        context
    }

    #[test]
    fn valid_payload_owns_the_checked_bytes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("training_pairs.jsonl");
        let content =
            b"{\"instruction\":\"synthetic\",\"input\":\"marker-143\",\"output\":\"safe\"}\n";
        std::fs::write(&path, content)?;

        let payload = ValidatedTrainingPayload::from_jsonl(&path, &identity_context())?;

        assert_eq!(payload.file_name(), "training_pairs.jsonl");
        assert_eq!(payload.bytes(), content);
        Ok(())
    }

    #[test]
    fn detected_pii_never_constructs_a_payload() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("training_pairs.jsonl");
        std::fs::write(
            &path,
            "{\"instruction\":\"contact\",\"input\":\"Synthetic Candidate\",\"output\":\"safe\"}\n",
        )?;

        let error = ValidatedTrainingPayload::from_jsonl(&path, &identity_context())
            .expect_err("PII-bearing bytes must be rejected");

        assert!(error.to_string().contains("personal data remains"));
        assert!(
            !temp.path().join("training_pairs.flagged.jsonl").exists(),
            "rejected data must not be copied into a second sensitive artifact"
        );
        Ok(())
    }

    #[test]
    fn missing_identity_context_fails_closed() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("training_pairs.jsonl");
        std::fs::write(&path, "{\"input\":\"safe\"}\n")?;

        let error = ValidatedTrainingPayload::from_jsonl(&path, &ScrubContext::default())
            .expect_err("free text without identity context must be rejected");

        assert!(error.to_string().contains("identity context"));
        Ok(())
    }

    #[test]
    fn unsupported_profile_identity_fails_closed() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("training_pairs.jsonl");
        std::fs::write(&path, "{\"input\":\"safe\"}\n")?;
        let profile = crate::engine::profile_parser::ProfileParser::profile_from_text("")?;
        let context = ScrubContext::from_profile(&profile);

        assert_eq!(profile.name, "Unknown");
        let error = ValidatedTrainingPayload::from_jsonl(&path, &context)
            .expect_err("an unparsed candidate identity must block free-text egress");
        assert!(error.to_string().contains("identity context"));
        Ok(())
    }

    #[test]
    fn missing_malformed_and_wrong_extension_inputs_fail_closed() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let context = identity_context();

        let missing = temp.path().join("missing.jsonl");
        assert!(ValidatedTrainingPayload::from_jsonl(&missing, &context).is_err());

        let malformed = temp.path().join("malformed.jsonl");
        std::fs::write(&malformed, "not-json\n")?;
        assert!(ValidatedTrainingPayload::from_jsonl(&malformed, &context).is_err());

        let non_object = temp.path().join("non_object.jsonl");
        std::fs::write(&non_object, "\"valid JSON, wrong JSONL shape\"\n")?;
        assert!(ValidatedTrainingPayload::from_jsonl(&non_object, &context).is_err());

        let wrong_extension = temp.path().join("training_pairs.txt");
        std::fs::write(&wrong_extension, "{\"input\":\"safe\"}\n")?;
        assert!(ValidatedTrainingPayload::from_jsonl(&wrong_extension, &context).is_err());
        Ok(())
    }
}
