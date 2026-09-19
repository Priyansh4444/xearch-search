import { mutation, query } from "./_generated/server";
import { components } from "./_generated/api";
import { AgentMail, type OutboundId } from "@agentmail/convex";
import { v, ConvexError } from "convex/values";
import { user, budget } from "./access";
const mail = new AgentMail(components.agentmail);
export const send = mutation({
  args: { sessionId: v.id("sessions"), recipient: v.string() },
  handler: async (ctx, args): Promise<void> => {
    const owner = await user(ctx);
    if (process.env.REQUIRE_VERIFIED_EMAIL === "true") {
      const account = await ctx.db.get(owner);
      if (
        !account?.emailVerificationTime ||
        account.email?.toLowerCase() !== args.recipient.trim().toLowerCase()
      )
        throw new ConvexError("Email sending requires sign-in with a verified email address.");
    }
    if (!process.env.AGENTMAIL_API_KEY || !process.env.AGENTMAIL_INBOX_ID)
      throw new ConvexError("Configure AgentMail on the backend to send results.");
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(args.recipient) || args.recipient.length > 254)
      throw new ConvexError("Enter a valid email address.");
    const result = await ctx.db.get(args.sessionId);
    if (!result || result.owner !== owner || result.status !== "complete" || !result.rows.length)
      throw new ConvexError("There are no completed search results to send.");
    await budget(ctx, "email:global", 20);
    await budget(ctx, `email:${owner}`, 3);
    const text =
      `Xearch results for: ${result.raw}\n\nFirst ${Math.min(10, result.rows.length)} results on this page.\n${result.warnings.join("\n")}\n\n` +
      result.rows
        .slice(0, 10)
        .map((p) => `@${p.author}\n${p.text.slice(0, 1500)}\n${p.url}`)
        .join("\n\n---\n\n");
    const outboundId = await mail.sendMessage(ctx, process.env.AGENTMAIL_INBOX_ID, {
      to: args.recipient,
      subject: `Xearch: ${result.raw.replace(/[\r\n]/g, " ").slice(0, 100)}`,
      text,
    });
    await ctx.db.insert("deliveries", { owner, outboundId, query: result.raw });
  },
});
export const deliveries = query({
  args: {},
  handler: async (ctx) => {
    const owner = await user(ctx);
    const rows = await ctx.db
      .query("deliveries")
      .withIndex("by_owner", (q) => q.eq("owner", owner))
      .order("desc")
      .take(5);
    return Promise.all(
      rows.map(async (row) => ({
        ...row,
        delivery: await mail.status(ctx, row.outboundId as OutboundId),
      })),
    );
  },
});
