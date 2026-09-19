import { ConvexHttpClient } from "convex/browser";
const client = new ConvexHttpClient(
  process.argv.includes("--production")
    ? "https://utmost-kudu-321.convex.cloud"
    : "http://127.0.0.1:3210",
);
const auth = await client.action("auth:signIn", { provider: "anonymous" });
if (!auth.tokens?.token) throw new Error("Local guest authentication did not return a token.");
client.setAuth(auth.tokens.token);
const jobId = await client.mutation("jobs:start", {
  kind: "profile",
  input: "theo",
});
console.log("Started a single profile-read smoke test.");
let last;
for (let i = 0; i < 70; i++) {
  const job = (await client.query("jobs:list", {})).find((job) => job._id === jobId);
  const state = `${job.status}: ${job.phase ?? "waiting"}`;
  if (state !== last) console.log(state);
  last = state;
  if (!["queued", "running"].includes(job.status)) {
    if (job.status !== "complete") throw new Error(job.error ?? "Capture did not complete");
    const receipts = await client.query("jobs:receipts", { jobId });
    console.log(
      `Verified ${receipts.length} durable receipt(s). Smoke job belongs to a separate test guest session.`,
    );
    process.exit(0);
  }
  await new Promise((resolve) => setTimeout(resolve, 2000));
}
throw new Error("Smoke job did not reach completion within the test window.");
