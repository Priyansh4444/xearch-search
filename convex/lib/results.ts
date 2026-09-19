import { z } from "zod";
const safeLink = z
  .string()
  .url()
  .refine((value) => new URL(value).protocol === "https:", "Expected HTTPS");
export const resultPost = z.object({
  tweetId: z.string().regex(/^\d+$/),
  author: z.string().regex(/^[A-Za-z0-9_]{1,15}$/),
  text: z.string().max(6000),
  url: safeLink,
  createdAt: z.number().optional(),
  likes: z.number().nonnegative().optional(),
  reposts: z.number().nonnegative().optional(),
  replies: z.number().nonnegative().optional(),
  links: z.array(safeLink).max(10),
  avatar: safeLink.optional(),
  displayName: z.string().max(100).optional(),
});
export type ResultPost = z.infer<typeof resultPost>;
export const searchResponse = z.object({
  rows: z.array(resultPost).max(20),
  nextCursor: z.string().max(4000).optional(),
  warnings: z.array(z.string().max(500)).max(10).default([]),
});
