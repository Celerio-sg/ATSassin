use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::scraper::JobSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialPost {
    pub platform: String,
    pub author: String,
    pub content: String,
    pub url: String,
    pub posted_at: Option<DateTime<Utc>>,
    pub job_links: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialJobLead {
    pub title: String,
    pub company: String,
    pub location: String,
    pub url: String,
    pub source_platform: String,
    pub author: String,
    pub posted_at: Option<DateTime<Utc>>,
    pub snippet: String,
}

pub struct SocialScraper {
    client: reqwest::Client,
}

impl SocialScraper {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("ATSassin/1.0 (social scraper)")
                .build()
                .expect("Failed to build reqwest client"),
        }
    }

    pub async fn scrape_social(&self, query: &str, limit: usize) -> Result<Vec<SocialJobLead>> {
        debug!("Scraping social platforms for query: {}", query);
        let mut leads = Vec::new();

        let scrapers: [&str; 10] = [
            "Reddit",
            "LinkedIn",
            "Twitter",
            "IndieHackers",
            "Wellfound",
            "RemoteOK",
            "WeWorkRemotely",
            "Telegram",
            "Discord",
            "HN_WhoIsHiring",
        ];

        for name in scrapers.iter() {
            let jobs = match *name {
                "Reddit" => self.scrape_reddit(query, limit).await,
                "LinkedIn" => self.scrape_linkedin(query, limit).await,
                "Twitter" => self.scrape_twitter(query, limit).await,
                "IndieHackers" => self.scrape_indiehackers(query, limit).await,
                "Wellfound" => self.scrape_wellfound(query, limit).await,
                "RemoteOK" => self.scrape_remoteok(query, limit).await,
                "WeWorkRemotely" => self.scrape_weworkremotely(query, limit).await,
                "Telegram" => self.scrape_telegram(query, limit).await,
                "Discord" => self.scrape_discord(query, limit).await,
                "HN_WhoIsHiring" => self.scrape_hn_who_is_hiring(query, limit).await,
                _ => continue,
            };

            match jobs {
                Ok(mut jobs) => {
                    debug!("{} returned {} jobs", name, jobs.len());
                    leads.append(&mut jobs);
                }
                Err(e) => {
                    debug!("{} scrape failed: {}", name, e);
                }
            }
        }

        self.deduplicate(&mut leads);
        leads.truncate(limit);
        Ok(leads)
    }

    fn deduplicate(&self, leads: &mut Vec<SocialJobLead>) {
        let mut seen = std::collections::HashSet::new();
        leads.retain(|lead| {
            let key = (lead.url.clone(), lead.title.clone());
            seen.insert(key)
        });
    }

    async fn scrape_hn_who_is_hiring(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SocialJobLead>> {
        let search_url = format!(
            "https://hn.algolia.com/api/v1/search?query=who+is+hiring+{}&tags=story&hitsPerPage={}",
            urlencoding::encode(query),
            limit
        );

        let resp = self.client.get(&search_url).send().await?;
        let data: serde_json::Value = resp.json().await?;

        let mut jobs = Vec::new();
        if let Some(hits) = data["hits"].as_array() {
            for hit in hits.iter().take(limit) {
                let title = hit["title"].as_str().unwrap_or("").trim().to_string();
                if title.is_empty() || !title.to_lowercase().contains("who is hiring") {
                    continue;
                }

                let url = hit["url"].as_str().unwrap_or(&search_url).to_string();
                let created_at = hit["created_at"].as_str().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                });

                jobs.push(SocialJobLead {
                    title,
                    company: "HN Who is Hiring".to_string(),
                    location: "Remote".to_string(),
                    url,
                    source_platform: "HN_WhoIsHiring".to_string(),
                    author: hit["author"].as_str().unwrap_or("unknown").to_string(),
                    posted_at: created_at,
                    snippet: "HackerNews Who is Hiring thread".to_string(),
                });
            }
        }

        Ok(jobs)
    }

    async fn scrape_reddit(&self, query: &str, limit: usize) -> Result<Vec<SocialJobLead>> {
        let search_url = format!(
            "https://www.reddit.com/search.json?q={}&sort=new&limit={}",
            urlencoding::encode(query),
            limit.min(25)
        );

        let resp = self
            .client
            .get(&search_url)
            .header("User-Agent", "ATSassin/1.0")
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let mut jobs = Vec::new();

        if let Some(children) = data["data"]["children"].as_array() {
            for child in children.iter().take(limit) {
                let post = &child["data"];
                let title = post["title"].as_str().unwrap_or("").trim().to_string();
                if title.is_empty() {
                    continue;
                }

                let subreddit = post["subreddit"].as_str().unwrap_or("reddit");
                let permalink = post["permalink"].as_str().unwrap_or("");
                let url = if permalink.is_empty() {
                    "https://www.reddit.com".to_string()
                } else {
                    format!("https://www.reddit.com{}", permalink)
                };

                let created_at = post["created_utc"]
                    .as_f64()
                    .map(|ts| DateTime::from_timestamp(ts as i64, 0).unwrap_or_else(Utc::now));

                jobs.push(SocialJobLead {
                    title,
                    company: format!("r/{}", subreddit),
                    location: "Remote".to_string(),
                    url,
                    source_platform: "Reddit".to_string(),
                    author: post["author"].as_str().unwrap_or("unknown").to_string(),
                    posted_at: created_at,
                    snippet: format!("Posted on r/{}", subreddit),
                });
            }
        }

        Ok(jobs)
    }

    async fn scrape_linkedin(&self, query: &str, limit: usize) -> Result<Vec<SocialJobLead>> {
        let search_url = format!(
            "https://www.linkedin.com/jobs/search/?keywords={}&limit={}",
            urlencoding::encode(query),
            limit
        );

        let resp = self
            .client
            .get(&search_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await?;

        let text = resp.text().await?;
        let document = scraper::Html::parse_document(&text);

        let mut jobs = Vec::new();

        let selectors = vec![
            ".job-card-container",
            ".jobs-search__results-list",
            ".base-card",
            ".job-search-card",
        ];

        for selector_str in selectors {
            if let Ok(selector) = scraper::Selector::parse(selector_str) {
                for element in document.select(&selector).take(limit) {
                    let title = element
                        .select(
                            &scraper::Selector::parse(
                                ".job-card-title, .result-card__title, h3, .job-title",
                            )
                            .unwrap(),
                        )
                        .next()
                        .and_then(|e| e.text().next())
                        .map(|t| t.trim().to_string())
                        .unwrap_or_default();

                    let company = element
                        .select(
                            &scraper::Selector::parse(
                                ".job-card-subtitle, .result-card__subtitle, h4, .company-name",
                            )
                            .unwrap(),
                        )
                        .next()
                        .and_then(|e| e.text().next())
                        .map(|t| t.trim().to_string())
                        .unwrap_or_else(|| "LinkedIn".to_string());

                    let location = element
                        .select(
                            &scraper::Selector::parse(
                                ".job-card-location, .result-card__location, .location",
                            )
                            .unwrap(),
                        )
                        .next()
                        .and_then(|e| e.text().next())
                        .map(|t| t.trim().to_string())
                        .unwrap_or_else(|| "APAC".to_string());

                    if !title.is_empty() && title.len() > 3 {
                        jobs.push(SocialJobLead {
                            title,
                            company,
                            location,
                            url: search_url.clone(),
                            source_platform: "LinkedIn".to_string(),
                            author: "LinkedIn Post".to_string(),
                            posted_at: Some(Utc::now()),
                            snippet: "Posted on LinkedIn".to_string(),
                        });
                    }
                }
            }
        }

        Ok(jobs)
    }

    async fn scrape_twitter(&self, query: &str, limit: usize) -> Result<Vec<SocialJobLead>> {
        let nitter_instances = [
            "https://nitter.net",
            "https://nitter.privacydev.net",
            "https://nitter.poast.org",
        ];

        let mut jobs = Vec::new();
        for instance in nitter_instances.iter().take(2) {
            let search_url = format!(
                "{}/search?f=tweets&q={}&since=day",
                instance,
                urlencoding::encode(query)
            );

            match self.client.get(&search_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let text = resp.text().await?;
                    let document = scraper::Html::parse_document(&text);

                    if let Ok(selector) = scraper::Selector::parse(".tweet-content") {
                        for element in document.select(&selector).take(limit) {
                            let content = element.text().collect::<String>().trim().to_string();
                            if content.len() > 20 {
                                jobs.push(SocialJobLead {
                                    title: format!(
                                        "Tweet: {}",
                                        content.chars().take(60).collect::<String>()
                                    ),
                                    company: "Twitter/X".to_string(),
                                    location: "Remote".to_string(),
                                    url: search_url.clone(),
                                    source_platform: "Twitter".to_string(),
                                    author: "Twitter User".to_string(),
                                    posted_at: Some(Utc::now()),
                                    snippet: content.chars().take(200).collect::<String>(),
                                });
                            }
                        }
                    }
                    break;
                }
                _ => continue,
            }
        }

        Ok(jobs)
    }

    async fn scrape_indiehackers(&self, query: &str, limit: usize) -> Result<Vec<SocialJobLead>> {
        let search_url = format!(
            "https://www.indiehackers.com/search?q={}",
            urlencoding::encode(query)
        );

        let resp = self.client.get(&search_url).send().await?;
        let text = resp.text().await?;
        let document = scraper::Html::parse_document(&text);

        let mut jobs = Vec::new();
        if let Ok(selector) = scraper::Selector::parse(".post-title, .topic-title") {
            for element in document.select(&selector).take(limit) {
                let title = element
                    .text()
                    .next()
                    .map(|t| t.trim().to_string())
                    .unwrap_or_default();
                if !title.is_empty() && title.len() > 5 {
                    let href = element.value().attr("href").unwrap_or("");
                    let url = if href.is_empty() {
                        search_url.clone()
                    } else {
                        format!("https://www.indiehackers.com{}", href)
                    };

                    jobs.push(SocialJobLead {
                        title,
                        company: "IndieHackers".to_string(),
                        location: "Remote".to_string(),
                        url,
                        source_platform: "IndieHackers".to_string(),
                        author: "IndieHacker".to_string(),
                        posted_at: Some(Utc::now()),
                        snippet: "Posted on IndieHackers".to_string(),
                    });
                }
            }
        }

        Ok(jobs)
    }

    async fn scrape_wellfound(&self, query: &str, limit: usize) -> Result<Vec<SocialJobLead>> {
        let search_url = format!(
            "https://wellfound.com/jobs?query={}",
            urlencoding::encode(query)
        );

        let resp = self.client.get(&search_url).send().await?;
        let text = resp.text().await?;
        let document = scraper::Html::parse_document(&text);

        let mut jobs = Vec::new();
        let selectors = vec![".job-card", ".job-listing", "[data-testid='job-listing']"];

        for selector_str in selectors {
            if let Ok(selector) = scraper::Selector::parse(selector_str) {
                for element in document.select(&selector).take(limit) {
                    let title = element
                        .select(
                            &scraper::Selector::parse("h2, h3, .job-title, a[href*='/jobs/']")
                                .unwrap(),
                        )
                        .next()
                        .and_then(|e| e.text().next())
                        .map(|t| t.trim().to_string())
                        .unwrap_or_default();

                    let company = element
                        .select(&scraper::Selector::parse(".company-name, .startup-name").unwrap())
                        .next()
                        .and_then(|e| e.text().next())
                        .map(|t| t.trim().to_string())
                        .unwrap_or_else(|| "Wellfound Startup".to_string());

                    if !title.is_empty() && title.len() > 3 {
                        let company_clone = company.clone();
                        jobs.push(SocialJobLead {
                            title,
                            company,
                            location: "Remote / Startup".to_string(),
                            url: search_url.clone(),
                            source_platform: "Wellfound".to_string(),
                            author: company_clone,
                            posted_at: Some(Utc::now()),
                            snippet: "Posted on Wellfound".to_string(),
                        });
                    }
                }
            }
        }

        Ok(jobs)
    }

    async fn scrape_remoteok(&self, query: &str, limit: usize) -> Result<Vec<SocialJobLead>> {
        let api_url = "https://remoteok.com/api";

        let resp = self
            .client
            .get(api_url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("RemoteOK API returned {}", resp.status());
        }

        let data: serde_json::Value = resp.json().await?;
        let mut jobs = Vec::new();

        if let Some(jobs_data) = data.as_array() {
            let query_lower = query.to_lowercase();
            for job in jobs_data.iter().take(limit * 3) {
                let title = job["position"].as_str().unwrap_or("");
                let company = job["company"].as_str().unwrap_or("");
                let tags = job["tags"]
                    .as_array()
                    .map(|t| {
                        t.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                let matches_query = query_lower.is_empty()
                    || title.to_lowercase().contains(&query_lower)
                    || company.to_lowercase().contains(&query_lower)
                    || tags.iter().any(|t| t.contains(&query_lower));

                if matches_query && !title.is_empty() {
                    jobs.push(SocialJobLead {
                        title: title.to_string(),
                        company: company.to_string(),
                        location: "Remote".to_string(),
                        url: job["url"].as_str().unwrap_or(api_url).to_string(),
                        source_platform: "RemoteOK".to_string(),
                        author: company.to_string(),
                        posted_at: job["date"].as_str().and_then(|s| {
                            chrono::DateTime::parse_from_rfc3339(s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc))
                        }),
                        snippet: format!("RemoteOK job at {}", company),
                    });
                }

                if jobs.len() >= limit {
                    break;
                }
            }
        }

        Ok(jobs)
    }

    async fn scrape_weworkremotely(&self, query: &str, limit: usize) -> Result<Vec<SocialJobLead>> {
        let search_url = format!(
            "https://weworkremotely.com/remote-jobs/search?term={}",
            urlencoding::encode(query)
        );

        let resp = self.client.get(&search_url).send().await?;
        let text = resp.text().await?;
        let document = scraper::Html::parse_document(&text);

        let mut jobs = Vec::new();
        if let Ok(selector) = scraper::Selector::parse(".job-listings li, .job-listing") {
            for element in document.select(&selector).take(limit) {
                let title = element
                    .select(&scraper::Selector::parse(".job-title, h2, a").unwrap())
                    .next()
                    .and_then(|e| e.text().next())
                    .map(|t| t.trim().to_string())
                    .unwrap_or_default();

                let company = element
                    .select(&scraper::Selector::parse(".company-name, .company").unwrap())
                    .next()
                    .and_then(|e| e.text().next())
                    .map(|t| t.trim().to_string())
                    .unwrap_or_else(|| "WeWorkRemotely".to_string());

                let href = element
                    .select(&scraper::Selector::parse("a[href]").unwrap())
                    .next()
                    .and_then(|e| e.value().attr("href"))
                    .unwrap_or("");

                let url = if href.is_empty() {
                    search_url.clone()
                } else if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://weworkremotely.com{}", href)
                };

                if !title.is_empty() && title.len() > 3 {
                    let company_for_author = company.clone();
                    jobs.push(SocialJobLead {
                        title,
                        company,
                        location: "Remote".to_string(),
                        url,
                        source_platform: "WeWorkRemotely".to_string(),
                        author: company_for_author,
                        posted_at: Some(Utc::now()),
                        snippet: "Posted on WeWorkRemotely".to_string(),
                    });
                }
            }
        }

        Ok(jobs)
    }

    async fn scrape_telegram(&self, query: &str, limit: usize) -> Result<Vec<SocialJobLead>> {
        let telegram_rss_urls = [
            format!(
                "https://rss.app/feeds/v1.1/telegram/channel/{}?query={}",
                urlencoding::encode("remotejobs"),
                urlencoding::encode(query)
            ),
            format!(
                "https://rss.app/feeds/v1.1/telegram/channel/{}?query={}",
                urlencoding::encode("job notifications"),
                urlencoding::encode(query)
            ),
        ];

        let mut jobs = Vec::new();
        for rss_url in telegram_rss_urls.iter().take(2) {
            match self.client.get(rss_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let text = resp.text().await?;
                    let document = scraper::Html::parse_document(&text);

                    if let Ok(selector) = scraper::Selector::parse("item title, .item-title") {
                        for element in document.select(&selector).take(limit) {
                            let title = element
                                .text()
                                .next()
                                .map(|t| t.trim().to_string())
                                .unwrap_or_default();
                            if !title.is_empty() && title.len() > 5 {
                                jobs.push(SocialJobLead {
                                    title,
                                    company: "Telegram".to_string(),
                                    location: "Remote".to_string(),
                                    url: rss_url.clone(),
                                    source_platform: "Telegram".to_string(),
                                    author: "Telegram Channel".to_string(),
                                    posted_at: Some(Utc::now()),
                                    snippet: "Posted on Telegram".to_string(),
                                });
                            }
                        }
                    }
                }
                _ => continue,
            }
        }

        Ok(jobs)
    }

    async fn scrape_discord(&self, query: &str, limit: usize) -> Result<Vec<SocialJobLead>> {
        let discord_search_urls = [
            format!(
                "https://discord.com/search?q={}",
                urlencoding::encode(query)
            ),
            format!(
                "https://discordapp.com/search?q={}",
                urlencoding::encode(query)
            ),
        ];

        let mut jobs = Vec::new();
        for url in discord_search_urls.iter().take(1) {
            match self.client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let text = resp.text().await?;
                    let document = scraper::Html::parse_document(&text);

                    if let Ok(selector) =
                        scraper::Selector::parse(".message-content, .markdown, .message")
                    {
                        for element in document.select(&selector).take(limit) {
                            let content = element.text().collect::<String>().trim().to_string();
                            if content.len() > 20
                                && (content.to_lowercase().contains("hiring")
                                    || content.to_lowercase().contains("job"))
                            {
                                jobs.push(SocialJobLead {
                                    title: format!(
                                        "Discord: {}",
                                        content.chars().take(60).collect::<String>()
                                    ),
                                    company: "Discord".to_string(),
                                    location: "Remote".to_string(),
                                    url: url.clone(),
                                    source_platform: "Discord".to_string(),
                                    author: "Discord User".to_string(),
                                    posted_at: Some(Utc::now()),
                                    snippet: content.chars().take(200).collect::<String>(),
                                });
                            }
                        }
                    }
                }
                _ => continue,
            }
        }

        Ok(jobs)
    }

    pub fn social_leads_to_jobs(leads: Vec<SocialJobLead>) -> Vec<JobSummary> {
        leads
            .into_iter()
            .map(|lead| JobSummary {
                title: format!("[{}] {}", lead.source_platform, lead.title),
                company: lead.company,
                location: lead.location,
                url: lead.url,
                posted_at: lead.posted_at,
                snippet: format!(
                    "{} via {} by {}",
                    lead.snippet, lead.source_platform, lead.author
                ),
                description: None,
            })
            .collect()
    }
}

impl Default for SocialScraper {
    fn default() -> Self {
        Self::new()
    }
}
