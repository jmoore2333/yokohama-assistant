import type { Env } from "./env";

export { Table } from "./table";

// Every request that reaches the Worker — a path under `run_worker_first`, or
// one the asset layer has no file for — is answered 404 with an empty body.
// The router and the identity are not built yet, so no request reaches the
// table object through the Worker: only through its binding, as the pool's
// cells reach it.
export default { fetch: () => new Response(null, { status: 404 }) } satisfies ExportedHandler<Env>;
