//! AGFS HTTP client

// TODO: Remove this allow once client implementation is complete
#![allow(dead_code)]

use reqwest::Client as HttpClient;
use std::sync::Arc;

/// AGFS client for connecting to AGFS server
pub struct Client {
    base_url: String,
    http_client: Arc<HttpClient>,
}

impl Client {
    /// Create a new AGFS client
    pub fn new(base_url: impl Into<String>) -> Result<Self, reqwest::Error> {
        let base_url = base_url.into();
        let http_client = HttpClient::builder().build()?;
        Ok(Self {
            base_url,
            http_client: Arc::new(http_client),
        })
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}
