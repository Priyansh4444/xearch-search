# Hackathon log

- **Project:** Xearch
- **What it does:** Search X posts and conversations, collect account histories through x.md, and read linked web context.
- **Live app:** https://utmost-kudu-321.convex.site
- **Repo:** https://github.com/pc-style/xearch
- **Frontend:** Convex static hosting
- **Convex deployment:** https://utmost-kudu-321.convex.cloud
- **Components:** @convex-dev/static-hosting
- **Convex features:** authenticated sessions, queries, mutations, scheduled actions, receipt/checkpoint transactions, reactive search and delivery state, bounded cleanup cron
- **Auth:** Convex Auth
- **AI models:** gpt-5-mini (code fallback); production model is environment-configured
- **Started:** 2026-09-19T04:38:51Z
- **Last updated:** 2026-09-19T04:38:51Z

## Log

### 2026-09-19 — working tree

Built the search interface with Codex after inspecting search.pronsh.dev. Added x.md bulk/profile/search/conversation/connection acquisition, exact raw-capture handoff, durable receipt validation, account identity pinning, and partial-stream handling. The handoff preserves the old project's separation between acquisition and normalization. Storage and normalization belong to a collaborator's service; the application does not implement a corpus database.

Registered Firecrawl, AgentMail, and static-hosting components. Added linked-page preview and web-context search, editable OpenAI query assistance, and explicit email-result delivery. Authenticated users own saved searches, bookmarks, and search sessions. API keys remain in the backend. Used Codex to run contract tests and inspect the browser interface.

Validated 29 tests plus TypeScript and a production frontend build. Local browser checks covered the search interface, missing-connection state, and saving/reopening a search through a fresh Convex guest session. The mobile viewport check was interrupted by an unavailable browser automation host and remains unverified. Provider integrations have contract tests but await credentials and the collaborator's receiver/retrieval endpoints for live verification. No live import, paid crawl, OpenAI generation, email send, public deployment, or hackathon submission has been performed.

Incorporated the supplied local-first specification selectively: JSON history now drives backfill pagination, while capped unordered NDJSON never produces an unsafe oldest-based continuation. Added tests for intact JSON envelopes, missing completion metadata, and changed identity. Recorded remaining application/integration work in `docs/spec-adoption.md`; no collaborator-owned storage or retrieval engine was implemented.

Follow-up integration hardening: separated read-only search and raw-capture credentials with a documented legacy fallback. Firecrawl previews now expose collection time, retain that time on cache hits, and store only a 4,000-character excerpt. OpenAI interpretation pins explicit author filters server-side and rejects unsupported hard operators before a model request. The updated suite passes 36 tests, TypeScript, and a production build. These new behaviors are contract-tested, not live-provider or browser verified.

At the user's request, added a temporary loopback capture receiver with token authentication, body checksums, atomic file publication, file/directory sync, and replay-stable receipts. Added the import dashboard with all seven collection tasks, Convex-subscribed stages, stop/retry/continuation controls, and private receipt inspection. The frontend-design skill guided the existing-theme control-panel layout. Raw files remain outside Convex and no normalizer or search index was added. Configured the receiver in the local deployment and ran one real x.md profile-read job through Convex: it completed with one durable receipt. No bulk history import was started. The test suite now passes 39 tests plus TypeScript and production build; browser automation was unavailable for visual dashboard verification. The local frontend, Convex backend, and receiver were left running.

Existing x.md is used as an external service. `xearch-old` was read for its handoff boundary and remains unchanged. This log describes new local work, not an assertion that prior services were built during the event.

### 2026-09-19 — import flow correction

Inspected the user's saved history responses: three 500-post batches each reported more history, not a provider history floor. Replaced manual per-page history imports with bounded automatic continuation in the same job, including budget pauses and a non-advancing boundary check. Added received-post counts and oldest dates; moved file receipts into technical details. Used unslop to remove "handed off", "raw envelopes", and storage jargon from both import views. Synced allowlisted local environment variables without printing values, including the user's model selection. No AgentMail webhook was registered or email sent. Added tests for continuation, count idempotence, stuck boundaries, and budget pauses. 42 tests pass; full historical coverage is not claimed.


### 2026-09-19 — production deployment

Created a separate `xearch-next` project in the Xearch team and deployed production backend and static frontend to `utmost-kudu-321`. The existing Xearch production deployment was untouched. Synced selected provider settings and separate auth keys. Registered an inbox-scoped AgentMail delivery webhook and stored its signing secret in production. Verified public HTML/assets and guest authentication plus saved-search create/read/remove. Local captures were not moved or exposed; production imports and retrieval remain disconnected. Production email sending now requires a verified email identity, with a regression test. 43 tests pass. No real email was sent and no full-history completion is claimed.


### 2026-09-19 — enable production imports

Connected production to the temporary Mac receiver through an outbound worker. Added authenticated worker polling/reporting, single-job claims, retry due times, heartbeat expiry, and an explicit offline explanation in the dashboard. No raw post bodies are queued in Convex and no inbound port or tunnel was opened. A real production profile-read job completed with one durable receipt; the frontend was republished with import controls enabled while the worker is online. 46 tests pass plus production build. The worker requires the Mac to remain running. Full-history coverage, production search, and email sending are not claimed.


### 2026-09-19 - repository setup

Added the official Convex hackathon skill with its references and license under `.agents/skills/hackathon`. Removed the old reference checkout from this repository and kept upstream architecture attribution in the docs. Added ignore rules for secrets, captures, databases, backups, and build output. The new Git history starts here; the earlier build chronology above comes from this work session and source timestamps, not older commits. Source timestamps are weaker evidence than Git history. Registered third-party integrations remain Firecrawl and AgentMail in `convex/convex.config.ts`; their runtime verification limits are recorded above.

### 2026-09-19 - 1198654

Recorded the initial application, integration code, tests, and hackathon skill in the first Git commit. The repository remote is `https://github.com/pc-style/xearch`. Updated the header's start time to this first commit; the earlier source-based start estimate was `2026-09-19T01:44:12Z`. The preceding entries preserve that pre-commit work chronology. This checkpoint adds Git evidence, not a new deployment or integration verification.
