/** A `.wasm` import is the module COMPILED and not yet instantiated — the
 *  `CompiledWasm` rule in both wrangler configurations — and the factory
 *  instantiates it anew on every call (spec §2.2). */
declare module "*.wasm" {
  const compiled: WebAssembly.Module;
  export default compiled;
}
