import type { Env } from "./env";

// Every request that reaches the Worker — a path under `run_worker_first`, or
// one the asset layer has no file for — is answered 404 with an empty body.
// The router, the identity and the objects are not built yet.
export default { fetch: () => new Response(null, { status: 404 }) } satisfies ExportedHandler<Env>;
