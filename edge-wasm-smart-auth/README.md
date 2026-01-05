# edge-wasm-smart-auth

「APIの前段で“落とせるものはすべて落とす”」を最小構成で実装する **Smart Auth Gateway** サンプルです。

- **Edge (Cloudflare Workers / TypeScript)**: HTTP / Header 抽出 / レスポンス制御
- **WASM (Rust)**: 検証ロジックのみ（HTTPを知らない）

## 構成

```text
edge-wasm-smart-auth/
├─ worker/
│  ├─ worker.ts        # Cloudflare Worker (Edge)
│  ├─ auth.wasm        # Rustで生成して配置（生成物）
│  └─ wrangler.toml
│
├─ wasm/
│  ├─ Cargo.toml
│  └─ src/
│     └─ lib.rs        # Rustの検証ロジック（WASM）
│
└─ README.md
```

## API仕様（最小）

### Request

```http
GET /protected
Authorization: Bearer edge-xxxxx
```

### Response

| 条件 | Status | Body |
|---|---:|---|
| トークンOK | 200 | OK |
| トークンNG | 401 | Unauthorized |

## 重要: 責務分離

- **WASMは HTTP を知らない**（bytes を受けて `i32` を返すだけ）
- **Edgeは 検証の中身を知らない**（WASM関数を呼んで結果に応じて返すだけ）

## WASMでJWTを「検証だけ」する設計（Qiita向け）

### ❌ WASMにやらせないこと

- HTTPヘッダ取得
- Cookie処理
- `Authorization: Bearer ...` のパース
- リクエスト拒否判断（ステータス/レスポンス設計）

### ✅ WASMにやらせること

- JWT文字列が **正しいか**
- **署名検証（HS256）**
- **`exp / aud / iss` の検証**
- 結果を **数値(i32)** で返す（Edgeが制御を握るため）

### Edge / WASM の責務

- **Edge**: token抽出 → WASMにbytes渡す → 結果で制御
- **WASM**: JWT検証ロジックのみ（純粋関数）

### なぜ境界が崩れないのか

- **WASMは状態を持たない**: DB/セッション/キャッシュ無し
- **WASMはI/Oをしない**: HTTPもファイルも触れない
- **Edgeが制御を握る**: WASMは判断材料(i32)を返すだけ

## Rust(WASM) ビルド手順（Rustが入っている環境で実行）

```bash
rustup target add wasm32-unknown-unknown
cd wasm
cargo build --release --target wasm32-unknown-unknown
```

生成物:

```text
wasm/target/wasm32-unknown-unknown/release/auth_wasm.wasm
```

これを以下へコピー:

```text
worker/auth.wasm
```

## Worker 起動（Wrangler）

```bash
cd worker
wrangler dev
```

### この環境（Windows / PowerShell優先）での実行方法

コマンドは先頭に `C:\cygwin64\bin\bash.exe` を付けて実行してください。

```bash
C:\cygwin64\bin\bash.exe -lc "cd edge-wasm-smart-auth/worker && wrangler dev"
```

## 注意: `worker/auth.wasm` が無い場合

`worker/worker.ts` は `./auth.wasm` を `fetch()` してロードします。  
Rustでビルドした `auth_wasm.wasm` を **`worker/auth.wasm`** にコピーしてから起動してください。

## 動作確認

```bash
curl -i -H "Authorization: Bearer edge-123" http://127.0.0.1:8787/protected
# → 200 OK

curl -i -H "Authorization: Bearer invalid" http://127.0.0.1:8787/protected
# → 401 Unauthorized
```

WASMは「判断材料」を返すだけにして、Edge側で分岐させます。

- `1` : OK
- `0` : Invalid
- `-1`: Expired


