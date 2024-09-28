use crate::errors::UrlError;
use crate::errors::UrlError::InvalidUrl;
use content_disposition::parse_content_disposition;
use regex::Regex;
use reqwest::header::{HeaderMap, ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, RANGE};
use reqwest::{Client, ClientBuilder, Error, Response};
use std::time::Duration;

const FILENAME_EXPRESSION: &str = r#"^[^\/:*?"<>|]+\.[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)?$"#;
const URL_EXPRESSION: &str = r"/(https:\/\/www\.|http:\/\/www\.|https:\/\/|http:\/\/)?[a-zA-Z]{2,}(\.[a-zA-Z]{2,})(\.[a-zA-Z]{2,})?\/[a-zA-Z0-9]{2,}|((https:\/\/www\.|http:\/\/www\.|https:\/\/|http:\/\/)?[a-zA-Z]{2,}(\.[a-zA-Z]{2,})(\.[a-zA-Z]{2,})?)|(https:\/\/www\.|http:\/\/www\.|https:\/\/|http:\/\/)?[a-zA-Z0-9]{2,}\.[a-zA-Z0-9]{2,}\.[a-zA-Z0-9]{2,}(\.[a-zA-Z0-9]{2,})?/g";

pub trait HeaderParse {
    fn content_length(self) -> Option<usize>;
    fn parse_disposition(self) -> Option<String>;
}

impl HeaderParse for HeaderMap {
    fn content_length(self) -> Option<usize> {
        let header = self.get(CONTENT_LENGTH)?;
        header.to_str().ok()?.parse::<usize>().ok()
    }
    fn parse_disposition(self) -> Option<String> {
        let header = self.get(CONTENT_DISPOSITION)?;
        let parsed_header = parse_content_disposition(header.to_str().ok()?);
        parsed_header.filename_full()
    }
}

#[derive(Default, Debug, Clone)]
pub struct Url {
    pub link: String,
    pub filename: Option<String>,
    pub total_size: usize,
    pub range_support: bool,
}

impl Url {
    async fn range_support(
        url: &str,
        total_size: usize,
        client: &Client,
        headers: &HeaderMap,
    ) -> Result<bool, UrlError> {
        if total_size == 0 {
            return Ok(false);
        }
        if let Some(header) = headers.get(ACCEPT_RANGES) {
            if header
                .to_str()
                .map(|v| v.to_lowercase().contains("bytes"))
                .unwrap_or(false)
            {
                return Ok(true);
            }
            return Ok(false);
        }
        let res = client.get(url).header(RANGE, "bytes=0-1").send().await?;
        if res.headers().to_owned().content_length() == Some(1) {
            return Ok(true);
        }
        Ok(false)
    }

    async fn head_req(link: &str, client: &Client) -> Result<Response, Error> {
        client.head(link).send().await
    }
    fn parse_url(link: &str) -> Option<String> {
        let filename = link.split("/").last()?;
        let re = Regex::new(FILENAME_EXPRESSION).expect("Invalid filename regex");
        if re.is_match(filename) {
            return Some(filename.to_string());
        }
        None
    }
    pub async fn from(mut link: &str) -> Result<Self, UrlError> {
        if link.ends_with("/") {
            link = link.trim_end_matches('/');
        }
        let re = Regex::new(URL_EXPRESSION).expect("Invalid url regex");
        if !re.is_match(link) {
            return Err(InvalidUrl);
        }
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(5))
            .build()?;
        let res = Self::head_req(link, &client).await?;
        let headers = res.headers().to_owned();
        let filename: Option<String> = headers
            .clone()
            .parse_disposition()
            .or_else(|| Self::parse_url(link));
        let content_length = headers.clone().content_length();
        let total_size = content_length.unwrap_or_default();
        let range_support = Self::range_support(link, total_size, &client, &headers).await?;
        Ok(Self {
            link: link.to_string(),
            filename,
            total_size,
            range_support,
        })
    }
}
