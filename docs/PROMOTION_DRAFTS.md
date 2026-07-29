# ATSassin Promotion Drafts

> ## ⛔ DO NOT PUBLISH AS-IS — corrections required (2026-07-29)
>
> The 2026-07-29 adversarial review found that every draft below asserts at least one claim the codebase does not support. Fix these before any of this copy is posted:
>
> | Claim in drafts | Reality |
> |---|---|
> | "~9.5 MB binary" | **10.96 MB**, measured on a clean `--release` build |
> | "runs on 4 GB RAM" stated as fact | **Unvalidated target.** Never tested on 4 GB CPU-only hardware (#5/#57/#73) |
> | "runs entirely with local Ollama by default" | Testing to date has run against cloud Groq; local CPU-only behaviour is untested |
> | Career-coach mode, community pooling, LoRA sharing in present tense | All unbuilt. Career coaching is Steps 2–3 of the critical chain; LoRA sharing is Stage 0–1 only, and Stage 3 (DHT/P2P) is **rejected** |
>
> Publishing an unverified hardware claim is the single fastest way to lose the trust this project's positioning depends on. If a claim cannot be cited to a measurement or a test, cut it rather than soften it.

Use these as starting points. Adapt the tone to each community and replace placeholder metrics with real ones before posting.

---

## A — Reddit

### r/rust

**Title:** ATSassin — a Rust CLI that crowdsources job-market intelligence and acts as your local-first career coach (open source)

**Body:**

I’m building ATSassin, a Rust CLI for job search that stays local-first and privacy-first by default. It parses your resume, infers target roles, scrapes job boards, scores jobs against your profile, and tailors resumes/cover letters — all with local Ollama or whichever OpenAI-compatible provider you choose.

What’s different:
- No SaaS lock-in, no resume data sold, no account required.
- A real TUI dashboard (not a web app) for scanning, evaluating, and tailoring.
- Building toward community-pooled job/salary intelligence and LoRA sharing so better models produce better shared artifacts.
- Lightweight: ~9.5 MB release binary, designed for 4GB CPU laptops, local-first by default (Ollama).

The codebase is MIT-licensed and actively looking for contributors. If you care about local AI, privacy, or Rust CLI ergonomics, there are good first issues.

- Repo: https://github.com/Celerio-sg/ATSassin
- Demo: assets/demo.gif

Would love feedback, especially on the scraper design and the community-sharing roadmap.

### r/opensource

**Title:** ATSassin — an open-source, local-first job-search assistant that doesn’t sell your resume

**Body:**

Most job-search tools are either expensive SaaS or quietly monetize your data. ATSassin is the opposite: a lightweight, open-source Rust CLI that keeps your resume and pipeline on your own machine.

What it does:
- Parses resume/LinkedIn/portfolio
- Infers suitable roles
- Scans boards and scores matches
- Tailors resumes and cover letters
- Tracks your pipeline in SQLite

Community direction:
- Crowdsourced board/salary/review pooling (anonymized, opt-in)
- Community LoRA adapter sharing
- Continuous career-coach mode that alerts you to better opportunities before you’re actively looking

MIT licensed. Contributions welcome.

https://github.com/Celerio-sg/ATSassin

### r/cscareerquestions

**Title:** I built a career-coach CLI that continuously watches the market for roles that fit you

**Body:**

ATSassin is an open-source, local-first CLI that:
- Parses your resume and infers roles you might not have considered
- Continuously scans job boards (or can be scheduled to)
- Scores jobs against your actual profile and preferences
- Generates tailored resumes and cover letters
- Tracks your pipeline

The part I’m most excited about is the career-coach angle: instead of only job hunting when you’re ready to leave, it keeps an anonymized watch on the market and flags opportunities that could improve your income/career — relocating, contracting, adjacent skills, etc. It’s a prompt, not a prescription.

Everything is opt-in and runs locally by default. Your data isn’t uploaded to anyone unless you explicitly configure a cloud LLM provider.

Repo: https://github.com/Celerio-sg/ATSassin

Curious what features you’d actually use.

### r/SideProject

**Title:** Side project — ATSassin, a local-first Rust CLI for job search and career optimization

**Body:**

ATSassin is my open-source side project: a Rust CLI that helps you find, score, and apply for jobs without giving your resume to a SaaS.

Current features:
- Resume/LinkedIn parsing
- Role inference
- Job board scanning (LinkedIn, Seek, Greenhouse directory, social aggregators)
- ATS-style job scoring
- Resume/cover-letter tailoring
- TUI dashboard
- Pipeline tracking

Where it’s going:
- Continuous career-coach mode
- Community pooling of salary/review/board data
- LoRA adapter sharing for local models

Looking for contributors and feedback.

https://github.com/Celerio-sg/ATSassin

---

## B — Blogs

### Dev.to / Hashnode / LinkedIn Article

**Title:** Building the Anti-Headhunter: A Local-First, Open-Source Career Coach in Rust

**Hook:**

Job-search tools today fall into two camps: expensive SaaS products that lock you in and sell your data, or generic advice that doesn’t understand your specific skills. We need a third option — a local-first, open-source career optimizer that keeps your data under your control and helps you stay ahead of the market continuously.

**Body (key points):**

1. **Local-first is not optional.** Your resume, your pipeline, and your salary data should live on your own machine. ATSassin runs entirely with local Ollama by default. Cloud providers are opt-in and you control the API keys.

2. **From reactive to proactive.** Traditional tools help you when you decide to job hunt. ATSassin’s career-coach mode keeps an anonymized watch on the market and surfaces opportunities that match your profile — before you’ve decided to leave.

3. **Community without surveillance.** Crowd-sourced board URLs, salary signals, and company reviews are opt-in and anonymized. Better data helps everyone without selling anyone’s resume.

4. **Built for the AI era.** The roadmap includes AI-exposure analysis to help users identify skills at risk of automation and pivot toward more durable roles.

5. **Open source, Rust, MIT.** Contributions are welcome. The goal is a free, autonomous earning optimizer for everyone.

**CTA:**

If you believe job-search tooling should be private, local, and community-owned, star the repo and open an issue: https://github.com/Celerio-sg/ATSassin

---

## C — Product Hunt

**Name:** ATSassin

**Tagline:** The local-first, open-source career optimizer that keeps you ahead of the market.

**Description:**

ATSassin is a privacy-first, local-first job-search assistant written in Rust. It parses your resume, infers target roles, scans job boards, scores matches, and tailors application materials — all on your own machine.

**Key features:**
- Local-first by default (Ollama) — ~9.5 MB binary, runs on 4GB RAM
- Role inference from resume/LinkedIn/portfolio
- Job board scanning with ATS-style scoring
- Resume and cover-letter tailoring
- Interactive TUI dashboard + pipeline tracking

**What makes it different:**
- Your resume data never leaves your machine unless you explicitly opt in.
- Career-coach mode watches the market and flags better opportunities.
- Community pooling of anonymized salary, board, and review data.
- Open source (MIT).

**CTA:**

Check it out, leave feedback, and contribute on GitHub: https://github.com/Celerio-sg/ATSassin

**Topics/tags:** Open Source, Developer Tools, Career, Job Search, Privacy, Local AI, Rust
