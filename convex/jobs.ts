import { v, ConvexError } from "convex/values";
import { query, mutation, internalMutation } from "./_generated/server";
import { internal } from "./_generated/api";
import { kindValidator } from "./schema";
import { user } from "./access";
import { handle, statusUrl } from "./lib/xmd";

export const list = query({
  args: {},
  handler: async (ctx) => {
    const owner = await user(ctx);
    return ctx.db
      .query("jobs")
      .withIndex("by_owner", (q) => q.eq("owner", owner))
      .order("desc")
      .take(20);
  },
});
// Local upgrade: recover display statistics from an already-acknowledged capture.
export const restoreSummary = internalMutation({
  args: {
    jobId: v.id("jobs"),
    captureId: v.string(),
    posts: v.number(),
    oldest: v.optional(v.string()),
    floorReached: v.boolean(),
  },
  handler: async (ctx, args) => {
    const job = await ctx.db.get(args.jobId);
    if (!job || job.kind !== "bulk" || job.status !== "complete" || job.postsReceived !== undefined)
      return;
    const receipt = await ctx.db
      .query("receipts")
      .withIndex("by_capture", (q) => q.eq("jobId", job._id).eq("captureId", args.captureId))
      .unique();
    if (!receipt || !Number.isInteger(args.posts) || args.posts < 0 || args.posts > 500)
      throw new Error("Invalid saved batch summary");
    await ctx.db.patch(job._id, {
      postsReceived: args.posts,
      pages: 1,
      oldest: args.oldest,
      floorReached: args.floorReached,
    });
  },
});
export const start = mutation({
  args: {
    kind: kindValidator,
    input: v.string(),
    since: v.optional(v.string()),
    refresh: v.optional(v.boolean()),
    previous: v.optional(v.id("jobs")),
  },
  handler: async (ctx, args) => {
    const owner = await user(ctx);
    const outbound = process.env.COLLECTOR_MODE === "outbound";
    const worker = outbound
      ? await ctx.db
          .query("collector")
          .withIndex("by_name", (q) => q.eq("name", "desktop"))
          .unique()
      : null;
    if (outbound && (!worker?.online || Date.now() - worker.lastSeen > 45_000))
      throw new ConvexError(
        "The download worker is offline. Imports will resume when it reconnects.",
      );
    if (!process.env.X_MD_API_KEY || (!outbound && !process.env.RAW_CAPTURE_URL))
      throw new ConvexError(
        "Connect x.md and the raw-capture receiver before starting an indexing job.",
      );
    const input =
      args.kind === "live"
        ? args.input.trim()
        : args.kind === "post"
          ? statusUrl(args.input)
          : handle(args.input);
    if (!input || input.length > 300) throw new ConvexError("Enter a search under 300 characters.");
    if (
      args.since &&
      (!/^\d{4}-\d{2}-\d{2}$/.test(args.since) || !Number.isFinite(Date.parse(args.since)))
    )
      throw new ConvexError("Choose a valid start date.");
    const previous = args.previous ? await ctx.db.get(args.previous) : null;
    if (
      args.previous &&
      (!previous ||
        previous.owner !== owner ||
        previous.input !== input ||
        previous.kind !== args.kind)
    )
      throw new ConvexError("Continuation does not belong to this indexing job.");
    for (const status of ["running", "queued"] as const) {
      if (
        await ctx.db
          .query("jobs")
          .withIndex("by_input", (q) =>
            q.eq("kind", args.kind).eq("input", input).eq("status", status),
          )
          .first()
      )
        throw new ConvexError("This indexing job is already active.");
    }
    const account =
      args.kind === "bulk"
        ? await ctx.db
            .query("accounts")
            .withIndex("by_handle", (q) => q.eq("handle", input))
            .unique()
        : null;
    const id = await ctx.db.insert("jobs", {
      owner,
      kind: args.kind,
      input,
      since: previous?.since ?? args.since,
      until: previous?.nextUntil,
      cursor: previous?.nextCursor,
      refresh: args.refresh ?? false,
      autoContinue: args.kind === "bulk",
      pages: 0,
      postsReceived: 0,
      expectedUserId: previous?.expectedUserId ?? account?.userId,
      status: "queued",
      count: 0,
      attempt: 0,
      warnings: [],
      updatedAt: Date.now(),
    });
    await ctx.scheduler.runAfter(0, internal.importer.run, { jobId: id });
    return id;
  },
});
export const claim = internalMutation({
  args: { jobId: v.id("jobs") },
  handler: async (ctx, { jobId }) => {
    const job = await ctx.db.get(jobId);
    if (!job || job.status !== "queued" || (job.readyAt ?? 0) > Date.now()) return null;
    const attempt = job.attempt + 1;
    await ctx.db.patch(jobId, {
      status: "running",
      attempt,
      pageAttempt: (job.pageAttempt ?? 0) + 1,
      updatedAt: Date.now(),
      error: undefined,
    });
    await ctx.scheduler.runAfter(600_000, internal.jobs.expire, {
      jobId,
      attempt,
    });
    return { ...job, attempt };
  },
});
export const progress = internalMutation({
  args: { jobId: v.id("jobs"), attempt: v.number(), phase: v.string() },
  handler: async (ctx, args) => {
    const job = await ctx.db.get(args.jobId);
    if (!job || job.status !== "running" || job.attempt !== args.attempt)
      throw new Error("Job is no longer active.");
    await ctx.db.patch(job._id, { phase: args.phase, updatedAt: Date.now() });
  },
});
export const cancel = mutation({
  args: { jobId: v.id("jobs") },
  handler: async (ctx, { jobId }) => {
    const owner = await user(ctx),
      job = await ctx.db.get(jobId);
    if (!job || job.owner !== owner) throw new ConvexError("Job not found.");
    if (!["queued", "running"].includes(job.status)) return;
    await ctx.db.patch(jobId, {
      status: "cancelled",
      phase: "Stopped; an in-flight request may still finish. Retained captures are not deleted.",
      updatedAt: Date.now(),
    });
  },
});
export const retry = mutation({
  args: { jobId: v.id("jobs") },
  handler: async (ctx, { jobId }) => {
    const owner = await user(ctx),
      job = await ctx.db.get(jobId);
    if (!job || job.owner !== owner) throw new ConvexError("Job not found.");
    if (!["failed", "partial", "cancelled"].includes(job.status))
      throw new ConvexError("Only stopped or failed jobs can be retried.");
    for (const status of ["queued", "running"] as const) {
      const active = await ctx.db
        .query("jobs")
        .withIndex("by_input", (q) =>
          q.eq("kind", job.kind).eq("input", job.input).eq("status", status),
        )
        .first();
      if (active) throw new ConvexError("This indexing job is already active.");
    }
    await ctx.db.patch(jobId, {
      status: "queued",
      readyAt: 0,
      error: undefined,
      phase: "Retry queued",
      updatedAt: Date.now(),
    });
    await ctx.scheduler.runAfter(0, internal.importer.run, { jobId });
  },
});
export const receipts = query({
  args: { jobId: v.id("jobs") },
  handler: async (ctx, { jobId }) => {
    const owner = await user(ctx),
      job = await ctx.db.get(jobId);
    if (!job || job.owner !== owner) throw new ConvexError("Job not found.");
    return ctx.db
      .query("receipts")
      .withIndex("by_capture", (q) => q.eq("jobId", jobId))
      .take(100);
  },
});
export const pinIdentity = internalMutation({
  args: { jobId: v.id("jobs"), attempt: v.number(), userId: v.string() },
  handler: async (ctx, args) => {
    const job = await ctx.db.get(args.jobId);
    if (!job || job.status !== "running" || job.attempt !== args.attempt)
      throw new Error("Indexing job is no longer active.");
    if (job.expectedUserId && job.expectedUserId !== args.userId)
      throw new Error("Account identity changed.");
    await ctx.db.patch(job._id, { expectedUserId: args.userId });
  },
});
export const expire = internalMutation({
  args: { jobId: v.id("jobs"), attempt: v.number() },
  handler: async (ctx, args) => {
    const job = await ctx.db.get(args.jobId);
    if (job?.status === "running" && job.attempt === args.attempt)
      await ctx.db.patch(job._id, {
        status: job.count ? "partial" : "failed",
        error: "Collection timed out. Only acknowledged captures are recorded; retry to continue.",
        updatedAt: Date.now(),
      });
  },
});
export const ack = internalMutation({
  args: {
    jobId: v.id("jobs"),
    attempt: v.number(),
    captureId: v.string(),
    receiptId: v.string(),
    count: v.number(),
  },
  handler: async (ctx, args) => {
    const job = await ctx.db.get(args.jobId);
    if (!job || job.status !== "running" || job.attempt !== args.attempt)
      throw new Error("Indexing job is no longer active.");
    const existing = await ctx.db
      .query("receipts")
      .withIndex("by_capture", (q) => q.eq("jobId", job._id).eq("captureId", args.captureId))
      .unique();
    if (existing) return;
    await ctx.db.insert("receipts", {
      jobId: job._id,
      captureId: args.captureId,
      receiptId: args.receiptId,
      records: args.count,
    });
    await ctx.db.patch(job._id, {
      count: job.count + args.count,
      updatedAt: Date.now(),
    });
  },
});
export const finish = internalMutation({
  args: {
    jobId: v.id("jobs"),
    attempt: v.number(),
    warnings: v.array(v.string()),
    error: v.optional(v.string()),
    retryAfter: v.optional(v.number()),
    nextUntil: v.optional(v.string()),
    nextCursor: v.optional(v.string()),
    expectedUserId: v.optional(v.string()),
    postsReceived: v.optional(v.number()),
    oldest: v.optional(v.string()),
    floorReached: v.optional(v.boolean()),
    profile: v.optional(
      v.object({
        handle: v.string(),
        userId: v.string(),
        name: v.string(),
        avatar: v.optional(v.string()),
      }),
    ),
  },
  handler: async (ctx, args) => {
    const job = await ctx.db.get(args.jobId);
    if (!job || job.status !== "running" || job.attempt !== args.attempt) return;
    const retry = args.retryAfter !== undefined && (job.pageAttempt ?? args.attempt) < 3;
    const pages = (job.pages ?? 0) + (args.error ? 0 : 1);
    const wantsMore = !args.error && job.kind === "bulk" && job.autoContinue && !!args.nextUntil;
    const stalled =
      wantsMore &&
      (!Number.isFinite(Date.parse(args.nextUntil!)) ||
        (job.until !== undefined && Date.parse(args.nextUntil!) >= Date.parse(job.until)));
    const pause = stalled
      ? "Paused because x.md did not return an older page. Your downloaded posts are safe."
      : undefined;
    const continueImport = wantsMore && !pause;
    await ctx.db.patch(job._id, {
      status:
        retry || continueImport
          ? "queued"
          : args.error
            ? job.count > 0
              ? "partial"
              : "failed"
            : "complete",
      error: args.error ?? pause,
      readyAt: retry
        ? Date.now() + Math.max(1000, args.retryAfter!)
        : continueImport
          ? Date.now() + 2000
          : undefined,
      warnings: args.warnings.slice(0, 10),
      nextUntil: stalled ? undefined : args.nextUntil,
      nextCursor: args.nextCursor,
      expectedUserId: args.expectedUserId ?? job.expectedUserId,
      pages,
      postsReceived: (job.postsReceived ?? 0) + (args.error ? 0 : (args.postsReceived ?? 0)),
      oldest: args.oldest ?? job.oldest,
      floorReached: args.floorReached ?? job.floorReached,
      ...(continueImport
        ? {
            until: args.nextUntil,
            pageAttempt: 0,
            phase: "Downloading older posts",
          }
        : {}),
      updatedAt: Date.now(),
    });
    if (continueImport)
      await ctx.scheduler.runAfter(2000, internal.importer.run, {
        jobId: job._id,
      });
    if (retry)
      await ctx.scheduler.runAfter(Math.max(1000, args.retryAfter!), internal.importer.run, {
        jobId: job._id,
      });
    if (args.profile) {
      const profile = args.profile;
      const account = await ctx.db
        .query("accounts")
        .withIndex("by_handle", (q) => q.eq("handle", profile.handle))
        .unique();
      if (account) await ctx.db.patch(account._id, profile);
      else await ctx.db.insert("accounts", profile);
    }
  },
});
