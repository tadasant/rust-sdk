//! Probes a running `server` with the rmcp client and prints what the SDK
//! hands back: the lifecycle error for an unauthenticated `initialize`, and
//! the raw transport error for a POST and a GET with no token and with an
//! under-scoped token.
//!
//! Usage: `client <base-url>`.

use std::{collections::HashMap, sync::Arc, time::Duration};

use rmcp::{
    ServiceExt,
    model::{ClientConfig, ClientJsonRpcMessage, ClientRequest, PingRequest, RequestId},
    transport::{
        StreamableHttpClientTransport,
        auth::WWWAuthenticateParams,
        streamable_http_client::{
            StreamableHttpClient, StreamableHttpClientTransportConfig, StreamableHttpError,
        },
    },
};
use www_authenticate_repro::LOW_SCOPE_TOKEN;

const TIMEOUT: Duration = Duration::from_secs(3);

fn describe(error: &StreamableHttpError<reqwest::Error>) -> String {
    match error {
        StreamableHttpError::AuthRequired(e) => {
            format!("AuthRequired {{ www_authenticate_header: {:?} }}", e.www_authenticate_header)
        }
        StreamableHttpError::InsufficientScope(e) => format!(
            "InsufficientScope {{ required_scope: {:?}, www_authenticate_header: {:?} }}",
            e.required_scope, e.www_authenticate_header
        ),
        StreamableHttpError::HttpStatus(e) => {
            format!("HttpStatus {{ status: {}, retry_after: {:?} }}", e.status, e.retry_after)
        }
        StreamableHttpError::UnexpectedServerResponse(message) => {
            format!("UnexpectedServerResponse({message:?})")
        }
        StreamableHttpError::Client(e) => format!("Client(reqwest: {e})"),
        other => format!("{other:?}"),
    }
}

async fn probe_transport(uri: &Arc<str>, token: Option<&str>) {
    let client = reqwest::Client::new();
    let ping = ClientJsonRpcMessage::request(
        ClientRequest::PingRequest(PingRequest::default()),
        RequestId::Number(1),
    );
    let label = token.map_or("no token".to_owned(), |t| format!("token {t:?}"));
    let post = tokio::time::timeout(
        TIMEOUT,
        client.post_message(uri.clone(), ping, None, token.map(str::to_owned), HashMap::new()),
    )
    .await;
    match post {
        Err(_) => println!("  POST ({label}): still reading the body after {TIMEOUT:?}"),
        Ok(Ok(_)) => println!("  POST ({label}): Ok"),
        Ok(Err(e)) => println!("  POST ({label}): {}", describe(&e)),
    }
    let get = tokio::time::timeout(
        TIMEOUT,
        client.get_stream(uri.clone(), None, None, token.map(str::to_owned), HashMap::new()),
    )
    .await;
    match get {
        Err(_) => println!("  GET  ({label}): still reading the body after {TIMEOUT:?}"),
        Ok(Ok(_)) => println!("  GET  ({label}): Ok"),
        Ok(Err(e)) => println!("  GET  ({label}): {}", describe(&e)),
    }
}

async fn probe_initialize(uri: &Arc<str>) {
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(uri.clone()),
    );
    let result = tokio::time::timeout(TIMEOUT, ClientConfig::default().serve(transport)).await;
    match result {
        Err(_) => println!("  initialize (no token): still waiting after {TIMEOUT:?}"),
        Ok(Ok(_)) => println!("  initialize (no token): connected (unexpected)"),
        Ok(Err(error)) => {
            println!("  initialize (no token): Err({error})");
            println!(
                "    is_authorization_required = {}",
                error.is_authorization_required()
            );
            println!("    auth_challenge           = {:?}", error.auth_challenge());
            if let Some(challenge) = error.auth_challenge() {
                let base = url::Url::parse(uri).unwrap();
                let params = WWWAuthenticateParams::parse(challenge, &base);
                println!(
                    "    WWWAuthenticateParams    = resource_metadata_url={:?} scope={:?} error={:?}",
                    params.resource_metadata_url.as_ref().map(|u| u.as_str()),
                    params.scope,
                    params.error
                );
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: client <base-url>"))?;
    let uri: Arc<str> = format!("{}/mcp", base.trim_end_matches('/')).into();
    println!("rmcp {} against {uri}", env!("CARGO_PKG_VERSION"));
    probe_initialize(&uri).await;
    probe_transport(&uri, None).await;
    probe_transport(&uri, Some(LOW_SCOPE_TOKEN)).await;
    Ok(())
}
