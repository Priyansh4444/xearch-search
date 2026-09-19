#!/usr/bin/env bash
# Bring the VM to origin/main: fast-forward the pinned checkout, rebuild,
# install units, restart services, reindex, and prove every service healthy.
# No-op when main has not moved unless --force is given.
set -euo pipefail
CODE=${XEARCH_CODE:-/home/exedev/xearch-worker}
DATA=${XEARCH_DATA:-/home/exedev/xearch-data}
UNITS="$HOME/.config/systemd/user"
# One updater at a time: the timer and a manual run must never restart
# services underneath each other.
exec 9>"$DATA/update.lock"
if ! flock -n 9; then
  echo "another update is running; skipping"
  exit 0
fi
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin"
cd "$CODE"
before=$(git rev-parse HEAD)
git fetch -q origin main
git merge -q --ff-only origin/main
after=$(git rev-parse HEAD)
if [ "$before" = "$after" ] && [ "${1:-}" != "--force" ]; then
  echo "up to date at ${after:0:7}"
  exit 0
fi
bun install --frozen-lockfile
(cd search && cargo build --release --locked -p xearch-search)
chmod +x scripts/reindex.sh scripts/vm-update.sh
mkdir -p "$UNITS" "$DATA/hosting/logs" "$DATA/hosting/tmp" "$DATA/search"
install -m 600 deploy/systemd/*.service deploy/systemd/*.timer deploy/systemd/*.path "$UNITS/"
systemctl --user daemon-reload
systemctl --user enable -q xearch-capture xearch-production-worker xearch-search xearch-frontend \
  xearch-reindex.timer xearch-reindex.path xearch-update.timer
systemctl --user restart xearch-capture xearch-search xearch-production-worker xearch-frontend
systemctl --user start xearch-reindex.timer xearch-reindex.path xearch-update.timer
"$CODE/scripts/reindex.sh"
# systemctl restart returns before a service binds; give each one up to 30s.
for port in 4319 4320 4321; do
  for _ in $(seq 1 30); do
    curl -fsS --max-time 2 "http://127.0.0.1:$port/health" >/dev/null 2>&1 && break
    sleep 1
  done
  curl -fsS --max-time 2 "http://127.0.0.1:$port/health" >/dev/null || { echo "health failed on :$port"; exit 1; }
done
echo "updated ${before:0:7} -> ${after:0:7}; capture, search, edge healthy"
