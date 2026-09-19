import type { Doc } from "../convex/_generated/dataModel";
export function jobLabel(job: Doc<"jobs">) {
  if (job.status === "complete")
    return job.error
      ? "Paused"
      : job.nextUntil || job.nextCursor
        ? "More to download"
        : "Downloaded";
  return {
    queued: "Waiting",
    running: "Downloading",
    cancelled: "Stopped",
    partial: "Interrupted",
    failed: "Failed",
  }[job.status];
}
export function jobSummary(job: Doc<"jobs">) {
  if (job.kind === "bulk") {
    if (job.postsReceived !== undefined)
      return `${job.postsReceived.toLocaleString()} posts received${job.pages ? ` across ${job.pages} ${job.pages === 1 ? "batch" : "batches"}` : ""}`;
    return job.status === "complete"
      ? "This older import saved a batch of posts. Its post count wasn't tracked."
      : "Waiting for the next batch of posts";
  }
  return job.status === "complete" ? "Response saved" : (job.phase ?? "Waiting to start");
}
export function jobWarnings(job: Doc<"jobs">) {
  return job.warnings.filter(
    (w) =>
      !w.startsWith("Raw data handed off.") &&
      !w.startsWith("More history is available.") &&
      w !== "More posts are available from x.md.",
  );
}
