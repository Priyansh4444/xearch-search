---
name: convex-test
description: "Generate convex-test tests for the app's Convex functions."
---

<!-- GENERATED from convex-agents content/capabilities/test.json — do not edit by hand. -->

# Generate Convex tests

Use convex-test + vitest to test functions against an in-memory backend: args/returns, auth paths, indexes, and scheduled functions.

## Workflow

1. Install `convex-test`, `vitest`, and `@edge-runtime/vm` with the project's package manager.
2. Configure Vitest with `test.environment: "edge-runtime"` before running tests.
3. Write tests using convexTest(schema): seed via t.run, call t.query/t.mutation, assert.
4. Cover auth (withIdentity), error paths, scheduled functions (t.finishInProgressScheduledFunctions), and relevant coverage targets.
5. Run Vitest deterministically.

## Rules

- Use convex-test (in-memory), not a live deployment.
- Require the edge-runtime Vitest environment and all three test packages before running the suite.
- Cover auth + error paths, not just the happy path.
- Keep tests deterministic (no real time/network).
