# Xearch

Rewrite coordination: this is now the shared app repository. Prronsh owns the
local indexer and Elasticsearch work; the application keeps Convex orchestration
and provider integrations. See [rewrite foundations](docs/rewrite-foundations.md)
for the imported lint rules, Effect validation, and integration boundaries.

## Local import dashboard

Open `http://localhost:5173/?dashboard=1` for the live import controls. Start/stop/retry jobs, continue older pages, and inspect raw-capture receipts. All seven x.md collection tasks are available. Jobs are private to the current guest session; other tabs in that session update through Convex subscriptions.

For the temporary local receiver, run `bun run capture:setup` once, then keep these three processes running in separate terminals:

```sh
bun run backend
bun run capture
bun run dev
```

The setup script is restricted to this anonymous local Convex deployment. It generates a private token and configures the loopback receiver automatically. Captures are saved unchanged under `.local-captures/raw/<sha256>.json`; the token is `.local-captures/token`. Both are ignored by Git. The receiver is loopback-only, checks authorization and checksums, and syncs files before acknowledging them. It is a temporary raw-file sink, not the normalization or search backend, and has no automatic deletion or disk quota. Monitor disk use. It is not suitable for a hosted Convex deployment without replacing the receiver with an authenticated reachable service.

`Downloaded` means saved locally, not searchable. New history imports automatically fetch older 500-post batches under one job. They pause at the daily budget or 20 batches, and stop if the date boundary fails to move backwards. Counts show received posts and may include repeated posts at inclusive page boundaries. Stop prevents later work and acknowledgments; an already-running upstream request may still finish and already-written files remain. Technical details show up to 100 receipts per job. Older completed jobs with more history offer a continuation button; they are not silently restarted.

Put backend keys and `OPENAI_MODEL` in `.env.local`, then run `bun run env:sync`. The script only syncs allowlisted nonempty variables to the local anonymous deployment and never prints their values. AgentMail webhooks need a public deployment URL; leave the webhook secret unset during local work unless a public callback has separately been configured.

The search backend is still separate. Import controls work without `SEARCH_API_URL`. Replace `RAW_CAPTURE_URL` and `RAW_CAPTURE_TOKEN` with the collaborator's receiver when ready; no collector rewrite is needed.

Search X posts, import account histories through x.md, and read the pages behind the links. The interaction follows [search.pronsh.dev](https://search.pronsh.dev/). Convex owns the app and indexing orchestration; a separately owned data service handles storage, normalization, and retrieval.

- [Product and tool responsibilities](docs/product.md)
- [Contract for the data-service owner](docs/integration-contract.md)
- [Hackathon build evidence](hackathon.md)

The previous [Xearch project](https://github.com/Priyansh4444/xearch) was used as an architecture reference. Its checkout is not included in this repository.

## Hackathon build log

The official Convex hackathon skill is included at `.agents/skills/hackathon`, with its log-format reference, agent configuration, and upstream license. Invoke `/hackathon` to update `hackathon.md` from repository evidence. Upstream: [get-convex/convex-hackathon-skill](https://github.com/get-convex/convex-hackathon-skill), revision `5306ddc9d0cbe8b659dd7d0d7b488be399bf55bf`.

## Run locally

```sh
bun install --frozen-lockfile
CONVEX_AGENT_MODE=anonymous bun run backend
```

On the first run the Firecrawl component requires its environment variable to exist. Without a key yet, set an empty value so other features can run; Firecrawl operations remain disabled:

```sh
bunx convex env set FIRECRAWL_API_KEY ''
node scripts/setup-auth.mjs --local
bun run dev
```

Run the auth-key script once per new local deployment. It generates backend signing keys without printing their values. Re-running rotates them and signs out existing sessions. The frontend is http://localhost:5173. `.env.local` and `.convex/` are ignored. Use Connections in the app to inspect which integrations are configured.

## Connect providers

Use `bunx convex env set NAME` and supply the value through stdin/the prompt. Do not use `VITE_` variables for secrets.

| Variable | Purpose |
| --- | --- |
| `X_MD_API_KEY` | x.md acquisition credential |
| `X_MD_BASE_URL` | Optional alternate official origin, `https://x.pcstyle.dev` |
| `RAW_CAPTURE_URL` | Friend's durable raw-capture receiver |
| `SEARCH_API_URL` | Friend's retrieval endpoint |
| `SEARCH_SERVICE_TOKEN` | Read-only credential for the search endpoint |
| `RAW_CAPTURE_TOKEN` | Ingestion-only credential for the capture receiver |
| `DATA_SERVICE_TOKEN` | Legacy shared fallback when a dedicated token is unset |
| `FIRECRAWL_API_KEY` | Linked-page scraping and web-context search |
| `OPENAI_API_KEY` | Editable query interpretation |
| `OPENAI_MODEL` | Optional model override; default `gpt-5-mini` |
| `AGENTMAIL_API_KEY` | Result-digest delivery |
| `AGENTMAIL_INBOX_ID` | Existing sender inbox |
| `AGENTMAIL_WEBHOOK_SECRET` | Verification of delivery webhooks |

Register AgentMail's webhook at `<deployment>.convex.site/agentmail/webhook` for delivery events. A send is queued only by the explicit Email → Send results action. The interface distinguishes queued/sent/delivered states.

The UI exposes account imports, search, and conversation collection. The same `jobs.start` API accepts `profile`, `following`, `followers`, and `archive`; these preserve complete responses for downstream account-discovery work. `bulk` supports `refresh:true` for engagement updates. All collection paths require a configured receiver, so an import never claims success by merely fetching data.

## Verify

```sh
bun run test
bun run build
```

Tests cover raw payload preservation, JSON backfill pagination, safe unordered-stream behavior, stream completion, partial capture, identity pinning, origin selection, retry timing, durable receipts, user isolation, and the Firecrawl component response shape. Provider calls are mocked in tests. No email is sent and no provider credits are consumed by the suite. Selected ideas and remaining work from the supplied local-first spec are tracked in [spec adoption](docs/spec-adoption.md).

## Hosting and current limits

The static-hosting Convex component is registered with root routing while auth and webhook routes remain intact. `bun run deploy` invokes its deployment workflow after a cloud project is configured. This has not been deployed or submitted. Before publishing, configure cloud auth keys and `SITE_URL` with the public address, provider credentials, both data-service endpoints, and AgentMail webhook verification; then test a real import, a search, a crawl, and a deliberate digest delivery.

Guest sessions let judges use the app without an invite. Saved state belongs to that browser session; clearing its credentials loses access. Per-session and global daily provider budgets bound usage. Guest sessions are not verified email identities: a public launch should add durable sign-in and a verified-recipient policy for email. Indexing is provider acquisition plus acknowledged raw handoff; the app does not claim that downstream normalization or indexing finished. Search pages are short-lived UI snapshots, not a local corpus. Only the temporary raw-file receiver is included; the collaborator owns the actual corpus/indexing server.
