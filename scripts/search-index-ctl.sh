#!/usr/bin/env bash
# xearch-search control: start, stop, restart, continue, status, logs, users.
#
# Thin process wrapper around the Rust binary — all indexing logic lives in
# the search-indexer crate. Prefers the systemd user unit when installed,
# otherwise runs the binary with nohup + pidfile so any machine works.
#
# Config precedence (adopted from opencode): flag > env > default.
#   SEARCH_BIN       default: <repo>/search/target/debug/xearch-search
#   SEARCH_BASE_DIR  default: <repo>/.local-search
#   (plus SEARCH_INDEX / SEARCH_ARCHIVE_DIR / SEARCH_DROP_DIR /
#    SEARCH_STATE_DIR / SEARCH_POLL_SECS for individual overrides)
#
# Postings layout under $SEARCH_BASE_DIR:
#   index/    Tantivy mmap store: meta.json + per-segment
#             .term/.idx/.pos (terms + posting lists + positions),
#             .fast (id/created/likes/engagement), .store (post JSON)
#   archive/  content-addressed raw dumps <sha>.json + receipts
#   drop/     per-user intake files <handle>.json (watched)
#   state/    users.json registry (complete/incomplete/error)
#   logs/     indexer.log
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${SEARCH_BIN:-$REPO/search/target/debug/xearch-search}"
BASE="${SEARCH_BASE_DIR:-$REPO/.local-search}"
PIDFILE="$BASE/indexer.pid"
UNIT="xearch-search-indexer.service"

build() {
  if [[ ! -x "$BIN" ]] || ! "$BIN" --help 2>/dev/null | grep -q "base-dir"; then
    echo "building xearch-search..." >&2
    (cd "$REPO/search" && cargo build -q -p xearch-search)
  fi
}

unit_installed() {
  systemctl --user list-unit-files "$UNIT" >/dev/null 2>&1
}

running_pid() {
  if [[ -f "$PIDFILE" ]]; then
    local pid
    pid="$(cat "$PIDFILE")"
    if kill -0 "$pid" 2>/dev/null; then echo "$pid"; return 0; fi
  fi
  return 1
}

do_start() {
  mkdir -p "$BASE"/{index,archive,drop,state,logs}
  if unit_installed; then
    systemctl --user start "$UNIT"
    echo "started via systemd ($UNIT)"
    return
  fi
  build
  if running_pid >/dev/null; then
    echo "already running (pid $(cat "$PIDFILE"))"
    return
  fi
  nohup "$BIN" --base-dir "$BASE" watch >>"$BASE/logs/indexer.log" 2>&1 &
  echo "$!" >"$PIDFILE"
  echo "started (pid $!)"
}

do_stop() {
  if unit_installed && systemctl --user is-active -q "$UNIT"; then
    systemctl --user stop "$UNIT"
    echo "stopped systemd unit"
    return
  fi
  local pid
  if pid="$(running_pid)"; then
    kill "$pid"
    rm -f "$PIDFILE"
    echo "stopped (pid $pid)"
  else
    rm -f "$PIDFILE"
    echo "not running"
  fi
}

do_status() {
  echo "bin:  $BIN"
  echo "base: $BASE"
  if unit_installed; then systemctl --user is-active "$UNIT" || true; fi
  if pid="$(running_pid)"; then echo "pid:  $pid (running)"; else echo "pid:  not running"; fi
  echo "drop files:  $(find "$BASE/drop" -maxdepth 1 -name '*.json*' 2>/dev/null | wc -l)"
  echo "index size:  $(du -sh "$BASE/index" 2>/dev/null | cut -f1)"
  if [[ -x "$BIN" ]]; then
    "$BIN" --base-dir "$BASE" users list 2>/dev/null | python3 -c "
import json,sys
try: users=json.load(sys.stdin)
except Exception: users={}
from collections import Counter
c=Counter(r['status'] for r in users.values())
print('users:', dict(c) or 'none yet', 'total:', len(users))
" || echo "users: registry unreadable"
  fi
}

case "${1:-status}" in
  start) do_start ;;
  stop) do_stop ;;
  restart) do_stop; sleep 1; do_start ;;
  continue) do_start ;; # keep running while things move; no-op if up
  status) do_status ;;
  logs) exec tail -f "$BASE/logs/indexer.log" ;;
  users) shift; exec "$BIN" --base-dir "$BASE" users "$@" ;;
  *) echo "usage: $0 {start|stop|restart|continue|status|logs|users ...}" >&2; exit 1 ;;
esac
