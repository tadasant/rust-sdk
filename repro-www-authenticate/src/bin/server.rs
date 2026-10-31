//! A minimal MCP server (rmcp's own Streamable HTTP server) behind a token
//! gate that answers unauthenticated or under-scoped requests the way a given
//! scenario prescribes.
//!
//! Usage: `server <scenario> [port]`. Prints the base URL on stdout.

use std::{convert::Infallible, sync::Arc};

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use futures::{StreamExt, stream};
use http::header::{AUTHORIZATION, CONTENT_TYPE, WWW_AUTHENTICATE};
use rmcp::{
    ServerHandler,
    model::{Implementation, ServerCapabilities, ServerConfig},
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use www_authenticate_repro::{ACCEPTED_TOKEN, Reply, Scenario, scenario};

#[derive(Clone)]
struct Repro;

impl ServerHandler for Repro {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::default())
            .with_server_info(Implementation::new("www-authenticate-repro", "0.0.0"))
    }
}

#[derive(Clone)]
struct Gate {
    scenario: &'static Scenario,
    base: Arc<str>,
}

fn reply(reply: &Reply, base: &str) -> Response {
    let mut builder = Response::builder().status(reply.status);
    for value in reply.www_authenticate {
        builder = builder.header(WWW_AUTHENTICATE, value.replace("{base}", base));
    }
    let body = if reply.json_body {
        builder = builder.header(CONTENT_TYPE, "application/json");
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32001,"message":"Unauthorized"}}"#
    } else {
        builder = builder.header(CONTENT_TYPE, "text/plain");
        "denied"
    };
    let body = if reply.unfinished_body {
        Body::from_stream(
            stream::once(async move { Ok::<_, Infallible>(body) }).chain(stream::pending()),
        )
    } else {
        Body::from(body)
    };
    builder.body(body).unwrap()
}

async fn gate(State(gate): State<Gate>, request: Request, next: Next) -> Response {
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    match token {
        Some(ACCEPTED_TOKEN) => next.run(request).await,
        Some(_) => reply(&gate.scenario.insufficient, &gate.base),
        None => reply(&gate.scenario.unauthenticated, &gate.base),
    }
}

async fn protected_resource_metadata(State(gate): State<Gate>) -> Response {
    (
        [(CONTENT_TYPE, "application/json")],
        serde_json::json!({
            "resource": format!("{}/mcp", gate.base),
            "authorization_servers": [format!("{}/as", gate.base)],
            "scopes_supported": ["mcp:read", "mcp:write"],
        })
        .to_string(),
    )
        .into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let name = args.next().unwrap_or_default();
    let Some(scenario) = scenario(&name) else {
        anyhow::bail!(
            "usage: server <scenario> [port]; scenarios: {}",
            www_authenticate_repro::SCENARIOS
                .iter()
                .map(|s| s.name)
                .collect::<Vec<_>>()
                .join(", ")
        );
    };
    let port: u16 = args.next().map(|p| p.parse()).transpose()?.unwrap_or(0);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let base: Arc<str> = format!("http://{}", listener.local_addr()?).into();
    let gate_state = Gate { scenario, base };

    let ct = tokio_util::sync::CancellationToken::new();
    let mcp = StreamableHttpService::new(
        || Ok(Repro),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );
    let router = Router::new()
        .nest_service("/mcp", mcp)
        .layer(middleware::from_fn_with_state(gate_state.clone(), gate))
        .route(
            "/.well-known/oauth-protected-resource",
            get(protected_resource_metadata),
        )
        .with_state(gate_state.clone());

    println!("{}", gate_state.base);
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            ct.cancel();
        })
        .await?;
    Ok(())
}
