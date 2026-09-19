# Xearch

An X search experience modeled on https://search.pronsh.dev/: a central search box, account selection, @author filtering, useful sorting, original post links, and conversation context.

## Ownership

Selected ideas and follow-up boundaries from the supplied local-first specification are recorded in [spec adoption](spec-adoption.md).

This build owns x.md-powered indexing acquisition and app integration. The user's friend owns raw storage, normalization, corpus policy, and the data/search implementation. See [the handoff contract](integration-contract.md).

This follows the boundary in the [previous acquisition ADR](https://github.com/Priyansh4444/xearch/blob/master/docs/adr/0001-file-bound-collection-normalization.md): retain raw responses with request metadata, acknowledge them, and normalize separately. The new collector does not import the old normalizer or write posts to Convex. Numeric account identity is pinned before bulk collection. Incomplete upstream streams and failed storage receipts remain incomplete.

## Meaningful use of the sponsor tools

| Tool      | Product work                                                                                                                                                                                                          |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Convex    | Guest sessions, private saved searches/bookmarks, indexing job scheduling, atomic receipt/checkpoint updates, bounded retries, live search-session updates, and email delivery state.                                 |
| x.md      | Bulk account histories, incremental top-ups, historical continuation, metric refresh, profile reads, post/thread context, live search, archive inspection, followers/following. Both official hosts are configurable. |
| Firecrawl | Read the actual page behind a post's link; search the web for context around the current X query. Hand original provider responses to the same downstream raw-capture receiver when connected.                        |
| OpenAI    | Interpret a natural-language request as an editable keyword/author query. Does not silently change a submitted query.                                                                                                 |
| AgentMail | Send an explicitly requested result digest and subscribe to delivery state through its Convex component.                                                                                                              |
| Codex     | Build, inspect reference interfaces, run contract tests and browser verification, and maintain an evidence-based hackathon log. Codex is the development tool, not an invented runtime dependency.                    |

Convex retains bounded, temporary search-page and web-preview snapshots for the interface, plus deliberately saved bookmarks. It contains no corpus table, full-text index, token postings, tokenizer, ranking engine, or normalization implementation. Search sessions and preview caches expire after a day.

## Hackathon

Rules checked at https://www.convex.dev/hackathons/all-gas on September 19, 2026.

Deadline: September 22 at noon Pacific / 21:00 Warsaw. The entry needs a new app started within the permitted window, Convex and partner integrations, a public repository, root hackathon.md, public convex.site or chatgpt.site URL, and a demo under three minutes. Register, share/tag the sponsors, and submit through the linked forms. The original page governs eligibility.

The reference supplies the interaction. X-specific indexing, context reading, and saved-search delivery supply the actual product utility. This local build is not yet a live or submitted entry.
