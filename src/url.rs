use std::path::Path;
use std::time::Duration;
use content_disposition::parse_content_disposition;
use random_string::generate;
use regex::Regex;
use reqwest::{ClientBuilder, Error, Response};
use reqwest::header::{HeaderMap, CONTENT_DISPOSITION, CONTENT_LENGTH};
use crate::errors::UrlError;
use crate::errors::UrlError::InvalidUrl;

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


#[derive(Default, Debug)]
pub struct Url {
    pub url: String,
    pub filename: Option<String>,
    pub total_size: usize,
}

impl Url {
    async fn head_req(link: &str) -> Result<Response, Error> {
        let client = ClientBuilder::new().timeout(Duration::from_secs(5)).build()?;
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
        let res = Self::head_req(link).await?;
        let headers = res.headers().to_owned();
        let filename: Option<String> = headers.clone().parse_disposition().or_else(|| Self::parse_url(link));
        let content_length = headers.content_length();
        Ok(Self {
            url: link.to_string(),
            filename,
            total_size: content_length.unwrap_or_default(),
        })
    }
}