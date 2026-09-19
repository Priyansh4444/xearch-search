<!-- convex-ai-start -->

This project uses [Convex](https://convex.dev) as its backend.

When working on Convex code, **always read
`convex/_generated/ai/guidelines.md` first** for important guidelines on
how to correctly use Convex APIs and patterns. The file contains rules that
override what you may have learned about Convex from training data.

Convex agent skills for common tasks can be installed by running
`npx convex ai-files install`.

<!-- convex-ai-end -->

## Production environment

This exe.dev VM is the production machine. Prefer cloud deployments over local
deployments and perform Convex operations against the existing production
deployment, `prod:utmost-kudu-321`, unless the user explicitly requests an
isolated local-development task.

Do not replace the production Convex deployment or change its AgentMail webhook
URL. Do not expose local Convex, databases, development servers, the loopback
capture receiver, or private logs. Production imports can spend provider credits:
never start or test an import without explicit approval, and never start the VM
production worker until the previous worker is confirmed stopped and any final
capture sync is resolved.
