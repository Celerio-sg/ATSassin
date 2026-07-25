# ATSassin Job-Securing Playbook
> Integrated strategies for landing contract/remote tech roles — especially for senior PM/GTM/Sales leaders with APAC experience.

---

## 1. Profile Hygiene (Day 1)

1. Export LinkedIn → Markdown/CSV
2. Run `atsassin profile init --linkedin <export-dir>`
3. Review parsed profile; save to `profile.md`
4. Run `atsassin roles infer -n 8` to discover target archetypes
5. Lock 3–5 target roles in `roles/` directory

## 2. Daily Discovery Routine (15 min/day)

1. `atsassin scan --role "<target-role>" --limit 20`
2. Filter for: remote-first OR APAC-hybrid; revenue $20M–$500M; PM/GTM/Sales
3. Auto-score with lightweight model: `atsassin evaluate --file jd.txt`
4. Shortlist only ≥0.7 score

## 3. Evaluation Gate

Before applying, ATSassin runs a 6-dimension evaluation:
- Role match (1-5)
- North-star alignment (1-5)
- Compensation fit (1-5)
- Cultural signals (1-5)
- Red flags (1-5)
- Global fit (1-5)

Skip if overall score <0.7.

## 4. Tailoring & Export (10 min/job)

1. `atsassin tailor --job-id <id> --output tailored.md`
2. Review draft for accuracy
3. Export to Markdown/PDF
4. Submit manually via company portal or LinkedIn Easy Apply

## 5. Pipeline Tracking

1. `atsassin pipeline add --job-id <id> --status applied`
2. Log contact, follow-up date, notes
3. Review weekly in TUI: `atsassin tui`

## 6. APAC-Specific Tactics

- **LinkedIn**: Connection requests with value-first note (35-45% acceptance)
- **Recruiter outreach**: Target specialized APAC tech recruiters (NJCS, Salt APAC, Robert Half APAC)
- **Referrals**: Ask for intros from former colleagues; APAC hiring prioritizes trusted references
- **Contract platforms**: Use Upwork/Toptal for immediate contract roles while searching for FTE
- **Content**: Post 1-2×/week on LinkedIn about APAC GTM strategy to build inbound recruiter interest
- **Speed**: Apply within 48h of posting; ATSassin enables rapid tailoring

## 7. Interview Prep

1. ATSassin pulls company signals (opt-in web search)
2. Builds interview pack: 5 STAR+R stories mapped to likely questions
3. Generates APAC comp benchmarks for negotiation

---

**Remember**: ATSassin is a drafting aid, not an auto-apply bot. You submit. You interview. You negotiate. You win.
