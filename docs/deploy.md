# 配信の手順（Cloudflare Workers Static Assets）

`main` への push で GitHub Actions がビルドして本番（https://rustruct.matsumok.com）に置き、PR を開くとプレビュー URL が付く（Issue #13）。
ここには「人が一度だけやること」と「困ったときに見るところ」を書く。秘密（トークンの値）は書かない。
何に決めたかは `docs/stack.md` の配信の節、なぜ静的アセットだけなのかは ADR-0002。

## 全体像

```
main に push ──→ GitHub Actions ──→ 検査（fmt … test）──→ dx build --ssg ──→ 煙テスト（check-ssg.sh）
                                     ──→ stage-deploy.sh（404/index.html → 404.html）
                                     ──→ wrangler deploy ──→ Worker「rustruct」の静的アセット ──→ rustruct.matsumok.com
                                     ──→ check-live.sh（本番 URL の煙テスト）
PR を開く／更新 ──→ 同じ検査・ビルド・煙テスト・stage-deploy.sh ──→ wrangler preview --name pr-<番号>
                                     ──→ https://pr-<番号>-rustruct.<サブドメイン>.workers.dev ──→ check-live.sh
                                     ──→ PR にコメント（1 つを書き換える）と deployment「preview」
PR を閉じる ──→ wrangler preview delete
```

- 設定は `wrangler.toml` 1 枚。Worker のスクリプト（JS）は無く、`assets` だけ。wrangler は CI が npm から落として使う道具で、リポジトリに Node の物は置かない
- 配信物は `sh scripts/check.sh stage`（`scripts/stage-deploy.sh`）が作る `target/deploy`。SSG の出力との違いは not-found の置き場だけ（下の「not-found」）
- CI の job は 検査→ビルド→煙テスト（`check`）、本番配置（`deploy`）、プレビュー配置（`preview`）、掃除（`cleanup`）に分かれ、どこで落ちたかが job と step の名前で分かる

## 一度だけ人がやること（Cloudflare 側）

順にやる。終わったら次へ。

1. **アカウント ID を控える**。ダッシュボードの Workers & Pages の右側（または任意のドメインの概要ページ右下）に「Account ID」がある。後で GitHub の Secret `CLOUDFLARE_ACCOUNT_ID` に入れる
2. **ゾーン `matsumok.com` が Cloudflare にあることを確かめる**（ネームサーバーが Cloudflare を向いている）。カスタムドメインは自分のゾーンにしか作れない
3. **`rustruct.matsumok.com` の既存の DNS レコードを消す**（あれば）。カスタムドメインは既存の CNAME の上には作れない。レコードは配置のときに Cloudflare が作る
4. **workers.dev のサブドメインを決める**。Workers & Pages → 概要 → 右側の「サブドメイン」。`<サブドメイン>.workers.dev` の `<サブドメイン>` がプレビュー URL の一部になる（本番は workers.dev に出さない）。2026-10-14 に `matsumok-digital` と決めた
5. **API トークンを作る**。プロフィール → API トークン → トークンを作成 → テンプレート「Cloudflare Workers を編集する（Edit Cloudflare Workers）」。中身は Workers Scripts（Account）、Workers Routes（Zone）など。範囲は「Account Resources = 自分のアカウント」「Zone Resources = Specific zone = matsumok.com」に絞る。値は一度しか表示されないので、次の節の `gh secret set` にその場で渡す

## GitHub に Secrets を入れる

値を引数に書かず標準入力から渡す（シェルの履歴に残さない）。

```sh
printf '%s' '<アカウント ID>' | gh secret set CLOUDFLARE_ACCOUNT_ID --repo matsumok/rustruct
printf '%s' '<API トークン>'   | gh secret set CLOUDFLARE_API_TOKEN --repo matsumok/rustruct
gh secret list --repo matsumok/rustruct
```

Secrets が無いと、`check` job は通り、`deploy` / `preview` job が wrangler の認証で落ちる。

## 初回の配置

`main` に push する。Actions の「配置 (本番)」が `wrangler deploy` を回し、

- Worker `rustruct` が無ければ作る
- `wrangler.toml` の `routes`（`custom_domain = true`）で `rustruct.matsumok.com` を Worker に結び、DNS レコードを Cloudflare が作る。証明書も自動
- 続く step「live」が `scripts/check-live.sh https://rustruct.matsumok.com` で、住所ごとの HTML、`/tools` → `/tools/` の転送、未知の住所の 404、CSS・WASM、WASM の Content-Type を見る
- 初回は DNS レコードと証明書の発行に数十秒かかる（2026-10-14 の初回配置で、直後の curl が全部接続できず「live」だけ落ちた）。`check-live.sh` は応答が返るまで最大 3 分待つ。それでも落ちたら、Actions の「Re-run failed jobs」で `deploy` job をやり直す

GitHub の Environments（`production`、`preview`）は最初の run が自動で作る。手で作る必要は無い。

## PR のプレビュー

- PR を開く（同じリポジトリのブランチから。fork には Secrets が渡らない）と、`preview` job が `wrangler preview --name pr-<番号>` でプレビューを作り、URL を PR のコメントに書く。push のたびに同じ URL の中身が新しくなり、コメントは 1 つを書き換える
- URL は `https://pr-<番号>-rustruct.matsumok-digital.workers.dev`。PR の「deployments」からも辿れる
- 本番は `workers_dev = false` で workers.dev に出さず、プレビューは `preview_urls = true` で workers.dev に出す。この組み合わせで本当にプレビュー URL が出るかは Cloudflare の文書に明記が無いので、最初の PR で確かめる（出なければ下の「困ったとき」）
- workers.dev のプレビューは `X-Robots-Tag: noindex` 付きで配られ、検索エンジンには載らない
- PR を閉じる（merge を含む）と `cleanup` job がプレビューを消す。無料枠は Worker ごと 100 プレビューまで
- **既知の制限**: プレビューでは未知の住所が本文の無い 404 になる（`wrangler preview` が `not_found_handling` を落とす。cloudflare/workers-sdk#16047、2026-10-14 時点で open）。`check-live.sh` は workers.dev に限りこれを注意に留める。not-found の見た目はプレビューでは確かめられないので本番で見る。直ったら `check-live.sh` のその分岐を消す

## 人の目で見ること

CI が見ないのは hydration と見た目。本番とプレビューの URL をスマホで開いて、

- 一覧の項目を押して全画面の再読み込みが起きない。「戻る」が効く
- OS をダークにして配色が変わる。和文が Plex Sans JP、パスと wordmark が Plex Mono
- 存在しない住所で not-found の骨組みが出て、起動後に住所の行が埋まる

`/root/` に Cloudflare Access を掛けたら（`docs/stack.md`）、`scripts/check-live.sh` の住所一覧から `/root/` を外す。掛けた瞬間に本番の「live」が落ちる。

headless Chrome で DOM の印を見るなら `sh scripts/hydrate-check.sh https://rustruct.matsumok.com / /tools/ /404/`（サンドボックスの外で）。結果は Issue #8 に記録する。

## not-found

SSG は not-found を `404/index.html` に書く（dioxus-server は住所の下に必ず `index.html` を置く。`docs/dioxus-0.7.md`）。Workers Static Assets が未知の住所に返すのは最寄りの `404.html`（`not_found_handling = "404-page"`）。`scripts/stage-deploy.sh` がルートの `404.html` に移し、`404/` は残さない。検査は `crates/rustruct-web/tests/deploy.rs`。SPA のフォールバック（`index.html` を返す）にはしない。プリレンダー済みの住所だけが存在する。

## 手元から

wrangler は手元に入れていない。配置は CI からだけ行う。どうしても手元から出すなら `npx wrangler@<版> deploy`（版は `.github/workflows/ci.yml` の `WRANGLER_VERSION`。Node が要る。`CLOUDFLARE_API_TOKEN` と `CLOUDFLARE_ACCOUNT_ID` を環境変数で）。恒常の道具にはしない（`docs/philosophy.md`: JS は自分で書かない）。

## 困ったとき

- **`deploy` が custom domain で落ちる**: トークンの Zone Resources に `matsumok.com` が無い、または `rustruct.matsumok.com` に既存の CNAME がある
- **`preview` が URL を返さない**: workers.dev のサブドメインが未設定か、Worker の workers.dev 側のプレビュー URL が無効。ダッシュボードの Worker → Settings → Domains & Routes で workers.dev の Preview URLs を有効にする。`wrangler.toml` の `preview_urls = true` はこの設定を配置のたびに書き込む（省略すると既存の設定のまま）
- **本番の未知の住所が 200 で `/` の中身になる**: `not_found_handling` が `single-page-application` になっているか、`404.html` が無い（`stage-deploy.sh` が走っていない）
- **WASM が読めない**: `check-live.sh` の Content-Type の行を見る。`application/wasm` でなければ拡張子か配信側の設定
- **wrangler の版を上げる**: `.github/workflows/ci.yml` の `WRANGLER_VERSION` 1 箇所。`preview` コマンドと `preview-url` 出力は 4.136.3 以上、wrangler-action は v4.1.3 以上（wrangler-action の action.yml に書かれている）
