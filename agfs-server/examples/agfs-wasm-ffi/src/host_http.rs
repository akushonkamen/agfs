//! Host HTTP access from WASM
//!
//! This module provides HTTP request capabilities exposed by agfs-server.
//! WASM plugins can use this to make HTTP requests to external services.

use crate::types::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::CString;

// Simple base64 decoding (standard alphabet)
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    const BASE64_TABLE: &[u8; 128] = &[
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 62, 255, 255, 255, 63,
        52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 255, 255, 255, 0, 255, 255,
        255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
        15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 255, 255, 255, 255, 255,
        255, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
        41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 255, 255, 255, 255, 255,
    ];

    if input.is_empty() {
        return Ok(Vec::new());
    }

    let input = input.trim();
    let mut output = Vec::with_capacity((input.len() * 3) / 4);
    let mut buf = 0u32;
    let mut bits = 0;

    for &b in input.as_bytes() {
        if b == b'=' {
            break;
        }
        if b >= 128 {
            return Err(Error::Other("invalid base64 character".to_string()));
        }
        let val = BASE64_TABLE[b as usize];
        if val == 255 {
            continue; // Skip whitespace/invalid chars
        }

        buf = (buf << 6) | (val as u32);
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            output.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    Ok(output)
}

// Import host function from the "env" module
#[link(wasm_import_module = "env")]
extern "C" {
    fn host_http_request(request_ptr: *const u8) -> u64;
}

/// HTTP request to be sent by the host
#[derive(Debug, Serialize, Deserialize)]
pub struct HttpRequest {
    #[serde(default = "default_method")]
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Vec<u8>,
    #[serde(default = "default_timeout")]
    pub timeout: i32, // timeout in seconds
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_timeout() -> i32 {
    30
}

impl HttpRequest {
    /// Create a new HTTP GET request
    pub fn get(url: &str) -> Self {
        Self {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            timeout: 30,
        }
    }

    /// Create a new HTTP POST request
    pub fn post(url: &str) -> Self {
        Self {
            method: "POST".to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            timeout: 30,
        }
    }

    /// Create a new HTTP PUT request
    pub fn put(url: &str) -> Self {
        Self {
            method: "PUT".to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            timeout: 30,
        }
    }

    /// Create a new HTTP DELETE request
    pub fn delete(url: &str) -> Self {
        Self {
            method: "DELETE".to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            timeout: 30,
        }
    }

    /// Set request method
    pub fn method(mut self, method: &str) -> Self {
        self.method = method.to_string();
        self
    }

    /// Add a header
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Set request body
    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Set request body from string
    pub fn body_str(mut self, body: &str) -> Self {
        self.body = body.as_bytes().to_vec();
        self
    }

    /// Set request body as JSON
    pub fn json<T: Serialize>(mut self, data: &T) -> Result<Self> {
        let json_str = serde_json::to_string(data)
            .map_err(|e| Error::Other(format!("failed to serialize JSON: {}", e)))?;
        self.body = json_str.as_bytes().to_vec();
        self.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        Ok(self)
    }

    /// Set timeout in seconds
    pub fn timeout(mut self, seconds: i32) -> Self {
        self.timeout = seconds;
        self
    }
}

/// HTTP response from the host (internal, for JSON deserialization)
#[derive(Debug, Deserialize)]
struct HttpResponseRaw {
    status_code: i32,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: String, // Go encodes []byte as base64 string
    #[serde(default)]
    error: String,
}

/// HTTP response from the host
#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: i32,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub error: String,
}

impl HttpResponse {
    /// Get response body as string
    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.body.clone())
            .map_err(|e| Error::Other(format!("invalid UTF-8 in response body: {}", e)))
    }

    /// Parse response body as JSON
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        serde_json::from_slice(&self.body)
            .map_err(|e| Error::Other(format!("failed to parse JSON response: {}", e)))
    }

    /// Check if request was successful (status code 2xx)
    pub fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }

    /// Get error message if request failed
    pub fn error(&self) -> Option<&str> {
        if !self.error.is_empty() {
            Some(&self.error)
        } else {
            None
        }
    }
}

/// Http provides HTTP request capabilities from WASM
pub struct Http;

impl Http {
    /// Perform an HTTP request
    pub fn request(req: HttpRequest) -> Result<HttpResponse> {
        // Serialize request to JSON
        let request_json = serde_json::to_string(&req)
            .map_err(|e| Error::Other(format!("failed to serialize request: {}", e)))?;

        let request_c = CString::new(request_json)
            .map_err(|_| Error::InvalidInput("invalid request JSON".to_string()))?;

        unsafe {
            let result = host_http_request(request_c.as_ptr() as *const u8);

            // Unpack: lower 32 bits = pointer, upper 32 bits = size
            let response_ptr = (result & 0xFFFFFFFF) as u32;
            let response_size = ((result >> 32) & 0xFFFFFFFF) as u32;

            if response_ptr == 0 {
                return Err(Error::Other("HTTP request failed".to_string()));
            }

            // Read response from memory
            let slice = std::slice::from_raw_parts(response_ptr as *const u8, response_size as usize);
            let response_json = String::from_utf8_lossy(slice);

            // Parse response (raw format with base64 body)
            let response_raw: HttpResponseRaw = serde_json::from_str(&response_json)
                .map_err(|e| Error::Other(format!("failed to parse response: {}", e)))?;

            // Decode base64 body
            let body = base64_decode(&response_raw.body)?;

            // Build final response
            let response = HttpResponse {
                status_code: response_raw.status_code,
                headers: response_raw.headers,
                body,
                error: response_raw.error.clone(),
            };

            // Check for error in response
            if !response.error.is_empty() {
                return Err(Error::Other(response.error.clone()));
            }

            Ok(response)
        }
    }

    /// Perform a GET request
    pub fn get(url: &str) -> Result<HttpResponse> {
        Self::request(HttpRequest::get(url))
    }

    /// Perform a POST request with body
    pub fn post(url: &str, body: Vec<u8>) -> Result<HttpResponse> {
        Self::request(HttpRequest::post(url).body(body))
    }

    /// Perform a POST request with JSON body
    pub fn post_json<T: Serialize>(url: &str, data: &T) -> Result<HttpResponse> {
        Self::request(HttpRequest::post(url).json(data)?)
    }

    /// Perform a PUT request with body
    pub fn put(url: &str, body: Vec<u8>) -> Result<HttpResponse> {
        Self::request(HttpRequest::put(url).body(body))
    }

    /// Perform a DELETE request
    pub fn delete(url: &str) -> Result<HttpResponse> {
        Self::request(HttpRequest::delete(url))
    }
}
