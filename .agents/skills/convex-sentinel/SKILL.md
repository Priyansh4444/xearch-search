---
name: convex-sentinel
description: "Set up Sentinel production error capture in your own Convex deployment."
---

<!-- GENERATED from convex-agents content/capabilities/sentinel.json — do not edit by hand. -->

# Capture production errors in your own deployment

Install `@convex-dev/sentinel` to capture production errors (server function failures, client JS/React crashes, OCC and scale signals) into a table in the user's OWN deployment, redacted at write time, then react to new ones. Captured rows may remain in Convex; data selected for model analysis may be sent to the configured model provider.

## Workflow

1. Install the component: `app.use(sentinel)` in `convex/convex.config.ts`.
2. Wire the client SDK: a React error boundary plus `window.onerror`/`unhandledrejection` and breadcrumbs.
3. Redaction runs at write time and is on by default (default-deny on secret key names and value patterns).
4. Read recent errors with the Convex CLI (`convex data`, `run-once-query`); react to new ones via the monitor's `prod_error` event. Get explicit user consent before sending selected data to a model provider for analysis.
5. Optionally enable the self-healing cron: `triage` classifies each error and, for recurring non-transient ones, hands it to ai-runner to open a fix PR.

## Rules

- Redaction is mandatory and on by default — never store or send raw secrets.
- Captured data may remain inside Convex. Data selected for agent/model analysis may leave Convex for the configured model provider, and requires explicit user consent first.
- Keep service logs private.
- Sample and cap to control volume and cost.
- Capturing PROD errors needs a deployed cloud app; install works anonymously.
