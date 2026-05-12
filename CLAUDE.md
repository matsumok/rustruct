# Rustruct

日本の構造設計者向け構造計算Webアプリケーション。断面計算・地震応答解析などをブラウザ上で実行できる。

## プロジェクト概要

- **対象ユーザー**：構造設計者（メール認証制・約30人規模）
- **公開方針**：計算機能は未ログインでも使用可。保存機能はログイン必須
- **運用方針**：Cloudflare無料枠内で完結。セルフホスト不要

-----

## 技術スタック

|レイヤー   |技術                                                             |
|-------|---------------------------------------------------------------|
|UI     |React + TypeScript + Vite + shadcn/ui                          |
|ルーティング |TanStack Router                                                |
|APIフック |orval（`@hono/zod-openapi` からOpenAPI生成）                         |
|計算ロジック |Rust → Wasm（`wasm-bindgen` + `tsify-next`）                     |
|マスターデータ|CSVをビルド時に`serde_json`でJSON化し静的アセットとして配信。TanStack Queryで取得・キャッシュ|
|グラフ    |Recharts（メイン）・visx（断面力図）                                       |
|数式表示   |KaTeX                                                          |
|PDF出力  |react-to-print                                                 |
|バックエンド |Hono（TypeScript）                                               |
|認証     |Better Auth（メール＋パスワード・admin プラグイン）                             |
|DB     |Cloudflare D1                                                  |
|ストレージ  |Cloudflare R2                                                  |
|メール送信  |Resend（月3,000通無料）                                              |
|デプロイ   |Cloudflare Workers                                             |
|ORM    |Drizzle ORM（D1 + マイグレーション管理）                                   |
|リンター   |Biome（ESLint + Prettier の代替）                                   |
|パッケージ管理|pnpm                                                           |
|テーブルUI |react-datasheet-grid（質点モデル入力）・TanStack Table（一覧・選択）            |

-----

## リポジトリ構成（モノレポ）

```
rustruct/
├── package.json            ← pnpmワークスペース定義
├── pnpm-workspace.yaml
├── biome.json              ← Biome設定（ルートに1つ）
├── engine/                 ← Rust（計算エンジン）
│   ├── Cargo.toml          ← Rustワークスペース定義（[workspace]のみ）
│   ├── rustruct-types/
│   ├── rustruct-calc/
│   └── rustruct-wasm/
│       ├── build.rs        ← CSV→JSON変換（マスターデータ。wasm-pack build時に実行）
│       └── data/           ← マスターデータCSV（steel_h.csv等）
├── app/                    ← React + Vite
│   └── public/
│       └── master/         ← ビルド時に生成されるJSONアセット（gitignore）
└── edge/                   ← Hono on Cloudflare Workers
```

パッケージマネージャーは `pnpm`。Turboは不要（`concurrently` で十分）。

**pnpm-workspace.yaml**

```yaml
packages:
  - app
  - edge
```

**Cargo.toml（ルート）**

```toml
[workspace]
members = [
    "rustruct-types",
    "rustruct-calc",
    "rustruct-wasm",
]
# engine/Cargo.toml に配置。ルートからは --manifest-path engine/Cargo.toml で参照
resolver = "2"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
nalgebra = "0.33"
wasm-bindgen = "0.2"
```

-----

## Rustクレート構成

### 依存関係（一方向のみ）

```
rustruct-wasm
  └── rustruct-calc
        └── rustruct-types
```

### 各クレートの役割

**rustruct-types**

- 共通の型定義（断面型・荷重型・地震波型など）
- マスターデータの型定義のみ（`SteelSectionH` 等）。データ本体は持たない
- `tsify-next` + `wasm-bindgen` は `wasm` featureで分離（後述）

**rustruct-calc**

- 純粋な計算ロジックのみ
- `wasm-bindgen` 依存なし（`cargo test` でそのままテスト可能）
- 断面計算・地震応答解析（Nigam-Jennings法）・応答スペクトル・固有値解析・モーダルアナリシス・FFT（`rustfft`）・Newmark-β法（40質点時刻歴解析）
- rayon不使用（Wasm非対応のためシングルスレッド）。並列化はJS側のWeb Workersで行う

**rustruct-wasm**

- 薄いWasmバインディング層。`src/` を以下のモジュールに分割する：
  - `calc.rs` … `rustruct-calc` の関数を `#[wasm_bindgen]` でラップするだけ
- マスターデータは持たない（JSON静的アセットとして別途配信）
- `crate-type = ["cdylib"]`

### コード方針

```rust
// rustruct-calc（純粋関数・副作用なし・データ参照なし）
pub fn calc_bending_stress(section: &SteelSectionH, moment: f64) -> f64 { ... }
pub fn run_analysis(input: &SeismicInput) -> SeismicResult { ... }

// rustruct-wasm/src/calc.rs（薄いラッパー）
#[wasm_bindgen]
pub fn run_seismic_analysis(input: SeismicInput) -> SeismicResult {
    rustruct_calc::seismic::run_analysis(&input)
}
```

### wasm feature による依存分離

`tsify-next` と `wasm-bindgen` は `rustruct-types` の `wasm` featureに閉じ込め、`rustruct-calc` のネイティブビルド・テストに影響しないようにする。

**`engine/rustruct-types/Cargo.toml`**

```toml
[dependencies]
serde = { workspace = true }
tsify-next = { version = "...", optional = true }
wasm-bindgen = { workspace = true, optional = true }

[features]
wasm = ["tsify-next", "wasm-bindgen"]
```

**`engine/rustruct-calc/Cargo.toml`**

```toml
[dependencies]
rustruct-types = { path = "../rustruct-types" }
nalgebra = { workspace = true }
# wasm featureを有効化しない → wasm-bindgen依存なし
```

**`engine/rustruct-wasm/Cargo.toml`**

```toml
[dependencies]
rustruct-types = { path = "../rustruct-types", features = ["wasm"] }
rustruct-calc = { path = "../rustruct-calc" }
wasm-bindgen = { workspace = true }
```

**featureによるビルド分岐まとめ**

|コマンド                                            |wasm feature|用途               |
|------------------------------------------------|------------|-----------------|
|`cargo test -p rustruct-types`                  |❌           |型定義のネイティブテスト     |
|`cargo test -p rustruct-calc`                   |❌           |計算ロジックのネイティブテスト  |
|`wasm-pack build engine/rustruct-wasm`          |✅           |Wasmビルド・`.d.ts`生成|

### ビルドコマンド

```bash
wasm-pack build engine/rustruct-wasm --target bundler
cargo test --manifest-path engine/Cargo.toml -p rustruct-calc
```

-----

## 型共有（Rust → TypeScript）

`tsify-next` でRustの型定義からTypeScriptの型を自動生成する。`wasm-pack build` で `.d.ts` が出力され、React側で型安全にWasm関数を呼べる。

```rust
#[derive(Tsify, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct SeismicInput {
    pub mass: Vec<f64>,
    pub stiffness: Vec<f64>,
    pub height: Vec<f64>,
    pub damping: f64,
    pub wave: Vec<f64>,
}
```

-----

## 数値精度方針

- 全データ・計算を **`f64` で統一**
- f32に落とすパフォーマンス上のメリットはWASM環境ではほぼなし（SIMDが使えないため）
- Nigam-Jennings法の遷移行列はexp/sin/cosを含むため特にf64が必要
- メモリ使用量は安全圏（ブラウザ500MBの0.06%程度）

-----

## 地震波データ設計

### 波形の種別

```rust
pub struct Waveform {
    id: String,
    name: String,
    dt: f64,
    samples: Vec<f64>,
    kind: WaveformKind,
    pgv: Option<f64>,   // 既往波のみSome（cm/s）
}

pub enum WaveformKind {
    Observed,   // 既往波
    Code,       // 告示波
}
```

- フィールドはすべて非pub、getterで公開
- `new()` コンストラクタでバリデーション・初期化（外部からの直接フィールド操作不可）

### スケーリング設計

- R2には**Originalの波形バイナリのみ**保存
- L1/L2基準化波は保存しない。使用時に `scale = target_pgv / original_pgv` でリアルタイムスケーリング
- 告示波（Code）はPGVの概念がないためそのまま1レコード保存

```
L1基準: PGV 25 cm/s
L2基準: PGV 50 cm/s
scale = target_pgv / original_pgv
```

### 登録フロー

**既往波:**

- ユーザーがTXTをアップロード
- PGVはコメント行から取得（`# pgv=35.2`）
- Originalのみ登録。L1/L2は使用時にスケーリング

**告示波:**

- ユーザーがTXTをアップロード
- 1波登録

### TXTフォーマット

```
# pgv=35.2   ← コメント行にPGVを記載（既往波のみ必須）
0.001234
-0.002341
...
```

### ライセンス注意

建築性能基準推進協会の1994年観測波は**二次頒布禁止**のためアプリへの同梱不可。ユーザーが自分でダウンロード・アップロードする運用とする。

### テキストパーサ

設定ベースの汎用パーサを使用：

```rust
pub struct WaveParseConfig {
    pub skip_lines: usize,        // スキップするヘッダ行数
    pub comment_prefix: Option<String>,
    pub value_column: usize,      // 加速度値が何列目か（0-indexed）
    pub delimiter: Delimiter,     // Whitespace / Comma / Tab
    pub scale: f64,               // スケールファクタ
}
```

### バイナリ保存・参照フロー

```
ブラウザ → GET /api/waveforms/:id/binary
→ Workers: D1からr2_keyを取得
→ Workers: R2.get(r2_key)
→ Workers: バイナリをレスポンスとして返す
→ ブラウザ: Float64Arrayに変換
```

ブラウザはR2に直接アクセス不可 → **必ずWorkers経由**

### Rust ↔ TypeScript のバイナリ対応

|Rust                         |TypeScript                |
|-----------------------------|--------------------------|
|`bytemuck::cast_slice::<f64>`|`new Float64Array(buffer)`|
|`bytemuck::cast_slice::<f32>`|`new Float32Array(buffer)`|

エンディアンは現代環境（x86, ARM, WASM）すべてリトルエンディアンで一致するため実用上問題なし。

### 対象波形（建築性能基準推進協会 1994年版）

|波形            |継続時間|サンプリング|ステップ数(dt=0.02s)|
|--------------|----|-------|--------------|
|ElCentro 1940 |約54s|dt=0.02s|2,700        |
|Taft 1952     |約54s|dt=0.02s|2,700        |
|Hachinohe 1968|約51s|dt=0.02s|2,550        |
|Tohoku 1978   |約41s|dt=0.02s|2,050        |

応答スペクトル計算時は線形補間により `dt=0.01s` にアップサンプリングして使用。

-----

## 応答スペクトル計算

|項目  |設定                                          |
|----|---------------------------------------------|
|計算手法|Nigam-Jennings法                              |
|周期範囲|0.1〜10s 対数等間隔250点                           |
|dt  |0.01s（観測波は元データdt=0.02sを線形補間。告示波は元データdt=0.01s）|
|減衰  |複数指定可能                                      |
|推定時間|~3.8s（WASM・シングルスレッド）                         |
|並列化 |JS側のWeb Workersで実施。Rust/Wasmはシングルスレッド        |
|描画  |Web Workers併用、1秒ごとに部分更新                      |

### 表示

- 3パネル縦並び（Sv / Sa / Sd）
- 横軸: 周期 対数スケール（共通・連動）
- ホバー連動
- 複数波重ね描き

-----

## 40質点応答解析

|項目  |設定              |
|----|----------------|
|計算手法|Newmark-β法（線形）  |
|実装場所|`rustruct-calc`  |
|推定時間|400ms〜1.5s（WASM）|
|描画  |計算完了後に一括描画      |

### 入力パラメータ

- 各層: 質量(t) / 剛性(kN/m) / 階高(m)
- 全層共通: 減衰定数h（将来的にレイリー減衰も対応予定）

### 出力

- 1波ごとに最大層間変形角をプロット
- 複数波を重ね描き（縦軸: 層、横軸: 最大層間変形角）

-----

## 固有値解析・Ai分布比較

### 実装スコープ

**Step1: 固有値解析**（nalgebra、WASM対応済）

```
出力: 固有周期T_i、固有ベクトルφ_i
```

**Step2: モーダルアナリシス**

```
出力: 刺激関数Γ_i、有効質量比、各モードの層せん断力・変位
```

### Ai分布比較

3種類を同一グラフに重ね描き：

|種類     |算出方法                |
|-------|--------------------|
|告示Ai   |簡易式                 |
|固有値解析Ai|固有値解析ベース精算          |
|時刻歴解析Ai|各波の層せん断力から算出（波ごとに1本）|

包絡表示は将来対応。

-----

## 質点モデル入力UI

```
[質点数入力] → [Dialogを開く]

Dialog内（shadcn/ui <Dialog>）:
  [減衰定数入力]  [昇順/降順切り替え]

  react-datasheet-grid（MIT・React 19対応済）
  層 | 質量(t) | 剛性(kN/m) | 階高(m)
  Excelからコピペ・部分ペースト対応

  [キャンセル] [保存]
```

- 内部データは常に1階=index[0]で保持
- 表示時だけ昇順/降順に切り替え

-----

## Web Workers設計

```
ユーザーが計算ボタンを押す
→ Workerに波形データを送る
→ Worker内のWASMが計算（シングルスレッド）
→ 部分結果をpostMessage
  （応答スペクトル: 1秒ごと、応答解析: 完了後）
→ Reactのstateに追記
→ Rechartsが再描画
```

並列化はWeb Workersの複数インスタンスで実現。Rust/Wasm側はrayonを使用しない。

-----

## データ管理

### アクセス制御

```
未ログイン   → 計算できる（保存不可）
ログイン済み → プロジェクト・計算入力・結果を保存できる
```

### データ保存先

|データ         |保存先   |備考                        |
|------------|------|--------------------------|
|ユーザー情報      |D1    |Better Auth が管理            |
|セッション       |D1    |Better Auth が管理            |
|プロジェクト      |D1    |                          |
|計算ケース（メタデータ）|D1    |tool_type・r2_key等          |
|計算入力・結果     |R2    |JSONでまとめて保存               |
|観測波形メタデータ   |D1    |                          |
|観測波形本体      |R2    |float64バイナリ（Originalのみ）    |
|マスターデータ     |静的アセット|全件メモリ展開・JSでフィルタ           |

### D1スキーマ

Better Auth が管理するテーブル（CLIで自動生成。直接編集しない）：

```sql
-- Better Auth コアテーブル
user         (id, name, email, email_verified, image, created_at, updated_at,
              role, banned, ban_reason, ban_expires)
              -- role: 'user' | 'admin'（admin プラグインが追加）
              -- banned 〜 ban_expires: admin プラグインが追加
session      (id, user_id, token, expires_at, ip_address, user_agent,
              created_at, updated_at,
              impersonated_by)
              -- impersonated_by: admin プラグインが追加
account      (id, user_id, account_id, provider_id,
              access_token, refresh_token, ...)
verification (id, identifier, value, expires_at, created_at, updated_at)
```

アプリが管理するテーブル：

```sql
projects        (id, user_id, name, created_at)

-- 解析ケース（ツール単位の括り）
analysis_cases  (id, project_id, name, tool_type, comment, created_at)
                -- tool_type: 'response_spectrum' | 'time_history' | 'section' など

-- 解析セット（減衰 × 波形 の1組。ケースに紐づく）
analysis_sets   (id, analysis_case_id, waveform_id, damping,
                 status, r2_key, calculated_at, created_at)
                -- status: 'pending' | 'done' | 'error'
                -- r2_key: 計算完了時のみセット。未計算はNULL
                -- waveform_id・damping: 断面計算など波形不要のツールはNULL

waveforms       (id, user_id, name, kind, dt, duration, pgv, r2_key, created_at)
                -- kind: 'observed' | 'code'
                -- pgv: 既往波のみ（cm/s）。告示波はNULL
```

### R2のデータ構造

```
waveforms/{user_id}/{waveform_id}.bin                        ← float64バイナリ（Originalのみ）
analysis_results/{analysis_case_id}/{analysis_set_id}.json   ← セットごとの計算結果
```

#### ツール種別ごとの結果JSONスキーマ

**response_spectrum（応答スペクトル）**

```json
{
  "waveform_id": "abc123",
  "damping": 0.05,
  "periods": [0.1, 0.105, "...250点"],
  "Sv": [...],
  "Sa": [...],
  "Sd": [...]
}
```

**time_history（40質点時刻歴解析）**

```json
{
  "waveform_id": "abc123",
  "damping": 0.05,
  "floors": [1, 2, "...40"],
  "max_drift_angle": [...],
  "max_shear_force": [...],
  "time": [0.0, 0.02, "..."],
  "displacement": [[...], [...]]
}
```

**section（断面計算）**

```json
{
  "tool": "steel_h_bending",
  "input": { "...": "断面・荷重パラメータ" },
  "output": { "...": "応力・検定比など" }
}
```

断面計算は波形・減衰が不要なため `analysis_sets` の `waveform_id` / `damping` は NULL。結果は1セットのみ。

### Cloudflareストレージ無料枠（使用量見積もり）

|サービス|用途     |無料枠 |推定使用量      |
|----|-------|----|-----------|
|D1  |メタデータ・セッション全般|5GB |~35MB（7%）  |
|R2  |波形・計算結果|10GB|~50MB（0.5%）|

KVは使用しない。セッションはD1の `session` テーブルで管理する（Better Auth に委譲）。

-----

## 認証・ユーザー管理

認証は **Better Auth** に全面委譲する。自前のパスワードハッシュ・セッション管理は行わない。

### 登録フロー

```
1. ユーザーが登録フォームに入力（メールアドレス・パスワード）
2. Better Auth がアカウントを作成（メール未認証状態）
3. Resend でメール認証リンクを送信
4. ユーザーがリンクをクリック → メール認証完了
5. ログイン可能になる
```

- メール認証が完了するまでログイン不可（スパム対策）
- パスワードハッシュは Better Auth が内部で処理（自前実装なし）
- セッションは D1 の `session` テーブルで管理（TTL は Better Auth のデフォルト: 7日）

### Better Auth 設定概要

```typescript
// edge/src/auth/index.ts
import { betterAuth } from 'better-auth'
import { drizzleAdapter } from 'better-auth/adapters/drizzle'
import { admin } from 'better-auth/plugins'

export const auth = (env: Env) => betterAuth({
  database: drizzleAdapter(db, { provider: 'sqlite' }),
  emailAndPassword: {
    enabled: true,
    requireEmailVerification: true,
    // requireEmailVerification: true により未認証ユーザーはログイン不可
    // requireEmailVerification と admin() を併用する場合は customSyntheticUser が必要
    customSyntheticUser: ({ coreFields, additionalFields, id }) => ({
      ...coreFields,
      // admin プラグインが追加するフィールドをスキーマ順に明示する
      role: 'user',
      banned: false,
      banReason: null,
      banExpires: null,
      ...additionalFields,
      id,
    }),
  },
  emailVerification: {
    sendVerificationEmail: async ({ user, url }) => {
      // Resend でメール送信
    },
  },
  plugins: [admin()],
})
```

### スキーマ生成

Better Auth CLI でスキーマを生成し、Drizzle でマイグレーションする：

```bash
npx auth@latest generate   # edge/src/db/auth.schema.ts を生成
pnpm --filter edge drizzle-kit migrate
```

`auth.schema.ts` は **git管理する**。プラグイン追加・変更時のみ `generate` を再実行し、差分をコミットする。自動生成ファイルだが gitignore には含めない（マイグレーション履歴の一部として扱う）。

### 管理画面ルート

```
/admin/users  ← ユーザー一覧・ロール変更・BAN（Better Auth admin プラグイン経由）
```

管理者は `user.role = 'admin'` で識別する。初期管理者は D1 を直接操作して設定する。

### Resend の用途

メール認証リンクの送信のみ。管理者通知・招待メールは廃止。

-----

## デプロイ・ドメイン

- **URL**：`rustruct.matsumok.com`
- **デプロイ**：`wrangler deploy` で反映

```json
// wrangler.jsonc
{
  "routes": [
    {
      "pattern": "rustruct.matsumok.com/*",
      "zone_name": "matsumok.com"
    }
  ]
}
```

### 環境変数

| 変数 | ローカル | 本番 |
|---|---|---|
| `BETTER_AUTH_URL` | `http://localhost:8787`（`.dev.vars`） | `https://rustruct.matsumok.com`（`wrangler.jsonc` vars） |
| `BETTER_AUTH_SECRET` | `.dev.vars` | `wrangler secret put` |
| `RESEND_API_KEY` | `.dev.vars` | `wrangler secret put` |

**本番デプロイ手順：**

```bash
wrangler secret put BETTER_AUTH_SECRET   # 対話的に入力
wrangler secret put RESEND_API_KEY
pnpm --filter edge db:migrate:remote
pnpm --filter edge deploy
```

> `wrangler secret put` で登録した値はCloudflare側から後から確認できない。必ずパスワードマネージャーに控えておくこと。

### 型定義の更新

`wrangler.jsonc` の `vars` を変更したら型を再生成すること：

```bash
pnpm --filter edge cf-typegen
```

### ローカル DB リセット

```bash
rm edge/.wrangler/state/v3/d1/miniflare-D1DatabaseObject/*.sqlite*
pnpm --filter edge db:migrate:local
```

-----

## マスターデータ

```
engine/rustruct-wasm/data/
  ├── steel_h.csv
  ├── steel_c.csv
  └── bolt.csv
        ↓ engine/rustruct-wasm/build.rs（CSV → JSON。wasm-pack build時に実行）
app/public/master/
  ├── steel_h.json
  ├── steel_c.json
  └── bolt.json
        ↓ 静的アセット配信
JS側: TanStack Query（staleTime: Infinity・gcTime: Infinity）
```

-----

## ローカル開発環境

### 起動コマンド

```bash
pnpm dev        # app + edge を同時起動
pnpm dev:wasm   # Wasmウォッチ（別ターミナルで実行）
```

### package.json（ルート）

```json
{
  "scripts": {
    "dev:app": "pnpm --filter app dev",
    "dev:edge": "pnpm --filter edge wrangler dev",
    "dev": "concurrently \"pnpm dev:app\" \"pnpm dev:edge\"",
    "dev:wasm": "cargo watch -s 'wasm-pack build engine/rustruct-wasm --target bundler'",
    "lint": "biome lint .",
    "format": "biome format --write .",
    "check": "biome check --write ."
  }
}
```

-----

## 開発ツール

### リンター・フォーマッター：Biome

```bash
pnpm add -D --save-exact @biomejs/biome
pnpm dlx @biomejs/biome init
```

### ORM：Drizzle ORM

```typescript
// edge/src/db/schema.ts
export const analysisCases = sqliteTable('analysis_cases', {
  id: text('id').primaryKey(),
  projectId: text('project_id').notNull(),
  name: text('name').notNull(),
  toolType: text('tool_type').notNull(),
  comment: text('comment'),
  createdAt: integer('created_at', { mode: 'timestamp' }).notNull(),
})
```

Better Auth が生成する `auth.schema.ts` とアプリのスキーマは分離して管理し、マイグレーション時にマージする：

```typescript
// edge/src/db/index.ts
import * as authSchema from './auth.schema'   // Better Auth CLI 生成
import * as appSchema from './schema'          // アプリ側

export const schema = { ...authSchema, ...appSchema }
```

### スタイルバリアント管理：Tailwind Variants（未導入・要検討）

自作コンポーネントが増えてきたタイミングで導入する。
