export type Sort = "relevance" | "engagement" | "likes" | "newest" | "oldest";
export function parseQuery(raw: string) {
  if (raw.length > 300) throw new Error("Keep searches under 300 characters.");
  const authors = [
    ...raw.matchAll(/(?:^|\s)(?:from:)?@([A-Za-z0-9_]{1,15})(?=\s|$)/g),
  ].map((m) => m[1].toLowerCase());
  if (new Set(authors).size > 1)
    throw new Error(
      "Search one author at a time, or remove the @ filters to search everyone.",
    );
  const text = raw
    .replace(/(?:^|\s)(?:from:)?@[A-Za-z0-9_]{1,15}(?=\s|$)/g, " ")
    .trim()
    .replace(/\s+/g, " ");
  if (/(?:^|\s)-?(?!https?:\/\/)[a-z_][a-z0-9_]*:/i.test(text))
    throw new Error(
      "Use @handle to filter authors. Other X operators are available through Find on X.",
    );
  return { text, author: authors[0] };
}
