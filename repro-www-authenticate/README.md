# WWW-Authenticate reproduction for rust-sdk #1418 and #1419

A minimal MCP server (rmcp's Streamable HTTP server behind a token gate) and a
client probe that prints what the rmcp client hands back. The crate is not a
workspace member and depends on `../crates/rmcp` by path, so the same binaries
show the behaviour of whichever SDK revision is checked out next to it.

```sh
./run.sh                       # all scenarios, server started per scenario
cargo run --bin server -- basic-then-bearer 8787   # or run one by hand
cargo run --bin client -- http://127.0.0.1:8787
```

Tokens: `Authorization: Bearer full` reaches the MCP server; `Bearer low` gets
the scenario's 403; no token gets its 401.

| Scenario | PR | Unauthenticated (401) | Under-scoped (403) |
| --- | --- | --- | --- |
| `basic-then-bearer` | #1418 | two `WWW-Authenticate` fields: `Basic realm="proxy"`, then `Bearer resource_metadata=…, scope="mcp:read"` | two fields: `Basic realm="proxy"`, then `Bearer error="insufficient_scope", scope="mcp:write"` |
| `quoted-text` | #1418 | one `Bearer` field whose `error_description` quotes `scope=…` and `resource_metadata=…` text | one `Bearer` field whose `error_description` quotes `scope=mcp:read` |
| `no-challenge` | #1419 | 401, no `WWW-Authenticate`, JSON-RPC error body | 403, same |
| `no-challenge-unfinished-body` | #1419 | as above, body never finishes | as above |
