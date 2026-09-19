# Xearch VM transfer — 2026-09-19

PRIVATE: this archive contains real API keys, webhook secrets, worker tokens,
local Convex credentials/database state, and captured data. Transfer it using
SSH/SCP or another private channel. Do not attach it to a public issue or commit it.
The ZIP is not encrypted; its local file permissions restrict access to its owner.

## Included

- `xearch/`: current working tree and Git history, on `adam/rewrite-foundations`,
  commit `3b7b927b61af4ca024252a2d9d05668c0f86e4e3` (PR #1, not merged).
- Root `.env.local`, `.env.production-target`, and example env files.
- `.local-captures/raw/`, capture token, and production worker token.
- `.convex/` configuration, local file storage, and a SQLite online-backup snapshot.
- `xearch-old/`: the current local reference checkout, including its Git history,
  commit `6158b701a102da35e61da54d7edba39a7ffd4be2`.
- Project skills, tests, deployment scripts, lockfiles, and documentation.

Excluded: node_modules, dist, Rust target directories, log/temp files, and macOS
Finder metadata. Reinstall dependencies for the VM; do not move Mac binaries.
Global GitHub/Convex login stores, SSH keys, and browser sessions are not included.
Hosted Convex production data is not exported: it remains in the existing deployment.
Raw capture files were copied while the services remained running; writes after
the copy are not included. The local database backup is transactionally consistent,
but it is not a coordinated snapshot with live imports or the file-storage directory.

## Start the existing production download worker on Linux

1. Extract into a private directory owned by the service user; keep it outside any
   public web root. Use `umask 077` and restrict access to `.env*`, `.convex/`, and
   `.local-captures/` after extraction.
2. Install Bun 1.4.0 and Node.js 24 or newer. From `xearch/`, run
   `bun install --frozen-lockfile`, then `bun run test` and `bun run build`.
3. Stop the old Mac production worker before starting its VM replacement. This
   archive does not stop the Mac or switch traffic. Coordinate a final capture
   sync if imports continued after packaging.
4. Run `bun run capture`, then `bun run worker:production` in separate processes.
   The receiver binds only to `127.0.0.1:4319`; keep it private. Both commands must
   run from the `xearch/` directory. Use a service manager for persistent hosting.
5. The worker already targets `https://utmost-kudu-321.convex.cloud` and uses the
   included worker token. It does not require copying your global Convex login.

This runs the receiver/worker on the VM while the existing app and application
state remain hosted by Convex. It does not move the whole Convex backend onto the
VM. Production email webhook configuration also remains at its existing Convex URL.

## Development and next steps

`.env.local` selects local development, not production. `bun run backend` and
`bun run dev` are development commands, not a production hosting setup. The local
Convex database backup is included for recovery; cross-platform restoration has
not been tested. Do not expose the local Convex backend or development server to
the internet. Reauthenticate on the VM if you need deployment CLI access.

Elasticsearch and the new local indexer are still Prronsh's work. No temporary
SQLite search server was implemented. See `docs/rewrite-foundations.md` and
`docs/production.md` before changing URLs or deployment settings. Linux startup
and a VM cutover have not been performed as part of this archive task.
