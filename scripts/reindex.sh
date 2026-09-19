#!/usr/bin/env bash
# Import every raw capture that has no receipt yet. Safe to run at any time:
# imports are idempotent by source ID and the running search service reloads
# its reader on the next query, so no restart is needed.
set -euo pipefail
BIN=${XEARCH_SEARCH_BIN:-/home/exedev/xearch-worker/search/target/release/xearch-search}
DATA=${XEARCH_DATA:-/home/exedev/xearch-data}
RAW="$DATA/.local-captures/raw"
INDEX="$DATA/search/index"
ARCHIVE="$DATA/search/archive"
mkdir -p "$INDEX" "$ARCHIVE"
imported=0
for file in "$RAW"/*.json; do
  [ -e "$file" ] || continue
  digest=$(sha256sum "$file" | cut -c1-64)
  [ -f "$ARCHIVE/$digest.receipt.json" ] && continue
  "$BIN" --index "$INDEX" import --input "$file" --archive "$ARCHIVE" >/dev/null
  imported=$((imported + 1))
done
echo "reindex: $imported new capture(s) imported"
