# **Quantitative Baselines and Mechanistic Determinants of Cold Online Application Conversion Rates**

The recruitment funnel for unsolicited ("cold") online job applications operates as a high-friction, multi-stage stochastic filtration system. In unreferred job searches, candidates navigate sequential evaluation hurdles managed by automated applicant tracking systems (ATS), human resources screeners, hiring managers, and interview panels. Quantitative benchmarks established across empirical labor economics, industrial-organizational psychology, and enterprise recruitment data demonstrate that yield rates are severely constrained at the initial screening boundary, with recovery occurring conditional on reaching live human evaluation.

## **Baseline Yield Metrics Across the Selection Pipeline**

The online application conversion pipeline exhibits steep drop-off rates at initial transition points, followed by higher conditional probabilities of success once candidates enter direct human review. Data aggregated across talent acquisition systems, employer audit studies, and recruitment platform analytics establish clear quantitative benchmarks for each phase of the hiring process.

| Recruitment Pipeline Transition | Baseline Conversion Yield | Primary Controlling Variables | Sources |
| :---- | :---- | :---- | :---- |
| **Job Ad Click to Application Completion** | 6.00% | Application form length, compensation transparency, mobile interface design | 1 |
| **Generic Application to First Interview** | 1.00% – 3.00% | Resume length, automated ATS keyword parsing, negative structural filters | 2 |
| **Tailored Application to First Interview** | 4.00% – 10.00% | Structural keyword matching, contextual skill positioning, early submission velocity | 2 |
| **Deeply Tailored Application to First Interview** | 8.00% – 15.00% | Full narrative re-alignment, targeted competency mapping, timing (\< 7 days) | 2 |
| **Application to Recruiter Phone Screen** | 10.00% – 20.00% | ATS pass rate, structural parser compatibility, baseline credential fulfillment | 2 |
| **Phone Screen to Hiring Manager Interview** | 20.00% – 50.00% | Role alignment, salary expectation parity, verbal communication presentation | 2 |
| **Subsequent Interview Stage to Job Offer** | 30.00% – 50.00% | Structured interview performance, impression management, perceived fit | 4 |
| **Cumulative Cold Application to Offer Rate** | 1.00% – 4.00% | Overall pipeline volume, candidate positioning, structural market tightness | 2 |

Initial candidate entry into the recruitment pipeline exhibits major drop-off. Approximately 94% of individuals who view or click a digital job advertisement abandon the process prior to completing the submission form1. For candidates who submit an unreferred online application, initial callback rates for first-round screens range from under 2% in hyper-competitive technology and consumer internet sectors3 to an average baseline of 4% to 10% across broader corporate labor markets2. Conditional on securing an initial interview, the conditional probability of receiving a job offer increases significantly, with structured interview stages converting at 30% to 50%4, yielding an end-to-end cold-application-to-offer conversion rate between 1% and 4%2.

## **Phase I Determinants: Application-to-Interview Yields and Algorithmic Screening Mechanisms**

The transition from a submitted online application to an initial interview invitation is controlled by algorithmic screening, process friction, timing dynamics, and structural labor market biases.

### **Algorithmic Screening Logic and the Keyword Paradox**

Over 90% of large enterprise employers utilize Applicant Tracking Systems (ATS) or Recruitment Management Systems (RMS) to screen, rank, and filter incoming applicant pools5. These systems filter out 60% to 75% of incoming resumes prior to human HR review2.  
The primary mechanism of traditional ATS software relies on negative filtering logic designed to cull candidate volume rapidly rather than maximize long-term workforce performance7. Automated rules screen out candidates based on non-causal structural criteria, such as employment history gaps exceeding six months8, the absence of specific four-year post-secondary degree credentials regardless of practical competency7, or the absence of exact string matches for job description key terms2.  
Large-scale empirical evaluations of enterprise hiring data reveal a systemic structural flaw in keyword-based screening algorithms. Analysis of production data tracking candidates from initial application through post-hire output demonstrates that ATS keyword parsing has negligible or negative predictive validity regarding post-hire productivity10. Out of thousands of parsed skill keywords, zero exhibit statistically significant positive association with post-hire performance after correcting for multiple comparisons; in fact, a higher concentration of resume keywords is frequently associated with lower post-hire output10.

| Selection Screening Methodology | Predictive Power / Validity Metric | Operational Mechanism in Selection | Sources |
| :---- | :---- | :---- | :---- |
| **Behavioral & Personality Assessment** | Area Under Curve (AUC) \= 0.647 | Measures latent behavioral dimensions, adaptability, and resilience | 10 |
| **Structured Employment Interviews** | Criterion Validity (![][image1]) \= 0.42 – 0.51 | Standardized competency questions and objective rating rubrics | 10 |
| **ATS Keyword Screening Algorithms** | Area Under Curve (AUC) \= 0.558 | String matching of resume text against job description tokens | 10 |
| **Resume Keyword Density** | Negative Correlation with Output | High keyword counts correlate with 25% lower odds of high production | 10 |

Despite this lack of predictive validity, keyword matching remains the operational gatekeeper of cold applications2. Consequently, targeted tailoring—aligning resume phrasing precisely with the target job description—improves initial callback rates by 50% to 200%+ compared to generic, mass-submitted applications2. Candidates utilizing light tailoring (keyword and summary adaptation) achieve callback ratios of 4% to 8%, whereas deep tailoring (restructuring achievement statements to mirror target competencies) increases callback yields to 8% to 15%2.

### **Application Friction, Temporal Velocity, and Duration Dependence**

The structural design of the online application portal directly influences candidate pipeline composition through completion friction1. Completion rates decline non-linearly as application duration increases:

| Application Duration Threshold | Application Completion Yield | Operational Impact on Pipeline | Sources |
| :---- | :---- | :---- | :---- |
| **Under 5 Minutes** | 12.47% | Maximizes applicant volume; lowers drop-off friction | 1 |
| **5 to 15 Minutes** | \~5.00% | Moderate candidate drop-off across standard forms | 1 |
| **Over 15 Minutes** | 3.61% | 245% drop in completion relative to short forms | 1 |

Major drivers of application abandonment include lengthy redundant form fields (cited by 50% of candidates who drop out) and the omission of baseline compensation parameters (cited by 31%)1.  
Temporal velocity—the elapsed time between a job posting's publication and application submission—serves as a primary leading indicator of review probability3. Due to candidate volume constraints, recruiter review bandwidth depletes rapidly after posting deployment. Applications submitted within the first 7 days receive a disproportionate share of human evaluation time; submissions after this window experience steep declines in review probability as candidate pipelines saturate3.  
Furthermore, longitudinal search models demonstrate negative duration dependence13. In longitudinal tracking of unemployed job seekers, the probability of securing an interview and offer declines steadily over elapsed search duration13. Approximately half of this decline is driven by diminishing applicant search intensity and fatigue (supply-side duration dependence), while the remaining half stems from recruiter behavior and negative employer signaling associated with elapsed unemployment duration (demand-side duration dependence)13.

### **Structural Biases and Audit/Correspondence Study Evidence**

Cold application callback rates are subject to systemic friction driven by demographic, structural, and institutional biases documented across field experiments and audit studies.

| Candidate Demographic / Background Variable | Relative Callback Probability | Empirical Source & Market Context | Sources |
| :---- | :---- | :---- | :---- |
| **Majority Group Profile (Standard)** | Baseline Benchmark (100%) | Standardized reference profile in field experiments | 14 |
| **Minority / Foreign-Named Profile** | 25.00% – 50.00% of Baseline | Field audit studies across European and US markets | 14 |
| **Gig Economy Experience** | 50.00% – 88.00% of Baseline | Swedish correspondence study on platform labor | 15 |
| **Prior Self-Employment / Entrepreneurship** | \< 10.00% of Baseline Yield | UK field experiment on managerial applications | 20 |

Audit studies across international labor markets consistently show that applicants with minority- or foreign-sounding names must submit between 50% and 300% more applications than identically qualified majority candidates to obtain a single interview callback15. In European labor markets, applicants with Swedish or German profiles demonstrate callback rates nearly double those of Arabic or Turkish profiles holding identical qualifications14. Centralized, algorithm-driven callback structures reduce discretionary reviewer bias slightly, but underlying training data risks re-encoding historical disparities21.  
Prior self-employment and entrepreneurial activity act as severe negative signals in cold corporate screening. In controlled UK field experiments, candidates transitioning from self-employment received positive interview callbacks in less than 1% of applications, compared to a 6% invitation rate for identically qualified regularly employed applicants20. Non-traditional employment experience (e.g., platform/gig economy work) yields an 11% improvement in contact rates relative to sustained unemployment, but provides only half the marginal value of traditional wage employment experience15. Automated RMS filters also universally penalize career gaps exceeding six months, systematically converting candidates with caregiving, medical, or military transition histories into "hidden workers" screened out before qualitative review5.

## **Phase II Determinants: Interview-to-Offer Conversion and Evaluative Variance**

Once a candidate navigates the cold application boundary to secure an interview, the controlling variables shift from document formatting and timing velocity to interpersonal evaluative dynamics, structured assessment validity, and behavioral impression management.

### **Interview Structuring and Construct Variance**

The employment interview exhibits widely variable criterion-related validity depending on its structural design10. Decades of selection psychology research confirm that structured interviews—characterized by standardized job-analyzed questions, objective scoring rubrics, and systematic rating procedures—substantially outperform unstructured interviews in predicting future job performance10.  
Empirical meta-analyses examining the constructs captured in interview ratings demonstrate that constructs related to interviewee performance (e.g., dynamic social presentation, communication style, impression management) account for twice as much variance in final interview ratings as constructs directly evaluating job-related content (e.g., job-specific declarative knowledge, technical domain expertise)11. This structural discrepancy explains why highly technically proficient candidates frequently fail to convert interviews to job offers when evaluating interactions purely through a technical lens.

### **Impression Management Mechanics and Perceived Organizational Fit**

Because interpersonal performance tactics heavily mediate interview scores, candidate use of Impression Management (IM) acts as a primary leading indicator of downstream success11. Impression management strategies during employment interviews divide into two primary operational classes:

> 1. **Ingratiation Tactics**: Behaviors designed to evoke interpersonal liking and perceived similarity, including praising the organization, expressing enthusiasm for company initiatives, validating interviewer opinions, and highlighting shared values24.  
> 2. **Self-Promotion Tactics**: Direct statements emphasizing individual achievements, competence, personal mastery, and past performance outcomes25.

Empirical field studies demonstrate that ingratiation tactics exert a powerful, statistically significant indirect effect on job offer generation24. Ingratiation directly increases interviewer perceptions of Person-Organization (P-O) Fit—the degree to which the candidate's values and style mirror the firm's culture24. Elevated P-O fit ratings directly drive hiring recommendations and job offer decisions24.  
Conversely, aggressive self-promotion tactics display a more complex relationship with job offers24. While self-promotion enhances baseline perceptions of competence, excessive or uncalibrated self-promotion can be perceived as boastful or manipulative, particularly when interviewers are highly sensitive to deceptive impression management27. Candidates with high self-monitoring capabilities successfully adjust IM tactics dynamically, using non-confrontational ingratiation to secure higher evaluative ratings25.

### **Candidate Pipeline Leakage and Evaluative Process Friction**

Despite reaching the interview stage, significant pipeline degradation occurs due to organizational friction and evaluative churn. Pipeline analytics reveal that 32% of all candidate drop-off across the entire talent acquisition life cycle occurs specifically at the interview stage—exceeding application abandonment (14%), scheduling delays (20%), and onboarding drop-off (18%) combined1.

| Recruitment Funnel Stage | Share of Total Pipeline Drop-Off | Primary Administrative Drivers | Sources |
| :---- | :---- | :---- | :---- |
| **Interview Evaluation Stage** | 32.00% | Role mismatch, assessment fatigue, uncalibrated panel variance | 1 |
| **Interview Scheduling Phase** | 20.00% | Delay friction, slow recruiter communication, calendar misalignment | 1 |
| **Onboarding Phase** | 18.00% | Background check friction, slow offer letter generation | 1 |
| **Application Submission Form** | 14.00% | Excessive form length, lack of salary disclosure | 1 |
| **Other Administrative Stages** | 16.00% | Internal budget holds, position cancellations | 1 |

Key drivers of candidate loss during the interview phase include extended latency between interview scheduling touchpoints (accounting for 20% of pipeline leakage), unclear job scope or surprise compensation mismatches revealed during initial interviews, and multi-stage evaluation fatigue, where multi-round interview panels introduce uncalibrated rater variance without incremental predictive validity1.

## **Strategic Optimization Levers and Empirical Impact**

To maximize conversion yields across both phases of the selection funnel, job applicants can deploy targeted interventions grounded in empirical selection literature.

| Funnel Target Stage | Specific Metric Targeted | Empirical Intervention Lever | Operational Mechanism & Measured Lift | Sources |
| :---- | :---- | :---- | :---- | :---- |
| **Phase I: Application** | Form Completion Yield | Select \< 5-Minute Application Portals | Reduces abandonment friction; increases completion yield from 3.61% to 12.47% | 1 |
| **Phase I: Screening** | ATS Filter Pass Rate | Deploy Deep Resume Tailoring | Bypasses automated negative filters; boosts callback rates from 1–3% up to 8–15% | 2 |
| **Phase I: Screening** | Recruiter Review Probability | Early Submission Velocity (\< 7 Days) | Capitalizes on unexhausted recruiter review capacity before pipeline saturation | 3 |
| **Phase II: Interview** | Evaluative Score | Structured Competency Framing (STAR) | Aligns response architecture with structured scoring rubrics, reducing rating error | 10 |
| **Phase II: Interview** | Offer Recommendation | Strategic Ingratiation & Value Matching | Drives perceived Person-Organization fit, the primary indirect predictor of job offers | 24 |
| **Phase II: Offer** | Pipeline Retention | Pre-Interview Scope & Salary Alignment | Mitigates the 32% interview-stage candidate drop-off driven by expectations mismatch | 1 |

Executing these levers requires systematically managing both entry point dynamics and live evaluative presentation. Tailoring applications to mirror target job description keywords bypasses algorithmic ATS rejection filters, elevating initial callback rates2. Once in live interviews, shifting focus toward ingratiation and organizational value alignment drives interviewer perceptions of cultural fit, translating interview invitations into finalized job offers24.

## **Analytical Conclusion**

The cold online job application process operates as a two-phase selection system controlled by distinct quantitative mechanisms. In Phase I (Application-to-Interview), conversion yields are suppressed by automated ATS screening logic, form friction, and submission latency1. Automated screening filters rely heavily on degree criteria, employment continuity, and keyword string matching, despite evidence indicating that keyword density lacks positive predictive validity for post-hire production7. Applicants optimize Phase I conversion by submitting deeply tailored resumes within seven days of job publication, raising baseline callback yields from 1%–3% up to 8%–15%2.  
In Phase II (Interview-to-Offer), conversion yields rise to 30%–50%, controlled by interpersonal presentation dynamics and assessment structuring4. Evaluative ratings in interviews are influenced twice as much by candidate performance constructs—such as communication style and impression management—as by objective job content knowledge11. Ingratiation tactics and value alignment directly drive perceived Person-Organization fit, which serves as the primary indirect catalyst for job offer issuance24. Cold job applicants maximize end-to-end offer conversion by aligning early application mechanics with ATS parsing criteria and balancing technical competence with strategic ingratiation during live interview evaluations2.

#### **Works cited**

> 1. Applicant Drop-Off Rates: Where Candidates Quit (2026) \- Pin, [https://www.pin.com/blog/applicant-drop-off-rates/](https://www.pin.com/blog/applicant-drop-off-rates/)  
> 2. State of Resume Tailoring 2026: Research Report & Data Analysis | TailorForge, [https://tailorforge.com/blog/state-of-resume-tailoring-2026](https://tailorforge.com/blog/state-of-resume-tailoring-2026)  
> 3. 6 Ways to Increase Your Chances of Getting a Job Interview, [https://www.ama.org/marketing-news/6-ways-to-increase-your-chances-of-getting-a-job-interview/](https://www.ama.org/marketing-news/6-ways-to-increase-your-chances-of-getting-a-job-interview/)  
> 4. Engaging the Passive Candidate: Conversion Rates and, [https://www.bristolassoc.com/wp-content/uploads/2026/03/Engaging-the-Passive-Candidate\_-Conversion-Rates-and-Communication-Strategies.pdf](https://www.bristolassoc.com/wp-content/uploads/2026/03/Engaging-the-Passive-Candidate_-Conversion-Rates-and-Communication-Strategies.pdf)  
> 5. 'Hidden workers' may be answer to hiring woes | Springfield Business Journal, [https://sbj.net/stories/hidden-workers-may-be-answer-to-hiring-woes,78404](https://sbj.net/stories/hidden-workers-may-be-answer-to-hiring-woes,78404)  
> 6. Hidden Workers: Uncovering Untapped Talent | Accenture, [https://www.accenture.com/content/dam/accenture/final/a-com-migration/r3-3/pdf/pdf-169/accenture-hidden-worker-report.pdf](https://www.accenture.com/content/dam/accenture/final/a-com-migration/r3-3/pdf/pdf-169/accenture-hidden-worker-report.pdf)  
> 7. Hidden Workers: Untapped Talent \- Harvard Business School, [https://www.hbs.edu/ris/Publication%20Files/hiddenworkers09032021\_Fuller\_white\_paper\_33a2047f-41dd-47b1-9a8d-bd08cf3bfa94.pdf](https://www.hbs.edu/ris/Publication%20Files/hiddenworkers09032021_Fuller_white_paper_33a2047f-41dd-47b1-9a8d-bd08cf3bfa94.pdf)  
> 8. Prof Joe Fuller on the Workforce Trends You Need To Know \- Jacob Morgan, [https://thefutureorganization.com/workforce-trends-you-need-to-know-how-covid-impacted-digital-transformation-why-low-wage-jobs-are-so-hard-to-fill/](https://thefutureorganization.com/workforce-trends-you-need-to-know-how-covid-impacted-digital-transformation-why-low-wage-jobs-are-so-hard-to-fill/)  
> 9. Joseph Fuller on Work in the Twenty-First Century | Harvard Kennedy School, [https://www.hks.harvard.edu/centers/mrcbg/programs/growthpolicy/joseph-fuller-work-twenty-first-century](https://www.hks.harvard.edu/centers/mrcbg/programs/growthpolicy/joseph-fuller-work-twenty-first-century)  
> 10. Decision Traces: What Multi-System Data Fusion Reveals About Institutional Knowledge in Enterprise Hiring \- arXiv, [https://arxiv.org/pdf/2604.19819](https://arxiv.org/pdf/2604.19819)  
> 11. (PDF) An Empirical Review of the Employment Interview Construct Literature \- ResearchGate, [https://www.researchgate.net/publication/228253889\_An\_Empirical\_Review\_of\_the\_Employment\_Interview\_Construct\_Literature](https://www.researchgate.net/publication/228253889_An_Empirical_Review_of_the_Employment_Interview_Construct_Literature)  
> 12. Why Traditional CVs Are Failing Both Recruiters and Candidates, [https://scovai.com/blog/why-traditional-cvs-are-failing/](https://scovai.com/blog/why-traditional-cvs-are-failing/)  
> 13. Why Finding a Job Gets Harder: Applications vs Interviews | Banca d'Italia, [https://www.bancaditalia.it/pubblicazioni/altri-atti-convegni/2022-5th-cepr/Lalive\_Why\_Finding\_a\_Job\_Gets\_Harder\_Applications\_vs\_Interviews.pdf](https://www.bancaditalia.it/pubblicazioni/altri-atti-convegni/2022-5th-cepr/Lalive_Why_Finding_a_Job_Gets_Harder_Applications_vs_Interviews.pdf)  
> 14. Ethnic discrimination in Germany's labour market: a field experiment \- EconStor, [https://www.econstor.eu/bitstream/10419/36152/1/61883852X.pdf](https://www.econstor.eu/bitstream/10419/36152/1/61883852X.pdf)  
> 15. Gig-jobs: stepping stones or dead ends? \- IFAU, [https://www.ifau.se/globalassets/pdf/se/2020/wp-2020-23-gig-jobs-stepping-stones-or-dead-ends.pdf](https://www.ifau.se/globalassets/pdf/se/2020/wp-2020-23-gig-jobs-stepping-stones-or-dead-ends.pdf)  
> 16. A Comparative Analysis of the Reception of Immigrants into Finnish Working Life in 2016 and 2024, [https://julkaisut.valtioneuvosto.fi/bitstreams/86041fcf-c2c4-4600-98f3-4102db5816f8/download](https://julkaisut.valtioneuvosto.fi/bitstreams/86041fcf-c2c4-4600-98f3-4102db5816f8/download)  
> 17. Discrimination against immigrants – measurement, incidence and policy instruments \- Global Forum on Migration and Development, [https://www.gfmd.org/sites/g/files/tmzbdl1801/files/documents/gfmd\_turkey2014-2015\_tm2\_contribution\_oecd1.pdf](https://www.gfmd.org/sites/g/files/tmzbdl1801/files/documents/gfmd_turkey2014-2015_tm2_contribution_oecd1.pdf)  
> 18. WP-20-46 Comparative Perspectives on Racial Discrimination in Hiring: The Rise of Field Experiments, [https://www.ipr.northwestern.edu/documents/working-papers/2020/wp-20-46rev3..pdf](https://www.ipr.northwestern.edu/documents/working-papers/2020/wp-20-46rev3..pdf)  
> 19. Gig-jobs: Stepping stones or dead ends? \- EconStor, [https://www.econstor.eu/handle/10419/246032](https://www.econstor.eu/handle/10419/246032)  
> 20. (PDF) Self-Employed But Looking: A Labor Market Experiment \- ResearchGate, [https://www.researchgate.net/publication/255858077\_Self-Employed\_But\_Looking\_A\_Labor\_Market\_Experiment](https://www.researchgate.net/publication/255858077_Self-Employed_But_Looking_A_Labor_Market_Experiment)  
> 21. Interdisciplinary narratives on artificial intelligence & personnel selection systems, [https://www.tandfonline.com/doi/full/10.1080/09585192.2025.2568782](https://www.tandfonline.com/doi/full/10.1080/09585192.2025.2568782)  
> 22. Racial Discrimination in Context: The Role of Organizational Policies and Practices in Hiring Discrimination1 | American Journal of Sociology: Vol 131, No 4, [https://www.journals.uchicago.edu/doi/10.1086/739291](https://www.journals.uchicago.edu/doi/10.1086/739291)  
> 23. What leaders can do to hire Hidden Workers and close skill and talent gaps, building better companies in the process \- Accenture, [https://www.accenture.com/content/dam/accenture/final/a-com-migration/pdf/accenture-uncover-missed-talent-pools-improve-diversity.pdf](https://www.accenture.com/content/dam/accenture/final/a-com-migration/pdf/accenture-uncover-missed-talent-pools-improve-diversity.pdf)  
> 24. WHAT APPLICANTS NEED TO KNOW ABOUT THE IN- TERVIEWING PROCESS: Separating Fact from Fiction by James A. Tan and Kenneth E. Graha, [https://homepages.se.edu/cvonbergen/files/2013/11/What-Applicants-Need-to-Know-about-the-Interviewing-Process-Separating-Fact-from-Fiction.pdf](https://homepages.se.edu/cvonbergen/files/2013/11/What-Applicants-Need-to-Know-about-the-Interviewing-Process-Separating-Fact-from-Fiction.pdf)  
> 25. DISSERTATION IMPRESSION MANAGEMENT MANIFESTED ON LINKEDIN AND IN RESUMES Submitted by Lauren Cotter Department of Psychology In \- Mountain Scholar, [https://mountainscholar.org/bitstreams/2fc4c94c-3efc-4ea9-816e-68e1e5c3c845/download](https://mountainscholar.org/bitstreams/2fc4c94c-3efc-4ea9-816e-68e1e5c3c845/download)  
> 26. Personal values and intended self-presentation during job interviews: A cross-cultural comparison \- Tilburg University Institutional Repository, [https://repository.tilburguniversity.edu/bitstreams/6068c835-cb2f-4950-a205-d262202a5fe2/download](https://repository.tilburguniversity.edu/bitstreams/6068c835-cb2f-4950-a205-d262202a5fe2/download)  
> 27. Interviewers' perceptions of impression management in employment interviews | Journal of Managerial Psychology | Emerald Publishing, [https://www.emerald.com/jmp/article/29/2/141/230955/Interviewers-perceptions-of-impression-management](https://www.emerald.com/jmp/article/29/2/141/230955/Interviewers-perceptions-of-impression-management)  
> 28. The Relation Between Deceptive Impression Management ... \- Ovid, [https://www.ovid.com/journals/cjbes/pdf/10.1037/cbs0000223\~the-relation-between-deceptive-impression-management-and](https://www.ovid.com/journals/cjbes/pdf/10.1037/cbs0000223~the-relation-between-deceptive-impression-management-and)  
> 29. The use of impression management tactics in structured interviews: A function of question type? \- ResearchGate, [https://www.researchgate.net/publication/298100679\_The\_use\_of\_impression\_management\_tactics\_in\_structured\_interviews\_A\_function\_of\_question\_type](https://www.researchgate.net/publication/298100679_The_use_of_impression_management_tactics_in_structured_interviews_A_function_of_question_type)

[image1]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAkAAAAXCAYAAADZTWX7AAAAgUlEQVR4XmNgGAWkAD4g9gViRiifGYjdgJgfpgAkUQ7EvED8H4i/AXECEM+B8sFAEYiVgFgcKjiBAaIRxIYr8gNiDiA2BuL3QKwDFb8BxMdgimAAZHwruiAyEATi0wwQU3GCIAaI/SDH4wSTGJAciQvcBeKX6ILoQAGIZdEFhzwAADyPE5FynCGTAAAAAElFTkSuQmCC>