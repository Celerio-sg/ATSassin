use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};

#[async_trait]
pub trait BrowserAutomation: Send + Sync {
    async fn navigate(&self, url: &str) -> Result<String>;
    async fn fill(&self, selector: &str, value: &str) -> Result<()>;
    async fn click(&self, selector: &str) -> Result<()>;
    async fn scrape(&self, selector: &str) -> Result<Vec<String>>;
    async fn screenshot(&self, _path: &str) -> Result<()> {
        anyhow::bail!("Screenshots not supported in HTTP mode")
    }
}

pub struct NoOpAutomation;

#[async_trait]
impl BrowserAutomation for NoOpAutomation {
    async fn navigate(&self, _url: &str) -> Result<String> {
        anyhow::bail!("Browser automation not enabled")
    }
    async fn fill(&self, _selector: &str, _value: &str) -> Result<()> {
        anyhow::bail!("Browser automation not enabled")
    }
    async fn click(&self, _selector: &str) -> Result<()> {
        anyhow::bail!("Browser automation not enabled")
    }
    async fn scrape(&self, _selector: &str) -> Result<Vec<String>> {
        anyhow::bail!("Browser automation not enabled")
    }
}

pub struct HttpAutomation {
    pub client: Client,
    pub base_url: String,
}

#[async_trait]
impl BrowserAutomation for HttpAutomation {
    async fn navigate(&self, url: &str) -> Result<String> {
        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("HTTP {} for {}", resp.status(), url);
        }
        let body = resp.text().await?;
        Ok(body)
    }

    async fn fill(&self, _selector: &str, _value: &str) -> Result<()> {
        anyhow::bail!("Form submission not supported in HTTP-only mode")
    }

    async fn click(&self, _selector: &str) -> Result<()> {
        anyhow::bail!("Form submission not supported in HTTP-only mode")
    }

    async fn scrape(&self, _selector: &str) -> Result<Vec<String>> {
        anyhow::bail!(
            "Scrape not supported via trait; use extract_text or extract_attribute directly"
        )
    }
}

impl HttpAutomation {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .user_agent("ATSassin/0.1 (local-first job search)")
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.into(),
        }
    }

    pub async fn extract_text(&self, html: &str, selector: &str) -> Result<Vec<String>> {
        let sel =
            Selector::parse(selector).map_err(|e| anyhow::anyhow!("Invalid selector: {}", e))?;
        let document = Html::parse_document(html);
        Ok(document
            .select(&sel)
            .filter_map(|el| el.text().next())
            .map(|s| s.to_string())
            .collect())
    }

    pub async fn extract_attribute(
        &self,
        html: &str,
        selector: &str,
        attr: &str,
    ) -> Result<Vec<String>> {
        let sel =
            Selector::parse(selector).map_err(|e| anyhow::anyhow!("Invalid selector: {}", e))?;
        let document = Html::parse_document(html);
        Ok(document
            .select(&sel)
            .filter_map(|el| el.value().attr(attr))
            .map(|s| s.to_string())
            .collect())
    }
}

pub struct BrowserMcpAutomation {
    pub ws_url: String,
    pub client: Client,
}

#[async_trait]
impl BrowserAutomation for BrowserMcpAutomation {
    async fn navigate(&self, url: &str) -> Result<String> {
        let _payload = serde_json::json!({
            "method": "Page.navigate",
            "params": { "url": url }
        });
        let resp = self
            .send_mcp_command("Target.attachToBrowserTarget", &_payload)
            .await?;
        if let Some(error) = resp.get("error") {
            anyhow::bail!("Browser MCP navigate error: {}", error);
        }
        Ok(format!("Navigated to {}", url))
    }

    async fn fill(&self, selector: &str, value: &str) -> Result<()> {
        let _payload = serde_json::json!({
            "method": "DOM.querySelector",
            "params": { "selector": selector }
        });
        let _resp = self.send_mcp_command("Runtime.evaluate", &serde_json::json!({
            "expression": format!("document.querySelector('{}').value = '{}'", selector, value)
        })).await?;
        Ok(())
    }

    async fn click(&self, selector: &str) -> Result<()> {
        let _resp = self
            .send_mcp_command(
                "Runtime.evaluate",
                &serde_json::json!({
                    "expression": format!("document.querySelector('{}').click()", selector)
                }),
            )
            .await?;
        Ok(())
    }

    async fn scrape(&self, selector: &str) -> Result<Vec<String>> {
        let resp = self.send_mcp_command("Runtime.evaluate", &serde_json::json!({
            "expression": format!("Array.from(document.querySelectorAll('{}')).map(el => el.textContent.trim())", selector)
        })).await?;
        if let Some(result) = resp
            .get("result")
            .and_then(|r| r.get("result"))
            .and_then(|r| r.get("value"))
        {
            if let Some(arr) = result.as_array() {
                return Ok(arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect());
            }
        }
        Ok(vec![])
    }

    async fn screenshot(&self, _path: &str) -> Result<()> {
        let _resp = self
            .send_mcp_command(
                "Page.captureScreenshot",
                &serde_json::json!({
                    "format": "png",
                    "fromSurface": true
                }),
            )
            .await?;
        Ok(())
    }
}

impl BrowserMcpAutomation {
    pub fn new() -> Self {
        Self {
            ws_url: "ws://localhost:9222/devtools/browser".to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    pub fn new_with_ws(ws_url: impl Into<String>) -> Self {
        Self {
            ws_url: ws_url.into(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    async fn send_mcp_command(
        &self,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let resp = self
            .client
            .post("http://localhost:9222/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Browser MCP command failed: HTTP {}", resp.status());
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(data)
    }

    pub async fn is_available(&self) -> bool {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        let result = client
            .get("http://localhost:9222/json/version")
            .send()
            .await;
        result.map(|r| r.status().is_success()).unwrap_or(false)
    }
}

impl Default for BrowserMcpAutomation {
    fn default() -> Self {
        Self::new()
    }
}
