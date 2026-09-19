---
name: convex-improve-convex-plugin
description: "Send this coding session's transcript to the Convex team for an AI post-mortem that improves the quickstart system."
---

<!-- GENERATED from convex-agents content/capabilities/improve-convex-plugin.json — do not edit by hand. -->

# improve-convex-plugin

Sends the current coding session transcript to the anteater POST /review endpoint for an AI post-mortem. A transcript may contain prompts, code, logs, end-user content, or other personal data. The review returns structured findings targeted at the runbook, bootstrap script, skills, and components. Sharing is opt-in and informed consent must happen before any helper can access or transmit transcript data.

## Workflow

1. Ask the user to share Always, Just this once, or Never before running any transcript-related helper. Stop if they decline or have not answered.
2. Use either a reviewed local/pinned helper or a pinned authenticated artifact whose integrity is verified before execution. Never pipe an unpinned remote response into a shell.
3. Run the verified helper with the chosen consent and the one-line app idea from this session.
4. Watch for output markers: REVIEW_SOURCE (transcript found), REVIEW_SUBMITTED id=... (accepted), REVIEW_DONE status=done (findings ready).
5. Summarize the highest-severity findings for the user: title → target → suggestedFix, then wins. Keep the summary about the system, not the user's data.

## Rules

- Never let a helper read or send a transcript until the user has explicitly chosen to share.
- Never execute an unpinned remote helper. Require reviewed local code or a pinned authenticated artifact with an integrity check.
- REVIEW_NO_TRANSCRIPT means no Claude/Codex .jsonl was found — tell the user.
- Never paste raw secrets back — the script redacts keys/tokens before upload; keep the summary system-focused.
- This is a system-improvement loop, not end-user feature feedback.
