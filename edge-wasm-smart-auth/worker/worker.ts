/// <reference path="./wasm.d.ts" />
import wasm from "./auth.wasm";

type AuthExports = {
  memory: WebAssembly.Memory;
  alloc: (len: number) => number;
  dealloc: (ptr: number, len: number) => void;
  validate_token: (ptr: number, len: number) => number;
};

let cached: Promise<AuthExports> | null = null;

async function getAuthExports(): Promise<AuthExports> {
  if (!cached) {
    cached = (async () => {
      // wasm が ArrayBuffer の場合: WebAssemblyInstantiatedSource が返る
      // wasm が WebAssembly.Module の場合: WebAssembly.Instance が返る
      const instantiated = await WebAssembly.instantiate(wasm, {});
      const instance =
        "instance" in (instantiated as any)
          ? ((instantiated as unknown) as WebAssembly.WebAssemblyInstantiatedSource).instance
          : (instantiated as WebAssembly.Instance);
      return instance.exports as unknown as AuthExports;
    })();
  }
  return cached;
}

function extractBearerToken(req: Request): string {
  const auth = req.headers.get("authorization") ?? "";
  // 最小: "Bearer " 前提（Qiita向けにシンプル）
  if (auth.startsWith("Bearer ")) return auth.slice("Bearer ".length);
  return "";
}

export default {
  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    if (url.pathname !== "/protected") {
      return new Response("Not Found", { status: 404 });
    }

    const token = extractBearerToken(request);
    if (!token) {
      return new Response("Unauthorized", { status: 401 });
    }

    const { memory, alloc, dealloc, validate_token } = await getAuthExports();

    const data = new TextEncoder().encode(token);
    const ptr = alloc(data.length);
    try {
      new Uint8Array(memory.buffer, ptr, data.length).set(data);
      const result = validate_token(ptr, data.length);
      switch (result) {
        case 1:
          return new Response("OK", { status: 200 });
        case -1:
          // Qiita映え用: 期限切れだけ別扱い（refreshの導線などに使える）
          return new Response("Token Expired", { status: 401 });
        default:
          return new Response("Unauthorized", { status: 401 });
      }
    } finally {
      dealloc(ptr, data.length);
    }
  },
};


