import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";
import { authTables } from "@convex-dev/auth/server";
export const sortValidator = v.union(
  v.literal("relevance"),
  v.literal("engagement"),
  v.literal("likes"),
  v.literal("newest"),
  v.literal("oldest"),
);
export const postFields = {
  tweetId: v.string(),
  author: v.string(),
  text: v.string(),
  url: v.string(),
  createdAt: v.optional(v.number()),
  likes: v.optional(v.number()),
  reposts: v.optional(v.number()),
  replies: v.optional(v.number()),
  links: v.array(v.string()),
  avatar: v.optional(v.string()),
  displayName: v.optional(v.string()),
};
export const kindValidator = v.union(
  v.literal("bulk"),
  v.literal("live"),
  v.literal("post"),
  v.literal("profile"),
  v.literal("following"),
  v.literal("followers"),
  v.literal("archive"),
);
export default defineSchema({
  ...authTables,
  collector: defineTable({
    name: v.string(),
    online: v.boolean(),
    lastSeen: v.number(),
  }).index("by_name", ["name"]),
  // Control-plane metadata only. Corpus bytes and normalization belong downstream.
  accounts: defineTable({
    handle: v.string(),
    userId: v.string(),
    name: v.string(),
    avatar: v.optional(v.string()),
  }).index("by_handle", ["handle"]),
  jobs: defineTable({
    owner: v.id("users"),
    kind: kindValidator,
    input: v.string(),
    since: v.optional(v.string()),
    until: v.optional(v.string()),
    refresh: v.boolean(),
    expectedUserId: v.optional(v.string()),
    cursor: v.optional(v.string()),
    nextUntil: v.optional(v.string()),
    nextCursor: v.optional(v.string()),
    status: v.union(
      v.literal("queued"),
      v.literal("running"),
      v.literal("complete"),
      v.literal("partial"),
      v.literal("failed"),
      v.literal("cancelled"),
    ),
    count: v.number(),
    phase: v.optional(v.string()),
    autoContinue: v.optional(v.boolean()),
    pages: v.optional(v.number()),
    pageAttempt: v.optional(v.number()),
    postsReceived: v.optional(v.number()),
    oldest: v.optional(v.string()),
    floorReached: v.optional(v.boolean()),
    attempt: v.number(),
    warnings: v.array(v.string()),
    error: v.optional(v.string()),
    updatedAt: v.number(),
    readyAt: v.optional(v.number()),
  })
    .index("by_status", ["status"])
    .index("by_owner", ["owner"])
    .index("by_input", ["kind", "input", "status"]),
  receipts: defineTable({
    jobId: v.id("jobs"),
    captureId: v.string(),
    receiptId: v.string(),
    records: v.number(),
  }).index("by_capture", ["jobId", "captureId"]),
  sessions: defineTable({
    owner: v.id("users"),
    raw: v.string(),
    sort: sortValidator,
    cursor: v.optional(v.string()),
    status: v.union(
      v.literal("queued"),
      v.literal("running"),
      v.literal("complete"),
      v.literal("failed"),
    ),
    rows: v.array(v.object(postFields)),
    nextCursor: v.optional(v.string()),
    warnings: v.array(v.string()),
    error: v.optional(v.string()),
  }).index("by_owner", ["owner"]),
  saved: defineTable({
    owner: v.id("users"),
    query: v.string(),
    sort: sortValidator,
  }).index("by_owner", ["owner"]),
  bookmarks: defineTable({ owner: v.id("users"), post: v.object(postFields) })
    .index("by_owner", ["owner"])
    .index("by_post", ["owner", "post.tweetId"]),
  pages: defineTable({
    url: v.string(),
    title: v.string(),
    text: v.string(),
    collectedAt: v.number(),
  }).index("by_url", ["url"]),
  deliveries: defineTable({
    owner: v.id("users"),
    outboundId: v.string(),
    query: v.string(),
  }).index("by_owner", ["owner"]),
  budgets: defineTable({ key: v.string(), count: v.number() }).index("by_key", [
    "key",
  ]),
});
