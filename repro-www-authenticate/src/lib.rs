//! Scenarios shared by the `server` and `client` binaries.
//!
//! Each scenario describes how the MCP server answers a request that does not
//! carry the one accepted token (`Authorization: Bearer full`):
//! `unauthenticated` for a request with no token, `insufficient` for a request
//! with any other token. Headers are listed as separate `WWW-Authenticate`
//! fields, exactly as the server sends them.

pub const ACCEPTED_TOKEN: &str = "full";
pub const LOW_SCOPE_TOKEN: &str = "low";

#[derive(Clone, Copy, Debug)]
pub struct Reply {
    pub status: u16,
    /// Separate `WWW-Authenticate` field values, in order. `{base}` is the
    /// server's base URL.
    pub www_authenticate: &'static [&'static str],
    /// JSON-RPC error body, when the server sends one.
    pub json_body: bool,
    /// Keep the body open forever after the first chunk.
    pub unfinished_body: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Scenario {
    pub name: &'static str,
    pub pr: u32,
    pub unauthenticated: Reply,
    pub insufficient: Reply,
}

const BEARER_METADATA: &str = r#"Bearer resource_metadata="{base}/.well-known/oauth-protected-resource", scope="mcp:read""#;

pub const SCENARIOS: &[Scenario] = &[
    // --- #1418: repeated WWW-Authenticate fields -------------------------
    Scenario {
        name: "basic-then-bearer",
        pr: 1418,
        unauthenticated: Reply {
            status: 401,
            www_authenticate: &[r#"Basic realm="proxy""#, BEARER_METADATA],
            json_body: false,
            unfinished_body: false,
        },
        insufficient: Reply {
            status: 403,
            www_authenticate: &[
                r#"Basic realm="proxy""#,
                r#"Bearer error="insufficient_scope", scope="mcp:write""#,
            ],
            json_body: false,
            unfinished_body: false,
        },
    },
    Scenario {
        name: "quoted-text",
        pr: 1418,
        unauthenticated: Reply {
            status: 401,
            www_authenticate: &[
                r#"Bearer error_description="use scope=mcp:read or resource_metadata=/elsewhere", resource_metadata="{base}/.well-known/oauth-protected-resource", scope="mcp:read""#,
            ],
            json_body: false,
            unfinished_body: false,
        },
        insufficient: Reply {
            status: 403,
            www_authenticate: &[
                r#"Bearer error="insufficient_scope", error_description="scope=mcp:read is not enough", scope="mcp:write""#,
            ],
            json_body: false,
            unfinished_body: false,
        },
    },
    // --- #1419: no challenge at all ---------------------------------------
    Scenario {
        name: "no-challenge",
        pr: 1419,
        unauthenticated: Reply {
            status: 401,
            www_authenticate: &[],
            json_body: true,
            unfinished_body: false,
        },
        insufficient: Reply {
            status: 403,
            www_authenticate: &[],
            json_body: true,
            unfinished_body: false,
        },
    },
    Scenario {
        name: "no-challenge-unfinished-body",
        pr: 1419,
        unauthenticated: Reply {
            status: 401,
            www_authenticate: &[],
            json_body: true,
            unfinished_body: true,
        },
        insufficient: Reply {
            status: 403,
            www_authenticate: &[],
            json_body: true,
            unfinished_body: true,
        },
    },
];

pub fn scenario(name: &str) -> Option<&'static Scenario> {
    SCENARIOS.iter().find(|s| s.name == name)
}
