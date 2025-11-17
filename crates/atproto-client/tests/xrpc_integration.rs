//! Integration tests for XRPC client
//!
//! These tests use wiremock to create a mock XRPC server and test
//! the full request/response cycle, error handling, and retry behavior.

use atproto_client::xrpc::{XrpcClient, XrpcClientConfig, XrpcError, XrpcRequest, XrpcResponse};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Test data structures
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct TestQuery {
    name: String,
    value: i32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct TestInput {
    text: String,
    count: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct TestOutput {
    uri: String,
    cid: String,
}

// =============================================================================
// Successful Request Tests
// =============================================================================

#[tokio::test]
async fn test_query_request_success() {
    // Setup mock server
    let mock_server = MockServer::start().await;

    let response_data = TestQuery { name: "test".to_string(), value: 42 };

    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.getQuery"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_data))
        .mount(&mock_server)
        .await;

    // Create client
    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    // Make request
    let request = XrpcRequest::query("com.example.getQuery");
    let response: XrpcResponse<TestQuery> = client.query(request).await.unwrap();

    // Verify
    assert_eq!(response.status, 200);
    assert_eq!(response.data, response_data);
}

#[tokio::test]
async fn test_query_request_with_params() {
    let mock_server = MockServer::start().await;

    let response_data = TestQuery { name: "filtered".to_string(), value: 100 };

    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.getQuery"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_data))
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::query("com.example.getQuery")
        .param("filter", "active")
        .param("limit", "10");

    let response: XrpcResponse<TestQuery> = client.query(request).await.unwrap();

    assert_eq!(response.data.name, "filtered");
    assert_eq!(response.data.value, 100);
}

#[tokio::test]
async fn test_procedure_request_success() {
    let mock_server = MockServer::start().await;

    let input = TestInput { text: "Hello, world!".to_string(), count: 5 };

    let output = TestOutput {
        uri: "at://did:plc:123/app.bsky.feed.post/abc".to_string(),
        cid: "bafyreigq4zsipbk5w3uqkbmh2w2633c4tcwudryvoqkfrq3mqfs3d5e3wq".to_string(),
    };

    Mock::given(method("POST"))
        .and(path("/xrpc/com.example.createRecord"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&output))
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::procedure("com.example.createRecord")
        .json_body(&input)
        .unwrap();

    let response: XrpcResponse<TestOutput> = client.procedure(request).await.unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.data, output);
}

#[tokio::test]
async fn test_request_with_custom_headers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.getQuery"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&TestQuery { name: "test".to_string(), value: 1 }),
        )
        .mount(&mock_server)
        .await;

    let config =
        XrpcClientConfig::new(mock_server.uri()).with_header("X-Custom-Header", "custom-value");

    let client = XrpcClient::new(config);

    let request =
        XrpcRequest::query("com.example.getQuery").header("Authorization", "Bearer token123");

    let response: XrpcResponse<TestQuery> = client.query(request).await.unwrap();

    assert_eq!(response.status, 200);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_404_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.notFound"))
        .respond_with(ResponseTemplate::new(404).set_body_json(&serde_json::json!({
            "error": "NotFound",
            "message": "Record not found"
        })))
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::query("com.example.notFound");
    let result: Result<XrpcResponse<TestQuery>, XrpcError> = client.query(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.status(), 404);
    assert_eq!(error.error(), "NotFound");
    assert_eq!(error.message(), "Record not found");
}

#[tokio::test]
async fn test_400_bad_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/xrpc/com.example.invalid"))
        .respond_with(ResponseTemplate::new(400).set_body_json(&serde_json::json!({
            "error": "InvalidRequest",
            "message": "Missing required field"
        })))
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::procedure("com.example.invalid");
    let result: Result<XrpcResponse<TestOutput>, XrpcError> = client.procedure(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.status(), 400);
    assert!(!error.is_network_error()); // Application error, not network error
}

#[tokio::test]
async fn test_500_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.serverError"))
        .respond_with(ResponseTemplate::new(500).set_body_json(&serde_json::json!({
            "error": "InternalServerError",
            "message": "Something went wrong"
        })))
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::query("com.example.serverError");
    let result: Result<XrpcResponse<TestQuery>, XrpcError> = client.query(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.status(), 500);
    assert!(error.is_network_error()); // 500 is a network error
    assert!(error.is_recoverable()); // Can be retried
}

#[tokio::test]
async fn test_503_service_unavailable() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.unavailable"))
        .respond_with(ResponseTemplate::new(503).set_body_json(&serde_json::json!({
            "error": "ServiceUnavailable",
            "message": "Service temporarily unavailable"
        })))
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::query("com.example.unavailable");
    let result: Result<XrpcResponse<TestQuery>, XrpcError> = client.query(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.status(), 503);
    assert!(error.is_network_error());
    assert!(error.is_recoverable());
}

// =============================================================================
// Retry Behavior Tests
// =============================================================================

#[tokio::test]
async fn test_retry_on_network_error_success() {
    let mock_server = MockServer::start().await;

    let success_data = TestQuery { name: "success".to_string(), value: 123 };

    // First request fails with 503
    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.retry"))
        .respond_with(ResponseTemplate::new(503).set_body_json(&serde_json::json!({
            "error": "ServiceUnavailable",
            "message": "Temporarily unavailable"
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Subsequent requests succeed
    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.retry"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&success_data))
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::query("com.example.retry");
    let response: XrpcResponse<TestQuery> = client.query_with_retry(request, 2).await.unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.data, success_data);
}

#[tokio::test]
async fn test_retry_exhausted() {
    let mock_server = MockServer::start().await;

    // All requests fail with 503
    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.alwaysFails"))
        .respond_with(ResponseTemplate::new(503).set_body_json(&serde_json::json!({
            "error": "ServiceUnavailable",
            "message": "Always fails"
        })))
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::query("com.example.alwaysFails");
    let result: Result<XrpcResponse<TestQuery>, XrpcError> =
        client.query_with_retry(request, 2).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.status(), 503);
}

#[tokio::test]
async fn test_no_retry_on_application_error() {
    let mock_server = MockServer::start().await;

    // Fail with 400 (application error - should not retry)
    Mock::given(method("POST"))
        .and(path("/xrpc/com.example.badRequest"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(&serde_json::json!({
                "error": "InvalidRequest",
                "message": "Bad input"
            })),
        )
        .expect(1) // Should only be called once (no retry)
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::procedure("com.example.badRequest");
    let result: Result<XrpcResponse<TestOutput>, XrpcError> =
        client.procedure_with_retry(request, 3).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.status(), 400);
    assert!(!error.is_recoverable());
}

// =============================================================================
// Header and Response Tests
// =============================================================================

#[tokio::test]
async fn test_response_headers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.withHeaders"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&TestQuery { name: "test".to_string(), value: 1 })
                .insert_header("X-Custom-Header", "custom-value")
                .insert_header("X-Request-Id", "req-123"),
        )
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::query("com.example.withHeaders");
    let response: XrpcResponse<TestQuery> = client.query(request).await.unwrap();

    assert!(response.headers.contains_key("x-custom-header"));
    assert!(response.headers.contains_key("x-request-id"));
}

// =============================================================================
// Configuration Tests
// =============================================================================

#[tokio::test]
async fn test_custom_timeout() {
    let config = XrpcClientConfig::new("https://example.com").with_timeout(Duration::from_secs(5));

    let client = XrpcClient::new(config);
    assert_eq!(client.config().timeout, Duration::from_secs(5));
}

#[tokio::test]
async fn test_custom_user_agent() {
    let config = XrpcClientConfig::new("https://example.com").with_user_agent("TestClient/1.0");

    let client = XrpcClient::new(config);
    assert_eq!(client.config().user_agent, "TestClient/1.0");
}

#[tokio::test]
async fn test_default_headers() {
    let config = XrpcClientConfig::new("https://example.com")
        .with_header("X-API-Key", "secret123")
        .with_header("X-Client-Version", "1.0");

    let client = XrpcClient::new(config);
    assert_eq!(client.config().default_headers.get("X-API-Key"), Some(&"secret123".to_string()));
    assert_eq!(
        client.config().default_headers.get("X-Client-Version"),
        Some(&"1.0".to_string())
    );
}

// =============================================================================
// Edge Cases
// =============================================================================

#[tokio::test]
async fn test_empty_response_body() {
    let mock_server = MockServer::start().await;

    // Return 200 with empty JSON object
    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.empty"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&serde_json::json!({})))
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::query("com.example.empty");
    let response: XrpcResponse<serde_json::Value> = client.query(request).await.unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.data, serde_json::json!({}));
}

#[tokio::test]
async fn test_malformed_json_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/xrpc/com.example.malformed"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
        .mount(&mock_server)
        .await;

    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    let request = XrpcRequest::query("com.example.malformed");
    let result: Result<XrpcResponse<TestQuery>, XrpcError> = client.query(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.error(), "ParseError");
    assert!(error.message().contains("Failed to parse JSON"));
}
