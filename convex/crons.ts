import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";
const crons = cronJobs();
crons.interval(
  "expire transient UI snapshots",
  { minutes: 10 },
  internal.cleanup.transient,
  {},
);
export default crons;
