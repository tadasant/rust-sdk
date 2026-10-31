#!/usr/bin/env sh
# Runs every scenario: starts the server, probes it with the client, stops it.
# Build against whichever `../crates/rmcp` is checked out to compare revisions.
set -eu
cd "$(dirname "$0")"
cargo build --quiet --bins
bin="${CARGO_TARGET_DIR:-target}/debug"
for scenario in $("$bin/server" 2>&1 | sed -n 's/.*scenarios: //p' | tr -d ','); do
  echo "== scenario: $scenario"
  "$bin/server" "$scenario" > server.url &
  pid=$!
  for _ in 1 2 3 4 5 6 7 8 9 10; do [ -s server.url ] && break; sleep 0.2; done
  "$bin/client" "$(cat server.url)"
  kill "$pid"; wait "$pid" 2>/dev/null || true
  rm -f server.url
done
