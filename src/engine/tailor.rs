use crate::engine::egress::PromptEgressBuilder;
use crate::engine::prompts::Prompts;
use crate::engine::router::ModelRouter;
use crate::models::job::Job;
use anyhow::Result;

pub struct Tailor {
    pub router: ModelRouter,
    pub prompts: Prompts,
}

impl Tailor {
    pub fn new(router: ModelRouter, prompts: Prompts) -> Self {
        Self { router, prompts }
    }

    pub async fn generate_resume(
        &self,
        job: &Job,
        profile: &crate::models::profile::UserProfile,
    ) -> Result<String> {
        let messages = self.prompts.tailor_resume_prompt(job, profile)?;
        let tier = self.router.tier("balanced");
        let resp = self.router.chat(messages, tier).await?;
        Ok(resp.content)
    }

    pub async fn generate_cover_letter(
        &self,
        job: &Job,
        profile: &crate::models::profile::UserProfile,
    ) -> Result<String> {
        let messages = self.prompts.cover_letter_prompt(job, profile)?;
        let tier = self.router.tier("balanced");
        let resp = self.router.chat(messages, tier).await?;
        Ok(resp.content)
    }

    pub async fn refine_resume(&self, draft: &str, job: &Job, feedback: &str) -> Result<String> {
        let mut builder = PromptEgressBuilder::new(
            "You are an expert resume editor. Refine the draft based on feedback. Keep it factual and grounded in the original profile.",
            "Provide the improved resume using only the labelled data blocks below.",
        );
        builder.add_untrusted("job_title", &job.title)?;
        builder.add_untrusted("current_draft", draft)?;
        builder.add_untrusted("user_feedback", feedback)?;
        let messages = builder.build()?;
        let tier = self.router.tier("balanced");
        let resp = self.router.chat(messages, tier).await?;
        Ok(resp.content)
    }
}
