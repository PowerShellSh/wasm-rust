declare module "*.wasm" {
  // Wrangler / bundler の挙動で ArrayBuffer になったり WebAssembly.Module になったりするため両対応にする
  const wasm: ArrayBuffer | WebAssembly.Module;
  export default wasm;
}


