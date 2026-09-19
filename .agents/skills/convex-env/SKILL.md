---
name: convex-env
description: "Set and wire Convex deployment env vars / secrets for the app."
---

<!-- GENERATED from convex-agents content/capabilities/env.json — do not edit by hand. -->

# Manage env vars + secrets

Store secrets as Convex deployment env vars, read them from supported Convex runtimes, and never commit or print them.

## Workflow

1. Select and announce the deployment through deploy-guard.
2. Set secrets without putting values in CLI arguments: use interactive input/stdin or `npx convex env set KEY --from-file value.txt`; use `--from-file .env` for multiple values.
3. Read environment variables in queries, mutations, actions, and HTTP actions. Prefer the generated typed `env` export when the app declares typed env vars; `process.env.KEY` is also supported. Keep external-I/O restrictions separate: queries and mutations still cannot perform arbitrary network I/O.
4. Never hardcode or commit secrets; add to .env.local only for local tooling that needs it.
5. Confirm names only with `npx convex env list --names-only`; never print full secret values.

## Rules

- Secrets live in Convex env vars, never in code or git.
- Environment variables are available to queries, mutations, actions, and HTTP actions. Prefer generated typed env access when configured.
- Do not pass secret values as CLI arguments or print them during verification.
- Environment access does not relax runtime I/O rules: external calls belong in actions/HTTP actions.
- Different deployments need their own values.
