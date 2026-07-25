use crate::pipeline::automation::BrowserAutomation;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResult {
    pub jobs: Vec<JobSummary>,
    pub source: String,
    pub scraped_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub title: String,
    pub company: String,
    pub location: String,
    pub url: String,
    pub posted_at: Option<DateTime<Utc>>,
    pub snippet: String,
    pub description: Option<String>,
}

#[derive(Clone)]
pub struct Scraper {
    pub rate_limit_ms: u64,
    pub user_agent: String,
}

const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

impl Scraper {
    pub fn new(rate_limit_ms: u64, user_agent: String) -> Self {
        Self {
            rate_limit_ms,
            user_agent,
        }
    }

    pub async fn scrape_board(
        &self,
        board: &str,
        query: &str,
        limit: usize,
    ) -> Result<ScrapeResult> {
        self.scrape_board_at(board, query, limit, None).await
    }

    /// Same as `scrape_board`, with an optional target location. Without
    /// this, LinkedIn's guest API silently defaults to a location it
    /// infers server-side - confirmed via real-world testing to
    /// consistently return US postings regardless of query wording, which
    /// is a real bug (not a "try different search terms" problem) for
    /// anyone searching outside the US.
    pub async fn scrape_board_at(
        &self,
        board: &str,
        query: &str,
        limit: usize,
        location: Option<&str>,
    ) -> Result<ScrapeResult> {
        debug!(
            "Scraping board: {} for query: {} location: {:?}",
            board, query, location
        );
        let mut jobs = match board {
            "linkedin" => self
                .scrape_linkedin(query, limit, location)
                .await
                .unwrap_or_default(),
            "seek" => self.scrape_seek(query, limit).await.unwrap_or_default(),
            "indeed" => self.scrape_indeed(query, limit).await.unwrap_or_default(),
            "glassdoor" => self
                .scrape_glassdoor(query, limit)
                .await
                .unwrap_or_default(),
            // Maps to the "who is hiring" thread search, not generic HN story
            // search - the latter returned irrelevant front-page stories
            // (e.g. "Show HN: ..." posts) instead of job postings.
            "hackernews" => self
                .scrape_social_platform(query, limit, "HN_WhoIsHiring")
                .await
                .unwrap_or_default(),
            "reddit" => self
                .scrape_social_platform(query, limit, "Reddit")
                .await
                .unwrap_or_default(),
            "twitter" => self
                .scrape_social_platform(query, limit, "Twitter")
                .await
                .unwrap_or_default(),
            "indiehackers" => self
                .scrape_social_platform(query, limit, "IndieHackers")
                .await
                .unwrap_or_default(),
            "wellfound" => self
                .scrape_social_platform(query, limit, "Wellfound")
                .await
                .unwrap_or_default(),
            "remoteok" => self
                .scrape_social_platform(query, limit, "RemoteOK")
                .await
                .unwrap_or_default(),
            "weworkremotely" => self
                .scrape_social_platform(query, limit, "WeWorkRemotely")
                .await
                .unwrap_or_default(),
            "telegram" => self
                .scrape_social_platform(query, limit, "Telegram")
                .await
                .unwrap_or_default(),
            "discord" => self
                .scrape_social_platform(query, limit, "Discord")
                .await
                .unwrap_or_default(),
            "hn_whoishiring" => self
                .scrape_social_platform(query, limit, "HN_WhoIsHiring")
                .await
                .unwrap_or_default(),
            "social" => self.scrape_social(query, limit).await.unwrap_or_default(),
            "companies" => self
                .scrape_companies(query, limit)
                .await
                .unwrap_or_default(),
            // "greenhouse:<company-slug>" / "lever:<company-slug>" /
            // "ashby:<company-slug>" - real public, documented, ToS-friendly
            // company career-page APIs. Unlike LinkedIn's guest-scrape,
            // these are meant to be consumed programmatically. They return
            // a company's full job list, so we filter it locally by query.
            _ if board.starts_with("greenhouse:") => self
                .scrape_greenhouse(&board["greenhouse:".len()..], query, limit)
                .await
                .unwrap_or_default(),
            _ if board.starts_with("lever:") => self
                .scrape_lever(&board["lever:".len()..], query, limit)
                .await
                .unwrap_or_default(),
            _ if board.starts_with("ashby:") => self
                .scrape_ashby(&board["ashby:".len()..], query, limit)
                .await
                .unwrap_or_default(),
            _ => anyhow::bail!("Unknown board: {}", board),
        };

        // Only the real job-board scrapers get a headless/MCP-browser retry — social
        // aggregators already hit real APIs directly and have nothing to retry.
        if jobs.is_empty() && matches!(board, "linkedin" | "seek" | "indeed" | "glassdoor") {
            debug!(
                "Primary scraper returned 0 jobs for board: {}, trying headless/MCP fallback",
                board
            );
            jobs = self
                .scrape_with_headless(board, query, limit)
                .await
                .unwrap_or_default();
        }

        // Honest failure: an empty board is a valid, non-fatal result. We never
        // substitute another board's (or a social aggregator's) results here -
        // that was the root cause of every board printing fabricated "[LinkedIn]"
        // listings regardless of which board was requested.
        Ok(ScrapeResult {
            jobs,
            source: board.to_string(),
            scraped_at: Utc::now(),
        })
    }

    async fn scrape_social(&self, query: &str, limit: usize) -> Result<Vec<JobSummary>> {
        let social_scraper = crate::pipeline::social_scraper::SocialScraper::new();
        let leads = social_scraper.scrape_social(query, limit).await?;
        Ok(crate::pipeline::social_scraper::SocialScraper::social_leads_to_jobs(leads))
    }

    async fn scrape_social_platform(
        &self,
        query: &str,
        limit: usize,
        platform: &str,
    ) -> Result<Vec<JobSummary>> {
        let social_scraper = crate::pipeline::social_scraper::SocialScraper::new();
        let leads = social_scraper.scrape_social(query, limit).await?;
        let platform_leads: Vec<crate::pipeline::social_scraper::SocialJobLead> = leads
            .into_iter()
            .filter(|l| l.source_platform == platform)
            .collect();
        Ok(crate::pipeline::social_scraper::SocialScraper::social_leads_to_jobs(platform_leads))
    }

    /// LinkedIn's public "guest" job-search API - no authentication required.
    /// Undocumented and can change without notice; any parse failure degrades
    /// to an empty result rather than fabricating data.
    async fn scrape_linkedin(
        &self,
        query: &str,
        limit: usize,
        location: Option<&str>,
    ) -> Result<Vec<JobSummary>> {
        let client = reqwest::Client::builder()
            .user_agent(BROWSER_USER_AGENT)
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let mut search_url = format!(
            "https://www.linkedin.com/jobs-guest/jobs/api/seeMoreJobPostings/search?keywords={}&start=0",
            urlencoding::encode(query)
        );
        if let Some(loc) = location.filter(|l| !l.trim().is_empty()) {
            search_url.push_str(&format!("&location={}", urlencoding::encode(loc)));
        }

        let resp = client.get(&search_url).send().await?;
        if !resp.status().is_success() {
            debug!("LinkedIn guest API returned HTTP {}", resp.status());
            return Ok(vec![]);
        }
        let html = resp.text().await?;

        // Scoped so every scraper::Html/Selector (none of which are Send -
        // they use non-atomic Rc/Cell internally) is dropped before the
        // description-fetch loop below crosses an .await point. Without this,
        // the whole function becomes a non-Send future and can't be
        // tokio::spawn'd (as the TUI's async scan does).
        let mut jobs = Vec::new();
        {
            let document = scraper::Html::parse_document(&html);
            let card_sel = match scraper::Selector::parse("li") {
                Ok(s) => s,
                Err(_) => return Ok(vec![]),
            };
            let title_sel = scraper::Selector::parse("h3.base-search-card__title").ok();
            let company_sel = scraper::Selector::parse(
                "h4.base-search-card__subtitle a, h4.base-search-card__subtitle",
            )
            .ok();
            let location_sel = scraper::Selector::parse("span.job-search-card__location").ok();
            let link_sel = scraper::Selector::parse("a.base-card__full-link").ok();
            // LinkedIn's guest cards carry the real posting date as an ISO
            // `datetime` attribute on this <time> element - previously
            // ignored, which meant every job silently got "posted now"
            // regardless of how old the listing actually was.
            let date_sel = scraper::Selector::parse(
                "time.job-search-card__listdate, time.job-search-card__listdate--new",
            )
            .ok();

            for card in document.select(&card_sel).take(limit) {
                let title = title_sel
                    .as_ref()
                    .and_then(|s| card.select(s).next())
                    .map(|e| e.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();
                if title.is_empty() {
                    continue;
                }

                let raw_url = link_sel
                    .as_ref()
                    .and_then(|s| card.select(s).next())
                    .and_then(|e| e.value().attr("href"))
                    .unwrap_or("");
                if raw_url.is_empty() {
                    continue;
                }
                let url = raw_url.split('?').next().unwrap_or(raw_url).to_string();

                let company = company_sel
                    .as_ref()
                    .and_then(|s| card.select(s).next())
                    .map(|e| e.text().collect::<String>().trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "Unknown".to_string());

                let location = location_sel
                    .as_ref()
                    .and_then(|s| card.select(s).next())
                    .map(|e| e.text().collect::<String>().trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "Unknown".to_string());

                let posted_at = date_sel
                    .as_ref()
                    .and_then(|s| card.select(s).next())
                    .and_then(|e| e.value().attr("datetime"))
                    .and_then(|d| {
                        DateTime::parse_from_rfc3339(&format!("{d}T00:00:00Z"))
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    })
                    .unwrap_or_else(Utc::now);

                jobs.push(JobSummary {
                    title,
                    company,
                    location,
                    url,
                    posted_at: Some(posted_at),
                    snippet: "LinkedIn job posting".to_string(),
                    description: None,
                });
            }
        }

        // Best-effort per-job description fetch, rate-limited between requests.
        // Never fails the scan - a job without a fetched description just keeps
        // its snippet.
        for job in jobs.iter_mut() {
            if self.rate_limit_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.rate_limit_ms)).await;
            }
            let id = job
                .url
                .trim_end_matches('/')
                .rsplit('-')
                .next()
                .unwrap_or("");
            if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let desc_url = format!(
                "https://www.linkedin.com/jobs-guest/jobs/api/jobPosting/{}",
                id
            );
            let Ok(resp) = client.get(&desc_url).send().await else {
                continue;
            };
            if !resp.status().is_success() {
                continue;
            }
            let Ok(desc_html) = resp.text().await else {
                continue;
            };
            // Tier 1: schema.org JobPosting JSON-LD, if the page embeds it -
            // real structured data, free, and doesn't break when a site
            // reskins its markup. Tier 2 (CSS selector) is the fallback.
            if let Some(desc) = extract_jsonld_description(&desc_html) {
                job.description = Some(desc);
                continue;
            }
            let doc = scraper::Html::parse_document(&desc_html);
            if let Ok(sel) = scraper::Selector::parse("div.show-more-less-html__markup") {
                if let Some(el) = doc.select(&sel).next() {
                    let text = el.text().collect::<String>().trim().to_string();
                    if !text.is_empty() {
                        job.description = Some(text);
                    }
                }
            }
        }

        Ok(jobs)
    }

    /// Seek's public SPA JSON search API. Undocumented and can change; any
    /// parse failure degrades to an empty result.
    async fn scrape_seek(&self, query: &str, limit: usize) -> Result<Vec<JobSummary>> {
        let client = reqwest::Client::builder()
            .user_agent(BROWSER_USER_AGENT)
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let url = format!(
            "https://www.seek.com.au/api/chalice-search/v4/search?siteKey=AU-Main&keywords={}&pageSize={}",
            urlencoding::encode(query),
            limit
        );

        let resp = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;
        if !resp.status().is_success() {
            debug!("Seek search API returned HTTP {}", resp.status());
            return Ok(vec![]);
        }
        let data: serde_json::Value = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                debug!("Seek search API returned non-JSON response: {}", e);
                return Ok(vec![]);
            }
        };
        let items = match data["data"].as_array() {
            Some(arr) => arr,
            None => {
                debug!("Seek search API response missing 'data' array");
                return Ok(vec![]);
            }
        };

        let mut jobs = Vec::new();
        for item in items.iter().take(limit) {
            let title = item["title"].as_str().unwrap_or("").trim().to_string();
            if title.is_empty() {
                continue;
            }
            let id = match item["id"].as_str() {
                Some(s) => s.to_string(),
                None => match item["id"].as_i64() {
                    Some(n) => n.to_string(),
                    None => continue,
                },
            };

            let company = item["advertiser"]["description"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or("Unknown")
                .to_string();

            let loc = item["location"].as_str().unwrap_or("");
            let area = item["area"].as_str().unwrap_or("");
            let location = [loc, area]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ");
            let location = if location.is_empty() {
                "Unknown".to_string()
            } else {
                location
            };

            let description = item["teaser"].as_str().map(|s| s.to_string());

            jobs.push(JobSummary {
                title,
                company,
                location,
                url: format!("https://www.seek.com.au/job/{}", id),
                posted_at: Some(Utc::now()),
                snippet: "SEEK job posting".to_string(),
                description,
            });
        }

        Ok(jobs)
    }

    /// Indeed is Cloudflare-protected; plain HTTP scraping usually returns 0
    /// results. We still make one honest attempt, then warn instead of
    /// fabricating results.
    /// Greenhouse's public job-board API - returns a company's full open-role
    /// list; we filter it locally by `query` since the API itself has no
    /// keyword search. `content=true` gets the HTML job description inline
    /// (real, free, structured - no LLM/scraping needed to read it).
    async fn scrape_greenhouse(
        &self,
        company: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<JobSummary>> {
        let client = reqwest::Client::builder()
            .user_agent(&self.user_agent)
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let url = format!(
            "https://boards-api.greenhouse.io/v1/boards/{}/jobs?content=true",
            urlencoding::encode(company)
        );
        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            debug!(
                "Greenhouse board '{}' returned HTTP {}",
                company,
                resp.status()
            );
            return Ok(vec![]);
        }
        let data: serde_json::Value = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                debug!("Greenhouse board '{}' returned non-JSON: {}", company, e);
                return Ok(vec![]);
            }
        };
        let Some(jobs_arr) = data["jobs"].as_array() else {
            return Ok(vec![]);
        };

        let query_lower = query.to_lowercase();
        let match_all = query_lower.is_empty() || query_lower == "general";
        let mut out = Vec::new();
        for item in jobs_arr {
            let title = item["title"].as_str().unwrap_or("").to_string();
            if title.is_empty() || (!match_all && !title.to_lowercase().contains(&query_lower)) {
                continue;
            }
            let url = item["absolute_url"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let location = item["location"]["name"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or("Unknown")
                .to_string();
            let description = item["content"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(|html| {
                    let plain: String = scraper::Html::parse_fragment(html)
                        .root_element()
                        .text()
                        .collect();
                    let plain = plain.trim();
                    if plain.is_empty() {
                        html.trim().to_string()
                    } else {
                        plain.to_string()
                    }
                });
            out.push(JobSummary {
                title,
                company: company.to_string(),
                location,
                url,
                posted_at: None,
                snippet: "Greenhouse job posting".to_string(),
                description,
            });
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }

    /// Lever's public postings API - same shape as Greenhouse: full company
    /// list, filtered locally by query.
    async fn scrape_lever(
        &self,
        company: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<JobSummary>> {
        let client = reqwest::Client::builder()
            .user_agent(&self.user_agent)
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let url = format!(
            "https://api.lever.co/v0/postings/{}?mode=json",
            urlencoding::encode(company)
        );
        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            debug!("Lever board '{}' returned HTTP {}", company, resp.status());
            return Ok(vec![]);
        }
        let data: serde_json::Value = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                debug!("Lever board '{}' returned non-JSON: {}", company, e);
                return Ok(vec![]);
            }
        };
        let Some(arr) = data.as_array() else {
            return Ok(vec![]);
        };

        let query_lower = query.to_lowercase();
        let match_all = query_lower.is_empty() || query_lower == "general";
        let mut out = Vec::new();
        for item in arr {
            let title = item["text"].as_str().unwrap_or("").to_string();
            if title.is_empty() || (!match_all && !title.to_lowercase().contains(&query_lower)) {
                continue;
            }
            let url = item["hostedUrl"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let location = item["categories"]["location"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or("Unknown")
                .to_string();
            let description = item["descriptionPlain"]
                .as_str()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            out.push(JobSummary {
                title,
                company: company.to_string(),
                location,
                url,
                posted_at: None,
                snippet: "Lever job posting".to_string(),
                description,
            });
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }

    /// Ashby's public job-board API - same pattern again.
    async fn scrape_ashby(
        &self,
        company: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<JobSummary>> {
        let client = reqwest::Client::builder()
            .user_agent(&self.user_agent)
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let url = format!(
            "https://api.ashbyhq.com/posting-api/job-board/{}",
            urlencoding::encode(company)
        );
        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            debug!("Ashby board '{}' returned HTTP {}", company, resp.status());
            return Ok(vec![]);
        }
        let data: serde_json::Value = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                debug!("Ashby board '{}' returned non-JSON: {}", company, e);
                return Ok(vec![]);
            }
        };
        let Some(arr) = data["jobs"].as_array() else {
            return Ok(vec![]);
        };

        let query_lower = query.to_lowercase();
        let match_all = query_lower.is_empty() || query_lower == "general";
        let mut out = Vec::new();
        for item in arr {
            let title = item["title"].as_str().unwrap_or("").to_string();
            if title.is_empty() || (!match_all && !title.to_lowercase().contains(&query_lower)) {
                continue;
            }
            let url = item["jobUrl"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let location = item["location"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or("Unknown")
                .to_string();
            let description = item["descriptionPlain"]
                .as_str()
                .or_else(|| item["descriptionHtml"].as_str())
                .map(|s| {
                    let plain: String = scraper::Html::parse_fragment(s)
                        .root_element()
                        .text()
                        .collect();
                    let plain = plain.trim();
                    if plain.is_empty() {
                        s.trim().to_string()
                    } else {
                        plain.to_string()
                    }
                });
            out.push(JobSummary {
                title,
                company: company.to_string(),
                location,
                url,
                posted_at: None,
                snippet: "Ashby job posting".to_string(),
                description,
            });
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }

    /// Sweeps every company in the curated `company_directory` concurrently,
    /// hitting each one's public Greenhouse JSON API directly - zero LLM
    /// tokens, no external agent/websearch dependency, same technique as
    /// career-ops's `scan.mjs` distilled to first principles (see
    /// `company_directory.rs` for the full rationale). Unlike a sequential
    /// sweep, every company is fetched in parallel, so wall-clock time is
    /// bounded by the slowest single request rather than N x latency.
    async fn scrape_companies(&self, query: &str, limit: usize) -> Result<Vec<JobSummary>> {
        use crate::pipeline::company_directory::GREENHOUSE_COMPANIES;
        let start = std::time::Instant::now();

        let mut handles = Vec::with_capacity(GREENHOUSE_COMPANIES.len());
        for (_name, slug) in GREENHOUSE_COMPANIES {
            let scraper = self.clone();
            let slug = slug.to_string();
            let query = query.to_string();
            handles.push(tokio::spawn(async move {
                scraper
                    .scrape_greenhouse(&slug, &query, limit)
                    .await
                    .unwrap_or_default()
            }));
        }

        let mut jobs = Vec::new();
        let mut companies_with_matches = 0usize;
        for handle in handles {
            let Ok(company_jobs) = handle.await else {
                continue;
            };
            if !company_jobs.is_empty() {
                companies_with_matches += 1;
            }
            jobs.extend(company_jobs);
        }

        // Honest, not silent: exactly how many of the curated companies had
        // a role matching this query, in exactly how long. Mirrors the
        // transparency of the indeed/glassdoor honest-failure warnings.
        eprintln!(
            "companies: swept {} real company job boards directly (zero LLM tokens) in {:.1}s - {} had roles matching \"{}\"",
            GREENHOUSE_COMPANIES.len(),
            start.elapsed().as_secs_f64(),
            companies_with_matches,
            query,
        );

        // This board sweeps many companies at once, so allow a higher
        // aggregate cap than the per-board `limit` argument implies.
        jobs.truncate(limit.saturating_mul(10).max(limit));
        Ok(jobs)
    }

    async fn scrape_indeed(&self, query: &str, limit: usize) -> Result<Vec<JobSummary>> {
        let url = format!(
            "https://au.indeed.com/jobs?q={}",
            urlencoding::encode(query)
        );
        let jobs = self
            .try_generic_html_scrape(&url, &[".jobTitle", "h2.jobTitle", "a[data-jk]"], limit)
            .await
            .unwrap_or_default();
        if jobs.is_empty() {
            eprintln!(
                "warning: 'indeed' blocked automated access - no results. Use --boards linkedin,seek,social or open {} manually.",
                url
            );
        }
        Ok(jobs)
    }

    /// Glassdoor is also bot-protected; same honest-attempt-then-warn approach.
    async fn scrape_glassdoor(&self, query: &str, limit: usize) -> Result<Vec<JobSummary>> {
        let url = format!(
            "https://www.glassdoor.com/Job/jobs.htm?sc.keyword={}",
            urlencoding::encode(query)
        );
        let jobs = self
            .try_generic_html_scrape(
                &url,
                &[
                    ".jobTitle",
                    ".JobCard_jobTitle___a7XN",
                    "a[data-test='job-title']",
                ],
                limit,
            )
            .await
            .unwrap_or_default();
        if jobs.is_empty() {
            eprintln!(
                "warning: 'glassdoor' blocked automated access - no results. Use --boards linkedin,seek,social or open {} manually.",
                url
            );
        }
        Ok(jobs)
    }

    async fn try_generic_html_scrape(
        &self,
        url: &str,
        selectors: &[&str],
        limit: usize,
    ) -> Result<Vec<JobSummary>> {
        let client = reqwest::Client::builder()
            .user_agent(&self.user_agent)
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let resp = client.get(url).send().await?;
        if !resp.status().is_success() {
            return Ok(vec![]);
        }
        let text = resp.text().await?;
        let document = scraper::Html::parse_document(&text);

        let mut jobs = Vec::new();
        for sel_str in selectors {
            if let Ok(selector) = scraper::Selector::parse(sel_str) {
                for element in document.select(&selector).take(limit) {
                    let title = element.text().collect::<String>().trim().to_string();
                    if !title.is_empty() && title.len() > 4 {
                        jobs.push(JobSummary {
                            title,
                            company: "Unknown".to_string(),
                            location: "Unknown".to_string(),
                            url: url.to_string(),
                            posted_at: Some(Utc::now()),
                            snippet: String::new(),
                            description: None,
                        });
                    }
                }
            }
            if !jobs.is_empty() {
                break;
            }
        }

        Ok(jobs)
    }

    async fn scrape_with_headless(
        &self,
        board: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<JobSummary>> {
        debug!("Attempting headless/MCP fallback for board: {}", board);
        let url = match board {
            "linkedin" => format!(
                "https://www.linkedin.com/jobs/search/?keywords={}",
                urlencoding::encode(query)
            ),
            "seek" => format!(
                "https://www.seek.com.au/jobs?keywords={}",
                urlencoding::encode(query)
            ),
            "indeed" => format!(
                "https://au.indeed.com/jobs?q={}",
                urlencoding::encode(query)
            ),
            "glassdoor" => format!(
                "https://www.glassdoor.com/Job/jobs.htm?sc.keyword={}",
                urlencoding::encode(query)
            ),
            _ => anyhow::bail!("Unknown board for headless fallback: {}", board),
        };

        // 1. Try Browser MCP Automation
        let mcp = crate::pipeline::automation::BrowserMcpAutomation::new();
        if mcp.is_available().await {
            debug!(
                "Browser MCP detected at localhost:9222. Navigating to {}",
                url
            );
            if let Ok(titles) = mcp.scrape("h1, h2, h3, .jobTitle").await {
                let jobs: Vec<JobSummary> = titles
                    .into_iter()
                    .take(limit)
                    .filter(|t| t.len() > 4)
                    .map(|title| JobSummary {
                        title,
                        company: format!("{} (via MCP Browser)", board),
                        location: "Unknown".to_string(),
                        url: url.clone(),
                        posted_at: Some(Utc::now()),
                        snippet: "Scraped via Chrome DevTools Browser MCP".to_string(),
                        description: None,
                    })
                    .collect();
                if !jobs.is_empty() {
                    return Ok(jobs);
                }
            }
        }

        // 2. Try Fantoccini WebDriver Fallback
        match self.try_fantoccini(&url, limit).await {
            Ok(jobs) if !jobs.is_empty() => Ok(jobs),
            _ => Ok(vec![]),
        }
    }

    async fn try_fantoccini(&self, url: &str, limit: usize) -> Result<Vec<JobSummary>> {
        use fantoccini::Client;
        use std::time::Duration;

        let mut caps = fantoccini::wd::Capabilities::new();
        caps.insert("browserName".to_string(), "chrome".into());
        caps.insert("goog:chromeOptions".to_string(), {
            let mut opts = serde_json::json!({});
            opts["args"] = serde_json::json!(["--headless", "--disable-gpu", "--no-sandbox"]);
            opts
        });

        #[allow(deprecated)]
        let client = Client::new("http://localhost:4444").await?;
        client.goto(url).await?;
        tokio::time::sleep(Duration::from_secs(2)).await;

        let mut jobs = Vec::new();
        for element in client
            .find_all(fantoccini::Locator::Css(
                "h1, h2, h3, .jobTitle, .result-card__title",
            ))
            .await?
            .into_iter()
            .take(limit)
        {
            if let Ok(text) = element.text().await {
                let title = text.trim().to_string();
                if !title.is_empty() && title.len() > 5 {
                    jobs.push(JobSummary {
                        title,
                        company: "Unknown".to_string(),
                        location: "Unknown".to_string(),
                        url: url.to_string(),
                        posted_at: Some(Utc::now()),
                        snippet: "Headless-browser job posting".to_string(),
                        description: None,
                    });
                }
            }
        }

        client.close().await?;
        Ok(jobs)
    }
}

/// Tier 1 of description extraction: schema.org `JobPosting` JSON-LD, which
/// many job pages embed in a `<script type="application/ld+json">` block for
/// search-engine indexing. Real, free, structured data that survives a
/// site's CSS/markup changes - tried before any CSS-selector scraping.
/// Returns `None` (never fabricates) if no such block exists or it doesn't
/// describe a JobPosting.
fn extract_jsonld_description(html: &str) -> Option<String> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse(r#"script[type="application/ld+json"]"#).ok()?;
    for el in document.select(&selector) {
        let text = el.text().collect::<String>();
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if let Some(desc) = find_jobposting_description(&value) {
            return Some(desc);
        }
    }
    None
}

fn find_jobposting_description(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Array(arr) => arr.iter().find_map(find_jobposting_description),
        serde_json::Value::Object(map) => {
            let is_job_posting = map.get("@type").is_some_and(|t| {
                t.as_str() == Some("JobPosting")
                    || t.as_array()
                        .is_some_and(|a| a.iter().any(|x| x.as_str() == Some("JobPosting")))
            });
            if is_job_posting {
                if let Some(desc) = map.get("description").and_then(|d| d.as_str()) {
                    // Descriptions are usually HTML fragments - strip tags
                    // for a plain-text job description.
                    let plain: String = scraper::Html::parse_fragment(desc)
                        .root_element()
                        .text()
                        .collect();
                    let plain = plain.trim();
                    return Some(if plain.is_empty() {
                        desc.trim().to_string()
                    } else {
                        plain.to_string()
                    });
                }
            }
            // Some pages nest JobPosting inside a top-level @graph array.
            map.get("@graph").and_then(find_jobposting_description)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_description_from_jsonld() {
        let html = r#"<html><head>
            <script type="application/ld+json">
            {"@context":"https://schema.org","@type":"JobPosting","title":"Senior Engineer","description":"<p>Build <b>great</b> things.</p>"}
            </script>
        </head><body></body></html>"#;
        assert_eq!(
            extract_jsonld_description(html),
            Some("Build great things.".to_string())
        );
    }

    #[test]
    fn extracts_description_from_jsonld_array() {
        let html = r#"<html><head>
            <script type="application/ld+json">
            [{"@type":"BreadcrumbList"},{"@type":"JobPosting","description":"Plain text description."}]
            </script>
        </head></html>"#;
        assert_eq!(
            extract_jsonld_description(html),
            Some("Plain text description.".to_string())
        );
    }

    #[test]
    fn returns_none_without_jobposting() {
        let html = r#"<html><head>
            <script type="application/ld+json">{"@type":"Organization","name":"Acme"}</script>
        </head></html>"#;
        assert_eq!(extract_jsonld_description(html), None);
    }

    #[test]
    fn returns_none_without_any_jsonld() {
        let html = "<html><head></head><body>No structured data here</body></html>";
        assert_eq!(extract_jsonld_description(html), None);
    }
}
