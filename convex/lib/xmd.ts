import { z } from "zod";

const object = z.record(z.string(), z.unknown());
export type RawObject = Record<string, unknown>;
export class ProviderError extends Error {
  constructor(
    public code: string,
    message: string,
    public retryAfter = 0,
    public retryable = false,
    public raw?: RawObject,
  ) {
    super(message);
  }
}
export function record(value: unknown): RawObject {
  return object.parse(value);
}
export function string(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}
export function handle(value: string): string {
  const result = value.trim().replace(/^@/, "");
  if (!/^[A-Za-z0-9_]{1,15}$/.test(result))
    throw new Error("Enter a valid X handle, without a URL.");
  return result.toLowerCase();
}
export function publicUrl(value: string): string {
  const url = new URL(value);
  const host = url.hostname.toLowerCase();
  // Provider-side fetching still enforces its own DNS/private-network protection.
  if (
    url.protocol !== "https:" ||
    url.username ||
    url.password ||
    url.port ||
    host === "localhost" ||
    !host.includes(".") ||
    host.endsWith(".local") ||
    host.endsWith(".internal") ||
    host.includes(":") ||
    /^\d+\.\d+\.\d+\.\d+$/.test(host)
  )
    throw new Error("Use a public HTTPS website URL.");
  url.hash = "";
  return url.toString();
}
export function statusUrl(value: string): string {
  const url = new URL(publicUrl(value));
  if (
    !["x.com", "www.x.com", "twitter.com", "www.twitter.com"].includes(url.hostname) ||
    !/^\/[A-Za-z0-9_]{1,15}\/status\/\d+$/.test(url.pathname)
  )
    throw new Error("Paste an X post link, including /status/ and its ID.");
  return `https://x.com${url.pathname}`;
}
export function retryDelay(value: string | null, now = Date.now()): number {
  if (!value) return 30_000;
  const seconds = Number(value);
  return Math.min(
    86_400_000,
    Math.max(1000, Number.isFinite(seconds) ? seconds * 1000 : Date.parse(value) - now || 30_000),
  );
}
export class XmdClient {
  readonly origin: string;
  constructor(
    private key?: string,
    private fetcher: typeof fetch = fetch,
    origin = "https://mdfromx.com",
  ) {
    const url = new URL(origin);
    if (
      !["https://mdfromx.com", "https://x.pcstyle.dev"].includes(url.origin) ||
      url.username ||
      url.password ||
      (url.pathname !== "/" && url.pathname !== "")
    )
      throw new Error("X_MD_BASE_URL must be https://mdfromx.com or https://x.pcstyle.dev.");
    this.origin = url.origin;
  }
  private async request(path: string, query: Record<string, string>, signal?: AbortSignal) {
    const url = new URL(path, this.origin);
    for (const [k, v] of Object.entries(query)) url.searchParams.set(k, v);
    const response = await this.fetcher(url, {
      headers: {
        Accept: query.format === "ndjson" ? "application/x-ndjson" : "application/json",
        ...(this.key ? { Authorization: `Bearer ${this.key}` } : {}),
      },
      signal: signal ?? AbortSignal.timeout(120_000),
      redirect: "error",
    });
    if (!response.ok) {
      let code = `http_${response.status}`;
      let problem: RawObject | undefined;
      try {
        problem = record(await response.json());
        code = string(problem.code) ?? code;
      } catch {
        /* status is still actionable */
      }
      const message =
        response.status === 401
          ? "x.md rejected the API key. Check X_MD_API_KEY on the backend."
          : response.status === 429
            ? "x.md rate limit reached. The job will retry after the provider's delay."
            : `x.md could not finish this request (${response.status}, ${code}).`;
      throw new ProviderError(
        code,
        message,
        retryDelay(response.headers.get("Retry-After")),
        [408, 429, 500, 502, 503, 504].includes(response.status),
        problem ? { error: problem, httpStatus: response.status } : undefined,
      );
    }
    return response;
  }
  async read(
    kind: "profile" | "search" | "post" | "following" | "followers" | "archive",
    input: string,
    cursor?: string,
  ) {
    const query: Record<string, string> = { format: "json", limit: "50" };
    if (cursor) query.cursor = cursor;
    let path: string;
    if (kind === "search") {
      path = "/api/v1/search";
      query.q = input;
      query.feed = "latest";
    } else if (kind === "post") {
      path = "/api/v1/posts";
      query.url = statusUrl(input);
      query.thread = "auto";
    } else {
      path = `/api/v1/profiles/${handle(input)}${kind === "profile" ? "" : kind === "archive" ? "/posts" : `/${kind}`}`;
      if (kind === "archive") query.index = "true";
    }
    return record(await (await this.request(path, query)).json());
  }
  async history(
    input: string,
    options: {
      since?: string;
      until?: string;
      maxPosts: number;
      refresh?: boolean;
    },
  ): Promise<RawObject> {
    const query: Record<string, string> = {
      format: "json",
      max_posts: String(Math.min(500, Math.max(1, options.maxPosts))),
      with_replies: "true",
      with_reposts: "true",
      concurrency: "8",
    };
    if (options.since) query.since = options.since;
    if (options.until) query.until = options.until;
    if (options.refresh) query.refresh = "true";
    return record(
      await (await this.request(`/api/v1/profiles/${handle(input)}/posts`, query)).json(),
    );
  }
  async *bulk(
    input: string,
    options: {
      since?: string;
      until?: string;
      maxPosts: number;
      refresh?: boolean;
    },
  ): AsyncGenerator<RawObject & ({ post: RawObject } | { meta: RawObject; profile?: RawObject })> {
    const query: Record<string, string> = {
      format: "ndjson",
      max_posts: String(Math.min(500, Math.max(1, options.maxPosts))),
      with_replies: "true",
      with_reposts: "true",
      concurrency: "8",
    };
    if (options.since) query.since = options.since;
    if (options.until) query.until = options.until;
    if (options.refresh) query.refresh = "true";
    const response = await this.request(`/api/v1/profiles/${handle(input)}/posts`, query);
    if (!response.body) throw new ProviderError("empty_stream", "x.md returned no import stream.");
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let pending = "",
      terminal = false,
      count = 0;
    const parse = (line: string) => {
      const item = record(JSON.parse(line));
      if (item.error)
        throw new ProviderError(
          "partial_import",
          "x.md stopped before completing the import. Only acknowledged captures are retained; retry to continue.",
          0,
          false,
          item,
        );
      if (item.post) return { ...item, post: record(item.post) };
      if (item.meta)
        return {
          ...item,
          meta: record(item.meta),
          ...(item.profile ? { profile: record(item.profile) } : {}),
        };
      throw new ProviderError("invalid_stream", "x.md returned an unrecognized import record.");
    };
    try {
      while (true) {
        const { done, value } = await reader.read();
        pending += decoder.decode(value, { stream: !done });
        if (pending.length > 2_000_000)
          throw new ProviderError(
            "oversized_record",
            "An x.md import record exceeded the size limit.",
          );
        const lines = pending.split("\n");
        pending = lines.pop()!;
        if (done && pending.trim()) {
          lines.push(pending);
          pending = "";
        }
        for (const line of lines) {
          if (!line.trim()) continue;
          if (terminal)
            throw new ProviderError(
              "invalid_stream",
              "x.md sent records after the import summary.",
            );
          const item = parse(line);
          if ("meta" in item) terminal = true;
          else if (++count > options.maxPosts)
            throw new ProviderError(
              "import_limit",
              "The provider exceeded the requested import size. Collected sources were kept.",
            );
          yield item;
        }
        if (done) break;
      }
      if (!terminal)
        throw new ProviderError(
          "incomplete_stream",
          "The connection closed without an import summary. Collected sources were kept; retry to continue.",
        );
    } finally {
      await reader.cancel().catch(() => {});
      reader.releaseLock();
    }
  }
}
