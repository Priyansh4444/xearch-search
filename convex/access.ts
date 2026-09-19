import { getAuthUserId } from "@convex-dev/auth/server";
import { ConvexError } from "convex/values";
import type { QueryCtx, MutationCtx } from "./_generated/server";
export async function user(ctx: QueryCtx | MutationCtx) {
  const id = await getAuthUserId(ctx);
  if (!id) throw new ConvexError("Start a session to use your workspace.");
  return id;
}
export async function budget(ctx: MutationCtx, key: string, maximum: number) {
  await budgets(ctx, [{ key, maximum }]);
}
export async function budgets(
  ctx: MutationCtx,
  limits: ReadonlyArray<{ key: string; maximum: number }>,
) {
  const day = new Date().toISOString().slice(0, 10);
  const entries = await Promise.all(
    limits.map(async ({ key, maximum }) => {
      const id = `${day}:${key}`;
      const row = await ctx.db
        .query("budgets")
        .withIndex("by_key", (q) => q.eq("key", id))
        .unique();
      return { id, maximum, row };
    }),
  );
  if (entries.some(({ maximum, row }) => (row?.count ?? 0) >= maximum))
    throw new ConvexError("Today's usage limit is reached. Try again tomorrow.");
  for (const { id, row } of entries) {
    if (row) await ctx.db.patch(row._id, { count: row.count + 1 });
    else await ctx.db.insert("budgets", { key: id, count: 1 });
  }
}
