# Search indexer: how it works

The Rust search engine lives in `search/` as a Cargo workspace (“postings”
=a Tantivy index; “search” = the served query pipeline). This document
explains the running pieces end to end. Nothing here calls a provider;
imports only read local dump files.

## Crates

| Crate            | Role                                                                   |
| ---------------- | ---------------------------------------------------------------------- |
| `search-model`   | Wire types: `Post`, `SearchRequest`/`SearchResponse`, `Sort`           |
| `search-query`   | Google-style grammar → `Expr` AST; rejects conflicting author filters  |
| `search-ranking` | Engagement + blend math used for sort keys                             |
| `search-backend` | Traits only: `SearchBackend` (search) and `IndexSink` (upsert+commit)  |
| `search-tantivy` | The index. mmap Tantivy store, cursors, five sorts                     |
| `search-ingest`  | Retain-import: archive, quarantine, receipt, idempotent upserts        |
| `search-indexer` | Drop-dir watcher + per-user retry registry (`users.json`)              |
| `search-api`     | Loopback HTTP: bearer `/search`, HMAC `/ticket-search`, signed cursors |
| `xearch-search`  | The binary: `import`, `query`, `serve`, `watch`, `users`               |

## Where postings actually live

Everything sits under one data root (`SEARCH_BASE_DIR`, default
`~/xearch-search`):

```
$BASE/index/    Tantivy mmap store:
                meta.json                  segment list + schema
                <seg>.term                 term dictionary
                <seg>.idx                  posting lists (the inverted index)
                <seg>.pos                  positions (phrase adjacency)
                <seg>.fast                 id/created/likes/engagement columns
                <seg>.fieldnorm            BM25 field lengths
                <seg>.store                post JSON bodies (source of rows)
$BASE/archive/  Content-addressed originals:
                <sha256>.json              exact retained input bytes
                <sha256>.receipt.json      {sha256, accepted, rejected}
                <sha256>.rejected.jsonl    quarantined records + reasons
$BASE/drop/     Intake: one file per account, named <handle>.json[l]
$BASE/state/    users.json — the per-user retry registry
$BASE/logs/     indexer.log
```

Raw input is never mutated; re-importing the same bytes converges to the
same documents (upserts delete-then-add by tweet ID).

## The per-user state file (`state/users.json`)

Every intake account is one record, keyed by normalized handle
(lowercase, `@` stripped, `1–15` ASCII alnum/`_`):

```json
{
  "version": 1,
  "users": {
    "hero": {
      "status": "complete",
      "attempts": 1,
      "accepted": 2,
      "rejected": 0,
      "sha256": "f39540…",
      "fileSig": "f39540…", // sha256 of file bytes (== sha256 for first import)
      "fileName": "hero.json",
      "updatedAtMs": 1789826880797
    }
  }
}
```

Status machine, applied by every pass:

| Situation                                   | Result                                                                   |
| ------------------------------------------- | ------------------------------------------------------------------------ |
| New file seen                               | record starts `incomplete`                                               |
| Import accepts ≥1 post                      | `complete` with receipt facts                                            |
| Import accepts 0 posts (or all quarantined) | `error` — "No posts accepted; N quarantined"                             |
| Import fails (malformed/IO)                 | `error` with reason, attempts+1                                          |
| `error`/`incomplete` user                   | retried on every subsequent pass                                         |
| `complete` user, unchanged bytes            | skipped (content-hash signature)                                         |
| File bytes changed                          | reimported even if `complete`                                            |
| Two files map to one handle                 | the second file is skipped with a warning until the collision is removed |

Safety properties:

- Signatures are **content hashes**, not `size:mtime`, so same-size edits
  with restored clocks cannot be skipped silently.
- `users.json` writes are atomic (temp file + rename + fsync); a crash
  mid-save never leaves a half registry.
- Each imported file is saved with its new registry entry in the same
  pass, so a killed watcher continues where the last file completed.
- Manual `users mark incomplete` clears the signature → next pass
  reimports that file even if unchanged.

## Retry + error operation

```sh
# What needs attention right now
xearch-search --base-dir "$BASE" users list --status error
# Force a re-import of an account
xearch-search --base-dir "$BASE" users mark <handle> incomplete
# Manual completion requires an explanatory note
xearch-search --base-dir "$BASE" users mark <handle> complete --note "verified by hand"
```

## Running it

Local/manual control (no systemd needed), derivable from its own header:

```sh
search-index-ctl.sh start | stop | restart | continue | status | logs | users …
```

`start`/`continue` are idempotent; `restart` waits for the old process to
exit before spawning the replacement (no pidfile race); a stale PID of an
unrelated process is never killed (cmdline identity check). If the systemd
unit `xearch-search-indexer.service` is installed it takes priority; the
script refuses to let a manual watcher run beside the unit.

The systemd unit runs the release binary; build it first:

```sh
cd <repo>/search && cargo build --release -p xearch-search
```

## CLI surface (`xearch-search`)

Precedence is flag > env > derived-from-`--base-dir`. All `SEARCH_*` env
names mirror the flags, so a systemd unit or shell profile can carry the
whole configuration.

| Command                                                | Flags/env                                                 |
| ------------------------------------------------------ | --------------------------------------------------------- |
| `--index`/`SEARCH_INDEX`                               | overrides `"$BASE/index"`                                 |
| `import --input --archive`                             | one-shot retained import                                  |
| `query <q> [--sort …]`                                 | prints version-1 response JSON                            |
| `serve [--listen 127.0.0.1:4320]`                      | needs `SEARCH_LOCAL_SIGNING_KEY` + `SEARCH_SERVICE_TOKEN` |
| `watch [--archive --drop-dir --state-dir --poll-secs]` | background indexer; resolves and logs its dirs at startup |
| `users list [--status …]` / `users mark …`             | registry ops                                              |

## Serving the app contract

`serve` exposes `/health` (open), `/search` (bearer) and `/ticket-search`
(HMAC ticket, 60 s max TTL) on loopback. Continuing cursors are
**HMAC-signed with the server key** before leaving the API, so page depth
and TTL can only be advanced by the server: a tampered cursor is rejected.
`/ticket-search` returns a signed receipt binding session, owner, expiry
and results. Tickets are replayable within their ≤60 s lifetime by design
(attacker value is bounded and each replay burns a search permit); a
single-use nonce requires a ticket issuer (Convex) and is intentionally
left to the integration layer.

## Operator workflow summary

1. Drop `<handle>.json` intake dumps into `$BASE/drop/`.
2. Keep `search-index-ctl.sh continue` (or the systemd unit) running.
3. Watch `users list --status error` and `logs/indexer.log`.
4. Postings grow in `$BASE/index`; the app fetches results from `serve`.
