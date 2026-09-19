# Local search implementation

- Requested outcome: reviewed stacked PRs, not merge or production deployment.
- Worktree: `/home/exedev/xearch-search`; base `origin/main`.
- Stack: `adam/search-contracts` -> `adam/search-tantivy` -> `adam/search-api`.
- Tantivy owns postings and retrieval on this VM. Backend-neutral requests, query AST and results allow a later Elasticsearch adapter.
- Keep stop words, lowercase Unicode tokens, no stemming. Quoted phrases require adjacency. No automatic query relaxation.
- Initial validation corpus is the supplied 1,708-post dump. Never check the dump or production secrets into Git.
- Use bounded disk-backed indexing, no corpus-sized resident cache. No paid collection or changes to running production services.
- Done requires behavior, deterministic randomized oracle tests, speed measurements, CLI and HTTP verification, green checks and reviewed current PR heads.
- Private browser integration must preserve Convex ownership and bookmarks; browser result data must be authenticated before persistence.
