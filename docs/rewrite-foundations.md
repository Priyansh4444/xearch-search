# Rewrite foundations

The plan changed on September 19: this repository is the shared application home.
The proposed throwaway SQLite store/search service was stopped before implementation.
Prronsh is building the local indexer and Elasticsearch integration. This branch
does not build a second indexer, change raw captures, or deploy anything.

## Reused from the old project

The Oxlint configuration follows [the old Xearch project](https://github.com/Priyansh4444/xearch):
TypeScript, Unicorn, Oxc, Vitest and React plugins; correctness errors; and the
Effect correctness, antipattern, effect-native and style presets. The existing
async-function, Node import and schema-number exceptions are preserved. Paths
are adapted to this repository; overrides for absent old collector apps are not copied.
Generated files, local captures and installed skills are outside lint scope.

Effect and its lint tooling use the old project's compatible versions. `prepare`
patches Oxlint and tsgolint on install, not TypeScript. `bun run lint` performs
type-aware linting and CI runs it before tests/build. The upstream Effect presets
include advisory warnings; those remain visible and are not presented as errors.
Four additional TypeScript checks are enabled: explicit overrides, switch
fallthrough, unchecked side-effect imports and consistent filename casing.
The old project's exact optional properties, checked index access and index-signature
property restrictions still need a separate migration across the existing code.

## Effect's first use

`convex/lib/results.ts` now uses Effect Schema to decode untrusted search-service
responses. `convex/search.ts` calls that decoder before publishing a result page.
The wire format is unchanged, including omitted warnings defaulting to an empty
array, HTTPS links, numeric post IDs as strings, finite metrics, and a 20-row cap.
Tests cover malformed input, unknown fields and the size limits.

This is a validation migration, not a conversion of Convex handlers into an Effect
runtime. Existing Zod validators remain until their own migrations have tests.

## Local indexer integration

Keep the existing contracts while the indexer is rewritten:

- Acquisition: `convex/lib/handoff.ts` describes versioned raw captures and exact
  durable acknowledgments. The current Mac worker/receiver remain transitional.
- Retrieval: `convex/search.ts` sends an authenticated POST with `version: 1`,
  keyword `query`, optional `author`, `sort`, optional opaque `cursor`, and `limit: 20`.
  Sorts are relevance, engagement, likes, newest and oldest. Responses use the
  Effect schema above. Elasticsearch ranking and cursor implementation belong to
  the local indexer, not a duplicated implementation here.
- Convex retains application state, sessions, ownership, job progress and sponsor
  integrations. Its raw receipt is not proof that Elasticsearch has published a post.

Still to agree together: how a hosted app reaches the local search service,
publication status, stable cursor semantics, deletion propagation, and the final
local worker interface. No localhost URL is configured on production by this PR.
