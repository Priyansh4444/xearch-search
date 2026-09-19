import { ConvexError } from "convex/values";

/** Read a Convex mutation/action error the way the app wants to show it.
 * ConvexError carries the intended message in `.data`; anything else is a
 * plain thrown Error, whose `.message` Convex wraps as
 * "[CONVEX M(fn)] [Request ID: ...] Server Error ... Uncaught Error: <text>"
 * to avoid leaking internals. Strip that wrapper down to the original text. */
export const describeError = (e: unknown) =>
  e instanceof ConvexError
    ? String(e.data)
    : e instanceof Error
      ? e.message.replace(/\[CONVEX[^]*?Uncaught (?:Error|ConvexError):\s*/, "").split("\n")[0]
      : "Something went wrong. Try again.";
