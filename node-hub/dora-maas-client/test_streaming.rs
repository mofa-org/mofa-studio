//! Simple test for streaming functionality
//! Run with: cargo run --example test_streaming

use bytes::Bytes;
use reqwest_eventsource::EventSource;
use serde_json::json;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("Testing EventSource with cloneable request...\n");

    // Test with simple client
    println!("=== Simple Client ===");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create client");
    test_with_client(&client);

    // Test with maas-client style configuration (with no_proxy)
    println!("\n=== MaaS-style Client (no_proxy) ===");
    let client_noproxy = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(30))
        .no_proxy()
        .build()
        .expect("Failed to create client");
    test_with_client(&client_noproxy);

    // Test with proxy enabled
    println!("\n=== MaaS-style Client (with proxy) ===");
    let client_proxy = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create client");
    test_with_client(&client_proxy);

    println!("\nAll tests done!");
}

fn test_with_client(client: &reqwest::Client) {
    // Create a simple request body
    let request_body = json!({
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "Hello"}],
        "stream": true
    });

    // Test with Bytes
    let body_bytes: Bytes = serde_json::to_vec(&request_body).unwrap().into();
    println!("  Body size: {} bytes", body_bytes.len());

    let token = std::env::var("TEST_API_KEY")
        .unwrap_or_else(|_| "test-key".to_string());
    let request = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format\!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(body_bytes);

    match EventSource::new(request) {
        Ok(_) => println!("  ✅ EventSource created successfully"),
        Err(e) => println!("  ❌ Failed: {:?}", e),
    }
}
