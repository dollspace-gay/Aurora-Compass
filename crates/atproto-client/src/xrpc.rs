//! XRPC client implementation
//!
//! This module implements the XRPC (Cross-Platform Remote Procedure Call) protocol
//! used by AT Protocol services. It provides request/response types, error handling,
//! and the core HTTP client with retry logic.
//!
//! Reference: original-bluesky/src/state/session/agent.ts

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// =============================================================================
// Error Types
// =============================================================================

/// XRPC error with HTTP status and message
///
/// This represents errors returned from XRPC endpoints, including both
/// network failures and application-level errors.
///
/// # Examples
/// ```
/// use atproto_client::xrpc::XrpcError;
///
/// let error = XrpcError::new(404, "NotFound", "Record not found");
/// assert_eq!(error.status(), 404);
/// assert!(!error.is_network_error());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrpcError {
    /// HTTP status code
    status: u16,
    /// Error code (e.g., "InvalidRequest", "NotFound")
    error: String,
    /// Human-readable error message
    message: String,
}

impl XrpcError {
    /// Create a new XRPC error
    pub fn new(status: u16, error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            error: error.into(),
            message: message.into(),
        }
    }

    /// Get the HTTP status code
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Get the error code
    pub fn error(&self) -> &str {
        &self.error
    }

    /// Get the error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Check if this is a network-related error that should be retried
    ///
    /// Based on original-bluesky/src/state/messages/convo/const.ts
    /// Network failure statuses: 1, 408, 425, 429, 500, 502, 503, 504, 522, 524
    pub fn is_network_error(&self) -> bool {
        matches!(
            self.status,
            1 | 408 | 425 | 429 | 500 | 502 | 503 | 504 | 522 | 524
        )
    }

    /// Check if this error is recoverable (can be retried)
    pub fn is_recoverable(&self) -> bool {
        self.is_network_error()
    }
}

impl std::fmt::Display for XrpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "XRPC error {}: {} - {}",
            self.status, self.error, self.message
        )
    }
}

impl std::error::Error for XrpcError {}

// =============================================================================
// Request Types
// =============================================================================

/// HTTP method for XRPC requests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    /// GET request (used for queries)
    Get,
    /// POST request (used for procedures)
    Post,
    /// PUT request
    Put,
    /// DELETE request
    Delete,
}

impl HttpMethod {
    /// Convert to reqwest Method
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
        }
    }
}

/// XRPC request parameters
///
/// Represents a request to an XRPC endpoint with method, path, headers,
/// query parameters, and optional body.
#[derive(Debug, Clone)]
pub struct XrpcRequest {
    /// HTTP method
    pub method: HttpMethod,
    /// NSID path (e.g., "com.atproto.repo.getRecord")
    pub nsid: String,
    /// Query parameters
    pub params: HashMap<String, String>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (for POST/PUT)
    pub body: Option<Vec<u8>>,
    /// Encoding type (e.g., "application/json")
    pub encoding: Option<String>,
}

impl XrpcRequest {
    /// Create a new GET request (query)
    pub fn query(nsid: impl Into<String>) -> Self {
        Self {
            method: HttpMethod::Get,
            nsid: nsid.into(),
            params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            encoding: None,
        }
    }

    /// Create a new POST request (procedure)
    pub fn procedure(nsid: impl Into<String>) -> Self {
        Self {
            method: HttpMethod::Post,
            nsid: nsid.into(),
            params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            encoding: Some("application/json".to_string()),
        }
    }

    /// Add a query parameter
    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Add a header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the request body (for POST/PUT)
    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// Set the request body from JSON
    pub fn json_body<T: Serialize>(mut self, value: &T) -> Result<Self, serde_json::Error> {
        let body = serde_json::to_vec(value)?;
        self.body = Some(body);
        self.encoding = Some("application/json".to_string());
        Ok(self)
    }

    /// Set encoding type
    pub fn encoding(mut self, encoding: impl Into<String>) -> Self {
        self.encoding = Some(encoding.into());
        self
    }
}

// =============================================================================
// Response Types
// =============================================================================

/// XRPC response
///
/// Generic response from an XRPC endpoint with headers and data.
#[derive(Debug, Clone)]
pub struct XrpcResponse<T> {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response data
    pub data: T,
}

impl<T> XrpcResponse<T> {
    /// Create a new response
    pub fn new(status: u16, headers: HashMap<String, String>, data: T) -> Self {
        Self {
            status,
            headers,
            data,
        }
    }

    /// Get a header value
    pub fn header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    /// Check if the response is successful (2xx status)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

// =============================================================================
// Client Configuration
// =============================================================================

/// Configuration for XRPC client
#[derive(Debug, Clone)]
pub struct XrpcClientConfig {
    /// Base service URL (e.g., "https://bsky.social")
    pub service_url: String,
    /// Request timeout
    pub timeout: Duration,
    /// User agent string
    pub user_agent: String,
    /// Custom headers to include in all requests
    pub default_headers: HashMap<String, String>,
}

impl Default for XrpcClientConfig {
    fn default() -> Self {
        Self {
            service_url: "https://bsky.social".to_string(),
            timeout: Duration::from_secs(30),
            user_agent: format!("Aurora-Compass/{}", env!("CARGO_PKG_VERSION")),
            default_headers: HashMap::new(),
        }
    }
}

impl XrpcClientConfig {
    /// Create a new config with a service URL
    pub fn new(service_url: impl Into<String>) -> Self {
        Self {
            service_url: service_url.into(),
            ..Default::default()
        }
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the user agent
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Add a default header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.insert(key.into(), value.into());
        self
    }
}

// =============================================================================
// Error Response Format
// =============================================================================

/// Standard XRPC error response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XrpcErrorResponse {
    /// Error code
    pub error: String,
    /// Error message
    pub message: String,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xrpc_error_network() {
        let error = XrpcError::new(503, "ServiceUnavailable", "Service is down");
        assert_eq!(error.status(), 503);
        assert_eq!(error.error(), "ServiceUnavailable");
        assert_eq!(error.message(), "Service is down");
        assert!(error.is_network_error());
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_xrpc_error_application() {
        let error = XrpcError::new(400, "InvalidRequest", "Bad input");
        assert_eq!(error.status(), 400);
        assert!(!error.is_network_error());
        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_xrpc_request_query() {
        let req = XrpcRequest::query("com.atproto.repo.getRecord")
            .param("repo", "did:plc:123")
            .param("collection", "app.bsky.feed.post")
            .header("Authorization", "Bearer token");

        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.nsid, "com.atproto.repo.getRecord");
        assert_eq!(req.params.get("repo"), Some(&"did:plc:123".to_string()));
        assert_eq!(
            req.headers.get("Authorization"),
            Some(&"Bearer token".to_string())
        );
    }

    #[test]
    fn test_xrpc_request_procedure() {
        let req = XrpcRequest::procedure("com.atproto.repo.createRecord")
            .header("Authorization", "Bearer token");

        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.encoding, Some("application/json".to_string()));
    }

    #[test]
    fn test_xrpc_request_json_body() {
        #[derive(Serialize)]
        struct TestData {
            foo: String,
        }

        let data = TestData {
            foo: "bar".to_string(),
        };

        let req = XrpcRequest::procedure("test.method")
            .json_body(&data)
            .unwrap();

        assert!(req.body.is_some());
        let body_str = String::from_utf8(req.body.unwrap()).unwrap();
        assert!(body_str.contains("bar"));
    }

    #[test]
    fn test_xrpc_response() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        let response = XrpcResponse::new(200, headers, "test data");

        assert_eq!(response.status, 200);
        assert!(response.is_success());
        assert_eq!(
            response.header("content-type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(response.data, "test data");
    }

    #[test]
    fn test_client_config_default() {
        let config = XrpcClientConfig::default();
        assert_eq!(config.service_url, "https://bsky.social");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.user_agent.starts_with("Aurora-Compass/"));
    }

    #[test]
    fn test_client_config_builder() {
        let config = XrpcClientConfig::new("https://custom.server")
            .with_timeout(Duration::from_secs(60))
            .with_user_agent("CustomAgent/1.0")
            .with_header("X-Custom", "value");

        assert_eq!(config.service_url, "https://custom.server");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.user_agent, "CustomAgent/1.0");
        assert_eq!(
            config.default_headers.get("X-Custom"),
            Some(&"value".to_string())
        );
    }

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Put.as_str(), "PUT");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
    }

    #[test]
    fn test_xrpc_error_display() {
        let error = XrpcError::new(404, "NotFound", "Record not found");
        let display = format!("{}", error);
        assert!(display.contains("404"));
        assert!(display.contains("NotFound"));
        assert!(display.contains("Record not found"));
    }
}

// =============================================================================
// Retry Logic with Exponential Backoff
// =============================================================================

use std::future::Future;
use tokio::time::sleep;

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: usize,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier (e.g., 2.0 for exponential backoff)
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new(max_retries: usize) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Set the initial delay
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set the maximum delay
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set the backoff multiplier
    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Calculate the delay for a given retry attempt
    fn calculate_delay(&self, attempt: usize) -> Duration {
        let delay_ms = self.initial_delay.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32);

        let delay = Duration::from_millis(delay_ms as u64);

        // Cap at max_delay
        if delay > self.max_delay {
            self.max_delay
        } else {
            delay
        }
    }
}

/// Retry an async operation with a configurable retry policy
///
/// Reference: original-bluesky/src/lib/async/retry.ts
///
/// # Arguments
/// * `config` - Retry configuration
/// * `should_retry` - Function to determine if an error should be retried
/// * `operation` - The async operation to retry
///
/// # Examples
/// ```
/// use atproto_client::xrpc::{retry, RetryConfig, XrpcError};
///
/// async fn example() -> Result<String, XrpcError> {
///     let config = RetryConfig::new(3);
///
///     retry(
///         config,
///         |err: &XrpcError| err.is_network_error(),
///         || async {
///             // Your operation here
///             Ok("success".to_string())
///         }
///     ).await
/// }
/// ```
pub async fn retry<F, Fut, T, E>(
    config: RetryConfig,
    should_retry: impl Fn(&E) -> bool,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut attempts = 0;
    let mut last_error: Option<E> = None;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                attempts += 1;

                // Check if we should retry this error
                if !should_retry(&err) {
                    return Err(err);
                }

                // Check if we've exhausted retries
                if attempts > config.max_retries {
                    return Err(last_error.unwrap_or(err));
                }

                // Calculate delay and sleep
                let delay = config.calculate_delay(attempts - 1);
                sleep(delay).await;

                last_error = Some(err);
            }
        }
    }
}

/// Convenience function to retry network errors
///
/// Reference: original-bluesky/src/lib/async/retry.ts:networkRetry
///
/// # Examples
/// ```
/// use atproto_client::xrpc::{network_retry, XrpcError};
///
/// async fn example() -> Result<String, XrpcError> {
///     network_retry(2, || async {
///         // Your network operation here
///         Ok("success".to_string())
///     }).await
/// }
/// ```
pub async fn network_retry<F, Fut, T>(max_retries: usize, operation: F) -> Result<T, XrpcError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, XrpcError>>,
{
    let config = RetryConfig::new(max_retries);
    retry(config, |err: &XrpcError| err.is_network_error(), operation).await
}

// =============================================================================
// Retry Tests
// =============================================================================

#[cfg(test)]
mod retry_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_first_attempt() {
        let config = RetryConfig::new(3);
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(
            config,
            |_: &String| true,
            || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, String>("success")
                }
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_retries() {
        let config = RetryConfig::new(3).with_initial_delay(Duration::from_millis(10));
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(
            config,
            |_: &String| true,
            || {
                let c = counter_clone.clone();
                async move {
                    let count = c.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err("temporary error".to_string())
                    } else {
                        Ok("success")
                    }
                }
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_non_retryable_error() {
        let config = RetryConfig::new(3);
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(
            config,
            |err: &String| !err.contains("permanent"),
            || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<String, _>("permanent error".to_string())
                }
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Only tried once
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let config = RetryConfig::new(2).with_initial_delay(Duration::from_millis(10));
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(
            config,
            |_: &String| true,
            || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<String, _>("always fails".to_string())
                }
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Initial + 2 retries
    }

    #[tokio::test]
    async fn test_network_retry_with_network_error() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = network_retry(2, || {
            let c = counter_clone.clone();
            async move {
                let count = c.fetch_add(1, Ordering::SeqCst);
                if count < 1 {
                    Err(XrpcError::new(503, "ServiceUnavailable", "Service down"))
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_network_retry_with_application_error() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result: Result<String, XrpcError> = network_retry(2, || {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err(XrpcError::new(400, "BadRequest", "Invalid input"))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Not retried
    }

    #[test]
    fn test_retry_config_calculate_delay() {
        let config = RetryConfig::new(3)
            .with_initial_delay(Duration::from_millis(100))
            .with_backoff_multiplier(2.0)
            .with_max_delay(Duration::from_secs(5));

        assert_eq!(config.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(config.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(config.calculate_delay(2), Duration::from_millis(400));
        assert_eq!(config.calculate_delay(3), Duration::from_millis(800));
    }

    #[test]
    fn test_retry_config_max_delay() {
        let config = RetryConfig::new(10)
            .with_initial_delay(Duration::from_millis(100))
            .with_backoff_multiplier(2.0)
            .with_max_delay(Duration::from_secs(1));

        // After enough attempts, should cap at max_delay
        assert_eq!(config.calculate_delay(10), Duration::from_secs(1));
    }
}

// =============================================================================
// XRPC Client Implementation
// =============================================================================

use reqwest::{Client as ReqwestClient, Response as ReqwestResponse};

/// XRPC client for making requests to AT Protocol services
///
/// Reference: original-bluesky/src/state/session/agent.ts
///
/// # Examples
/// ```
/// use atproto_client::xrpc::{XrpcClient, XrpcClientConfig, XrpcRequest};
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let config = XrpcClientConfig::new("https://bsky.social");
///     let client = XrpcClient::new(config);
///
///     // Make a query request
///     let request = XrpcRequest::query("com.atproto.server.describeServer");
///     let response = client.query::<serde_json::Value>(request).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct XrpcClient {
    /// HTTP client
    client: ReqwestClient,
    /// Configuration
    config: XrpcClientConfig,
}

impl XrpcClient {
    /// Create a new XRPC client
    pub fn new(config: XrpcClientConfig) -> Self {
        let client = ReqwestClient::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .build()
            .expect("Failed to build HTTP client");

        Self { client, config }
    }

    /// Make a query request (GET)
    ///
    /// # Examples
    /// ```
    /// use atproto_client::xrpc::{XrpcClient, XrpcClientConfig, XrpcRequest};
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct ServerDescription {
    ///     did: String,
    /// }
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = XrpcClientConfig::new("https://bsky.social");
    ///     let client = XrpcClient::new(config);
    ///
    ///     let request = XrpcRequest::query("com.atproto.server.describeServer");
    ///     let response = client.query::<ServerDescription>(request).await?;
    ///
    ///     println!("Server DID: {}", response.data.did);
    ///     Ok(())
    /// }
    /// ```
    pub async fn query<T>(&self, request: XrpcRequest) -> Result<XrpcResponse<T>, XrpcError>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.execute_request(request).await
    }

    /// Make a procedure request (POST)
    ///
    /// # Examples
    /// ```
    /// use atproto_client::xrpc::{XrpcClient, XrpcClientConfig, XrpcRequest};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Serialize)]
    /// struct CreateRecordInput {
    ///     repo: String,
    ///     collection: String,
    ///     record: serde_json::Value,
    /// }
    ///
    /// #[derive(Deserialize)]
    /// struct CreateRecordOutput {
    ///     uri: String,
    ///     cid: String,
    /// }
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = XrpcClientConfig::new("https://bsky.social");
    ///     let client = XrpcClient::new(config);
    ///
    ///     let input = CreateRecordInput {
    ///         repo: "did:plc:123".to_string(),
    ///         collection: "app.bsky.feed.post".to_string(),
    ///         record: serde_json::json!({"text": "Hello!"}),
    ///     };
    ///
    ///     let request = XrpcRequest::procedure("com.atproto.repo.createRecord")
    ///         .json_body(&input)?;
    ///     let response = client.procedure::<CreateRecordOutput>(request).await?;
    ///
    ///     println!("Created: {}", response.data.uri);
    ///     Ok(())
    /// }
    /// ```
    pub async fn procedure<T>(&self, request: XrpcRequest) -> Result<XrpcResponse<T>, XrpcError>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.execute_request(request).await
    }

    /// Make a query request with retry logic
    pub async fn query_with_retry<T>(
        &self,
        request: XrpcRequest,
        max_retries: usize,
    ) -> Result<XrpcResponse<T>, XrpcError>
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        network_retry(max_retries, || self.query(request.clone())).await
    }

    /// Make a procedure request with retry logic
    pub async fn procedure_with_retry<T>(
        &self,
        request: XrpcRequest,
        max_retries: usize,
    ) -> Result<XrpcResponse<T>, XrpcError>
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        network_retry(max_retries, || self.procedure(request.clone())).await
    }

    /// Execute an XRPC request
    async fn execute_request<T>(&self, request: XrpcRequest) -> Result<XrpcResponse<T>, XrpcError>
    where
        T: for<'de> Deserialize<'de>,
    {
        // Build URL
        let url = format!("{}/xrpc/{}", self.config.service_url, request.nsid);

        // Build reqwest request
        let mut req = match request.method {
            HttpMethod::Get => self.client.get(&url),
            HttpMethod::Post => self.client.post(&url),
            HttpMethod::Put => self.client.put(&url),
            HttpMethod::Delete => self.client.delete(&url),
        };

        // Add query parameters
        for (key, value) in &request.params {
            req = req.query(&[(key, value)]);
        }

        // Add default headers
        for (key, value) in &self.config.default_headers {
            req = req.header(key, value);
        }

        // Add request headers
        for (key, value) in &request.headers {
            req = req.header(key, value);
        }

        // Add body if present
        if let Some(body) = &request.body {
            if let Some(encoding) = &request.encoding {
                req = req.header("Content-Type", encoding);
            }
            req = req.body(body.clone());
        }

        // Execute request
        let response = req.send().await.map_err(|e| {
            XrpcError::new(0, "NetworkError", format!("Request failed: {}", e))
        })?;

        // Convert to XrpcResponse
        self.parse_response(response).await
    }

    /// Parse reqwest response into XrpcResponse
    async fn parse_response<T>(&self, response: ReqwestResponse) -> Result<XrpcResponse<T>, XrpcError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let status = response.status().as_u16();

        // Extract headers
        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(key.to_string(), value_str.to_string());
            }
        }

        // Check if response is an error
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_default();

            // Try to parse as XrpcErrorResponse
            if let Ok(error_response) = serde_json::from_str::<XrpcErrorResponse>(&error_body) {
                return Err(XrpcError::new(
                    status,
                    error_response.error,
                    error_response.message,
                ));
            } else {
                return Err(XrpcError::new(
                    status,
                    "Unknown",
                    format!("HTTP {}: {}", status, error_body),
                ));
            }
        }

        // Parse successful response
        let body = response.text().await.map_err(|e| {
            XrpcError::new(0, "ParseError", format!("Failed to read response: {}", e))
        })?;

        let data: T = serde_json::from_str(&body).map_err(|e| {
            XrpcError::new(0, "ParseError", format!("Failed to parse JSON: {}", e))
        })?;

        Ok(XrpcResponse::new(status, headers, data))
    }

    /// Get the client configuration
    pub fn config(&self) -> &XrpcClientConfig {
        &self.config
    }

    /// Get the service URL
    pub fn service_url(&self) -> &str {
        &self.config.service_url
    }
}

// =============================================================================
// Client Tests
// =============================================================================

#[cfg(test)]
mod client_tests {
    use super::*;

    #[test]
    fn test_xrpc_client_new() {
        let config = XrpcClientConfig::new("https://bsky.social")
            .with_timeout(Duration::from_secs(60))
            .with_user_agent("TestAgent/1.0");

        let client = XrpcClient::new(config);
        assert_eq!(client.service_url(), "https://bsky.social");
        assert_eq!(client.config().timeout, Duration::from_secs(60));
        assert_eq!(client.config().user_agent, "TestAgent/1.0");
    }

    #[test]
    fn test_xrpc_client_config_default() {
        let config = XrpcClientConfig::default();
        let client = XrpcClient::new(config);
        assert_eq!(client.service_url(), "https://bsky.social");
    }

    // Note: Integration tests with real network requests would go in tests/ directory
    // These are just basic construction tests
}
