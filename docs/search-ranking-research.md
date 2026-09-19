# Search ranking on the 8 GB VM

Research checked 2026-09-19. This covers the retrieval families relevant to this
service, not every published model. Model cards establish capabilities, not
latency on this machine. No neural model was downloaded or benchmarked for this
note. Runtime measurements belong in the accompanying benchmark report.

## Recommendation

Keep Tantivy 0.26.2 and BM25 as the default. Keep all words, including stop words,
in the positional index. Author filters, exact phrases, exclusions, dates, and
sorts are the primary requirements. They do not require a neural model.

The current adapter opens a `MmapDirectory`, uses a single writer with a
32,000,000-byte indexing buffer, and bounds its stored-document cache to eight
blocks. That writer budget is not a process RSS limit. Memory mapping lets the
OS page index files into memory on demand; it does not mean zero resident memory
or zero disk-cache use. See [Tantivy's directory documentation](https://docs.rs/tantivy/0.26.2/tantivy/directory/struct.MmapDirectory.html).

Elasticsearch remains a reasonable later backend when its distributed operations
justify another service. Its heap is only part of its memory footprint; it also
needs off-heap memory and filesystem cache. On a shared 8 GB workstation I prefer
the embedded library. This is an operational judgment, not a measured
Tantivy-versus-Elasticsearch performance result. See [Elastic's JVM sizing guidance](https://www.elastic.co/docs/reference/elasticsearch/jvm-settings).

Keep backend-specific index types and query compilation behind the existing
`SearchBackend` and `IndexSink` interfaces. Preserve normalized raw captures so a
backend change can build a fresh index. Tantivy index files are not a portable
interchange format. Contract tests should define the matching behavior; engines
need not produce numerically identical BM25 scores.

## Options worth evaluating

| Family / exact model | Offline work | Work for each new query | Decision here |
| --- | --- | --- | --- |
| Tantivy BM25 | Tokenize and write postings, positions, and metadata | Parse, retrieve, score, select results | Ship and measure first |
| BM25 plus engagement and freshness | Store numeric features | Read features for matching documents | Already supported; benchmark broad queries separately |
| `BAAI/bge-small-en-v1.5` | Embed each tweet into 384 dimensions | Encode the query and retrieve vector candidates | First optional semantic experiment |
| `intfloat/e5-small-v2` | Embed passages with the required prefix | Encode with the query prefix and retrieve candidates | Alternative English retrieval baseline |
| `sentence-transformers/all-MiniLM-L6-v2` | Embed tweets into 384 dimensions | Encode query and retrieve candidates | Useful compact similarity baseline |
| `cross-encoder/ms-marco-MiniLM-L6-v2` or `cross-encoder/ms-marco-TinyBERT-L2-v2` | No reusable query-independent pair score | Run inference on each query/candidate pair | Optional bounded reranker after measurement |
| SPLADE sparse neural retrieval | Compute sparse learned term weights | Neural query encoding, then sparse retrieval | Defer pending demonstrated lexical recall problems |
| ColBERT late interaction | Store multiple token embeddings per tweet | Encode query and compare token representations | Defer because it adds storage and scoring machinery |
| `Qwen/Qwen3-Embedding-0.6B` | Encode documents with a larger multilingual model | Encode query, then retrieve | Defer on this shared CPU machine |

BGE small supports 512-token inputs and documents an ONNX inference path. Its
card recommends a retrieval instruction for short queries and no instruction
for documents. E5 small is English-only, supports 512 tokens, and requires the
`query: ` / `passage: ` prefixes. MiniLM's default limit is 256 word pieces, so
long X posts require an explicit truncation or chunking policy. These are
documented capabilities; none of these models has demonstrated a quality win on
our tweets yet. Sources: [BGE card](https://huggingface.co/BAAI/bge-small-en-v1.5),
[E5 card](https://huggingface.co/intfloat/e5-small-v2),
[MiniLM card](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2).

Cross-encoders score pairs, so indexing tweets offline cannot precompute their
answers for unseen queries. Their published throughput is not this VM's CPU
throughput. Try at most a small candidate set in an isolated experiment before
putting one in the request path. Sources: [cross-encoder architecture](https://www.sbert.net/examples/cross_encoder/applications/README.html)
and [published models](https://www.sbert.net/docs/cross_encoder/pretrained_models.html).

SPLADE uses learned sparse representations. ColBERT keeps token-level
representations for late interaction. Qwen's 0.6B embedding model supports
multilingual input and output dimensions from 32 to 1024. These capabilities do
not establish that the added work is useful for our operator-heavy searches.
Sources: [SPLADE](https://github.com/naver/splade),
[ColBERT](https://github.com/stanford-futuredata/ColBERT),
[Qwen model card](https://huggingface.co/Qwen/Qwen3-Embedding-0.6B).

If semantic search earns a place, first compare quantized ONNX BGE small against
the lexical baseline, with one inference thread and bounded batches. Keep exact
filters mandatory. Combine lexical and semantic candidate rankings instead of
assuming their raw scores share a scale. Pin the model revision, tokenizer,
pooling, normalization, and quantization in the index manifest. The same settings
must encode documents and queries. ONNX and quantization are documented options,
but speed and quality must be measured locally. See [Sentence Transformers inference optimization](https://www.sbert.net/docs/sentence_transformer/usage/efficiency.html).

For capacity planning only, one million 384-dimensional float32 embeddings use
1,536,000,000 bytes, about 1.43 GiB, before IDs, an ANN structure, model weights,
and runtime allocations. Int8 values alone use about 0.36 GiB. These are
arithmetic storage estimates, not observed index sizes or RSS. Offline document
embedding removes document inference from requests; query inference remains.

## Stop words and ranking correctness

The current analyzer is `SimpleTokenizer` plus lowercase, with positions and no
stemming or stop-word filter. Tantivy exposes stop-word filtering as an optional
token filter. We deliberately do not enable it. See [versioned tokenizer documentation](https://docs.rs/tantivy/0.26.2/tantivy/tokenizer/index.html).

Words such as `go`, `not`, `to`, and `be` can carry meaning in technical tweets
and quoted phrases. `"to be"`, `"not safe"`, and `"go to"` must retain their
words and ordering. Lowercase token matching is not byte-for-byte matching:
punctuation is split by the analyzer. Add fixtures for apostrophes, hashtags,
Unicode, and programming names such as `C++` before promising their semantics.

Broad common-word queries may visit many postings. Removing a user's required
word to make a query cheaper changes correctness. Measure them separately and
bound concurrency instead. Compare relevance-only and custom feature ranking:
collector choice can affect top-k optimizations. Do not assume custom scoring
has the same cost as the stock score collector. See [Tantivy TopDocs](https://docs.rs/tantivy/0.26.2/tantivy/collector/struct.TopDocs.html).

## Acceptance and measurements

Treat 200 ms as the target and 300 ms as the upper p95 acceptance threshold for
local authenticated HTTP search at the declared workload. Report concurrency,
corpus size, segment count, query distribution, build mode, and whether indexing
ran concurrently. A tiny-corpus microbenchmark does not prove that SLA at scale.
This is a proposed benchmark criterion, not a claim that requests meet it.

Measure first request after process start separately from steady state. That
first request is not necessarily disk-cold; another process may have warmed the
OS cache. Do not flush system caches on this shared production machine.

Record p50/p90/p95/p99/max latency, throughput, errors, overloads, queue time,
search time, serialization time, process RSS/peak RSS, CPU, read/write bytes,
page faults, index size, import rate, and commit latency where instrumentation
exists. Mark uninstrumented values unavailable. Include rare terms, frequent
terms, all-stop-word queries, phrases, author-only searches, negative clauses,
all sorts, and pagination. Use deterministic random seeds for reproducibility.

For model selection, hand-label a small held-out set of representative tweet
queries and compare recall@20, MRR@10, and nDCG@10 alongside latency and memory.
Randomized behavior tests establish invariants, not human relevance. Add a
neural model only if that evaluation demonstrates a useful gain within the same
resource and latency budget.
