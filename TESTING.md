# Testing Guide for Aurora Compass

This document outlines the testing strategy, infrastructure, and best practices for the Aurora Compass project.

## Testing Philosophy

Aurora Compass follows a comprehensive testing approach:

1. **Unit Tests**: Test individual functions and types in isolation
2. **Integration Tests**: Test interaction between components using mock services
3. **Property Tests**: Verify invariants hold across many inputs (where applicable)
4. **Documentation Tests**: Ensure code examples in documentation work correctly

All tests should be fast, reliable, and provide clear failure messages.

## Test Organization

### Unit Tests

Unit tests are located in the same file as the code they test, in a `#[cfg(test)]` module at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

**Location**: Throughout `src/` files in each crate

**Run with**: `cargo test --lib`

### Integration Tests

Integration tests are in the `tests/` directory of each crate:

```
crates/
  atproto-client/
    tests/
      xrpc_integration.rs
```

**Location**: `crates/*/tests/*.rs`

**Run with**: `cargo test --test <test_name>`

### Documentation Tests

Documentation tests are embedded in doc comments:

```rust
/// Creates a new DID
///
/// # Examples
/// ```
/// use atproto_client::types::Did;
/// let did = Did::new("did:plc:abc123").unwrap();
/// ```
pub fn new(s: impl Into<String>) -> Result<Self, Error> { ... }
```

**Location**: In `///` doc comments throughout the codebase

**Run with**: `cargo test --doc`

## Test Utilities and Fixtures

The `atproto-client` crate provides comprehensive test utilities in `src/test_utils.rs` (available only in test builds):

### Fixed Test Data

```rust
use crate::test_utils::*;

#[test]
fn test_with_fixtures() {
    let alice = dids::alice();
    let bob_handle = handles::bob();
    let post_uri = uris::alice_post();
    let strong_ref = strong_refs::alice_post();
}
```

### Random Test Data

For tests that need unique values:

```rust
use crate::test_utils::*;

#[test]
fn test_with_random_data() {
    let did = dids::random_plc();
    let handle = handles::random();
    let uri = uris::random_post(&did);
}
```

### Available Fixtures

- **DIDs**: `dids::alice()`, `dids::bob()`, `dids::carol()`, `dids::random_plc()`
- **Handles**: `handles::alice()`, `handles::bob()`, `handles::random()`
- **URIs**: `uris::alice_post()`, `uris::bob_profile()`, `uris::random_post()`
- **TIDs**: `tids::fixed()`, `tids::recent()`, `tids::sequence(n)`
- **CIDs**: `cids::post()`, `cids::image()`, `cids::video()`
- **Strong Refs**: `strong_refs::alice_post()`, `strong_refs::bob_profile()`

### Assertion Helpers

Custom assertions for AT Protocol types:

```rust
use crate::test_utils::assertions::*;

#[test]
fn test_with_assertions() {
    let did = dids::alice();
    assert_did_method(&did, "plc");

    let uri = uris::alice_post();
    assert_uri_collection(&uri, "app.bsky.feed.post");

    let tids = tids::sequence(2);
    assert_tid_ordering(&tids[0], &tids[1]);
}
```

## Mock Server Testing

For XRPC integration tests, we use [wiremock](https://docs.rs/wiremock/) to create mock AT Protocol servers:

```rust
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_xrpc_endpoint() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock response
    Mock::given(method("GET"))
        .and(path("/xrpc/com.atproto.repo.getRecord"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&response_data)
        )
        .mount(&mock_server)
        .await;

    // Create client pointing to mock server
    let config = XrpcClientConfig::new(mock_server.uri());
    let client = XrpcClient::new(config);

    // Make request and verify
    let response = client.query(request).await.unwrap();
    assert_eq!(response.status, 200);
}
```

### Mock Server Best Practices

1. **Use `.mount()` for basic mocks**: Single expectation per test
2. **Use `.up_to_n_times()` for retries**: Test retry behavior
3. **Use `.expect()` for call verification**: Ensure endpoints are called the right number of times
4. **Return proper error formats**: Match AT Protocol error response structure:
   ```json
   {
     "error": "NotFound",
     "message": "Record not found"
   }
   ```

## Running Tests

### All Tests

```bash
cargo test
```

### Specific Crate

```bash
cargo test --package atproto-client
```

### Specific Test

```bash
cargo test test_did_validation
```

### With Output

```bash
cargo test -- --nocapture
```

### Integration Tests Only

```bash
cargo test --test '*'
```

### Fast Tests (skip slow integration tests)

```bash
cargo test --lib
```

## Test Coverage

We aim for high test coverage across all crates:

- **Core types**: 100% coverage (all validation paths)
- **XRPC client**: 90%+ coverage (success, errors, retries)
- **Business logic**: 80%+ coverage (main paths and edge cases)
- **UI components**: Best effort (focus on logic, not rendering)

### Measuring Coverage

Install tarpaulin:

```bash
cargo install cargo-tarpaulin
```

Run coverage:

```bash
cargo tarpaulin --workspace --out Html --output-dir coverage
```

View results:

```bash
# Open coverage/index.html in browser
```

## Writing Good Tests

### Test Naming

Use descriptive names that explain what is being tested:

```rust
#[test]
fn test_did_new_with_valid_plc_method() { }

#[test]
fn test_did_new_rejects_invalid_format() { }

#[test]
fn test_xrpc_retry_succeeds_after_network_error() { }

#[test]
fn test_xrpc_no_retry_on_application_error() { }
```

### Test Structure (Arrange-Act-Assert)

```rust
#[test]
fn test_feature() {
    // Arrange: Set up test data
    let input = "test input";
    let expected = "expected output";

    // Act: Call the function being tested
    let result = function_under_test(input);

    // Assert: Verify the result
    assert_eq!(result, expected);
}
```

### Testing Error Cases

Always test both success and failure paths:

```rust
#[test]
fn test_did_new_success() {
    let result = Did::new("did:plc:abc123");
    assert!(result.is_ok());
}

#[test]
fn test_did_new_invalid_format() {
    let result = Did::new("invalid");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Invalid DID format"));
}
```

### Async Tests

Use `#[tokio::test]` for async tests:

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### Testing Retry Logic

Use mock servers with `.up_to_n_times()`:

```rust
#[tokio::test]
async fn test_retry_behavior() {
    let mock_server = MockServer::start().await;

    // Fail twice, then succeed
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&data))
        .mount(&mock_server)
        .await;

    let result = client.query_with_retry(request, 3).await;
    assert!(result.is_ok());
}
```

## Continuous Integration

Tests run automatically on every push via GitHub Actions (see `.github/workflows/test.yml`).

### CI Test Matrix

- **Platforms**: Windows, macOS, Linux
- **Rust versions**: Stable, Beta, MSRV (Minimum Supported Rust Version)
- **Test types**: Unit, Integration, Doc tests

### Local CI Simulation

Run the same checks as CI locally:

```bash
# Format check
cargo fmt -- --check

# Linting
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --workspace

# Build
cargo build --release
```

## Performance Testing

For performance-critical code, use criterion:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_tid_generation(c: &mut Criterion) {
    c.bench_function("tid::now", |b| {
        b.iter(|| Tid::now())
    });
}

criterion_group!(benches, benchmark_tid_generation);
criterion_main!(benches);
```

Run benchmarks:

```bash
cargo bench
```

## Debugging Test Failures

### Enable logging

```bash
RUST_LOG=debug cargo test -- --nocapture
```

### Run single test

```bash
cargo test test_name -- --exact --nocapture
```

### Use `dbg!()` macro

```rust
#[test]
fn test_debug() {
    let value = compute_value();
    dbg!(&value);  // Prints value with line number
    assert_eq!(value, expected);
}
```

### Print test output

```rust
#[test]
fn test_with_output() {
    println!("Debug info: {:?}", data);
    assert!(condition);
}
```

## Test Dependencies

Key testing dependencies in `Cargo.toml`:

```toml
[dev-dependencies]
tokio = { workspace = true, features = ["test-util", "macros"] }
wiremock = "0.6"
mockall = { workspace = true }
criterion = "0.5"  # For benchmarks
```

## Guidelines

1. **Test early, test often**: Write tests as you develop
2. **Test behavior, not implementation**: Focus on what the code does, not how
3. **Keep tests simple**: Each test should verify one thing
4. **Use descriptive names**: Test names should explain what they verify
5. **Avoid test interdependence**: Tests should run in any order
6. **Mock external services**: Don't depend on real PDS/AppView in tests
7. **Test edge cases**: Empty inputs, boundary values, error conditions
8. **Update tests with code**: Keep tests in sync with implementation changes

## Current Test Status

As of the latest run:

```
Running 84 tests:
- Unit tests: 55 passing
- Integration tests: 17 passing
- Doc tests: 12 passing
- Total: 84 passing, 0 failing
```

Test coverage by crate:

- `atproto-client`: Comprehensive (types, xrpc, errors)
- `app-core`: Basic (branding only)
- Other crates: Placeholder/minimal

## Resources

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Tokio Testing](https://tokio.rs/tokio/topics/testing)
- [Wiremock Documentation](https://docs.rs/wiremock/)
- [Mockall Documentation](https://docs.rs/mockall/)
- [Criterion Benchmarking](https://bheisler.github.io/criterion.rs/book/)

## Questions?

If you have questions about testing or encounter issues with the test infrastructure, please:

1. Check this document first
2. Look at existing tests for examples
3. Consult the original Bluesky client tests in `original-bluesky/`
4. Create a bd issue for test infrastructure improvements
