# Production deployment

App: https://utmost-kudu-321.convex.site

Dashboard: https://utmost-kudu-321.convex.site/?dashboard=1

Convex project: `xearch/xearch-next`. Deployment: `utmost-kudu-321` (production).

The pre-existing `xearch/xearch` production app was not changed. `.env.local` still selects local development. Production has separate auth keys and user records; local downloads and sessions were not migrated.

Deploy backend and frontend explicitly:

```sh
CONVEX_DEPLOYMENT=prod:utmost-kudu-321 bunx convex deploy
CONVEX_DEPLOYMENT=prod:utmost-kudu-321 bunx @convex-dev/static-hosting upload --build --prod --build-command 'bun run build'
```

`scripts/setup-production.mjs` copies selected provider variables from `.env.local`, not local capture settings. `--init-auth` is only for a fresh deployment and refuses existing keys. Secrets are passed over stdin, never printed.

AgentMail delivery events for the configured sender inbox are registered at `https://utmost-kudu-321.convex.site/agentmail/webhook`. The inbox-scoped API succeeded; the organization-level create route rejected the key. `AGENTMAIL_WEBHOOK_SECRET` is configured in production. Incoming email processing is not registered. No email was sent during setup.

Production imports use an outbound worker on the Mac. Run `bun run capture` and `bun run worker:production` in separate terminals. The worker authenticates to production with `.local-captures/worker-token`, claims one due job at a time, downloads directly from x.md, and saves to the private loopback receiver. Only job metadata and receipts return to Convex. No inbound port or public tunnel is used. Keep the Mac awake; the UI marks the worker offline within 45 seconds without a heartbeat. `scripts/setup-worker.mjs` configures its production credential without printing it.

Search remains disconnected until the collaborator provides retrieval. Firecrawl and OpenAI settings are configured, but paid calls have not been live-tested in production. Email sending requires a verified email identity; the current guest-only login cannot send production email.

Verified public HTML/assets, production guest authentication plus saved-search create/read/remove, and one real production profile download through the outbound worker with a durable local receipt. Browser visual checks were unavailable during deployment.
