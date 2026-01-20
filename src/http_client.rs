use std::error::Error;

const PAGE_SIZE_LIMIT_MB: u64 = 10;

pub struct HTTPClient {
    client: reqwest::Client,
}

impl HTTPClient {
    pub fn new() -> Result<HTTPClient, Box<dyn Error>> {
        let client = reqwest::Client::builder()
            .user_agent("PalimpCralwer/0.1")
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|err| format!("Failed to initialize HTTP request client (reqwest):\n{}", err))?;

        Ok(HTTPClient { client })
    }

    pub async fn get_html(&self, url: &str) -> Result<(String, String), Box<dyn Error>> {
        let url = url.trim();

        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|err| format!("HTTP client could not connect with {}:\n{}", url, err))?;

        if !response.status().is_success() {
            return Err(format!("Server returned an error for {}: {}", url, response.status()).into());
        }

        if let Some(len) = response.content_length() {
            let max_size = PAGE_SIZE_LIMIT_MB * 1024 * 1024;
            if len > max_size {
                return Err(
                    format!("HTML page is unusually large ({} bytes) for URL: {}. The page size limit is {} MB.",
                    len,
                    url,
                    PAGE_SIZE_LIMIT_MB
                ).into());
            }
        }

        let content_type = response.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("text/html") {
            return Err(format!("Document type is not text/html, but {} for: {}", content_type, url).into());
        }

        let final_url = response.url().as_str().to_string();

        let response_text = response.text()
            .await
            .map_err(|err| format!("Could not read response text for {}: {}", url, err))?;

        Ok((final_url, response_text))
    }

    pub async fn get_sitemap(&self, url: &str) -> Result<String, Box<dyn Error>> {
        let url = url.trim();

        let response = self.client
            .get(url)
            .header("Accept", "application/xml, text/xml, */*")
            .send()
            .await?;

        let content_type = response.headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("xml") && !url.ends_with(".xml") {
            return Err(format!("Document type is not XML for: {}", url).into());
        }

        let body = response.text().await?;

        Ok(body)
    }
}
