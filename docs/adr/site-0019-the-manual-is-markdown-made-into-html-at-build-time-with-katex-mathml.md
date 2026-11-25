# 説明書の本文は Markdown をビルドのときに HTML にし、式は KaTeX の MathML だけで組む

Tool の計算の中身を文と式で説明する説明書（`GLOSSARY.md`）の本文の書き方と組み方（2026-11-25、#148）。Note も後で同じ仕組みで書く。今の規則は `docs/design/README.md` の「説明書の本文」、置き場と道具は `docs/stack.md`。

## 決めたこと

- **本文は Markdown のファイルで、住所と同じ形に置く。** `crates/rustruct-web/content/<住所>.md` が住所 `/<住所>` の本文（`/tools/kirchhoff-plate/manual` は `content/tools/kirchhoff-plate/manual.md`）。Note も `content/notes/<名前>.md` に置ける。ファイルと住所の対応は別に書かず、`build.rs` がフォルダを歩いて導く。ルーターに無い住所のファイルはテストが落とす
- **変換はビルドのとき。** `build.rs` が pulldown-cmark で Markdown を読み、式を KaTeX（`katex` クレート。KaTeX の JS を QuickJS で動かす）で組み、HTML の文字列を `$OUT_DIR` に書く。ページはその文字列を置くだけで、ブラウザでは Markdown も KaTeX も動かさない。SSG の HTML と hydration の WASM が同じ文字列を持つので、描き分けは起きない
- **式は MathML だけで出す。** KaTeX の出力の形は `mathml` で、`<math>` の要素だけを本文に入れる。KaTeX の CSS もフォントも配信物に足さない。式は文字の色を継ぐので、ライトとダークはトークンのまま切り替わる
- **扱う書き方は決めた分だけ。** 見出し（`##`・`###`）、段落、箇条書き、番号つきの箇条、表、式（文中の `$…$` と別行の `$$…$$`）、文中のコード。ほかの書き方と、組めない式はビルドを止める。黙って捨てたり、TeX のまま出したりしない
- **組み方はサイトの規則に合わせ、クラスは変換が付ける。** `##` はセクション見出し、`###` は `heading`、表は「表」の形、別行の式ははみ出すときに式だけが横に動く箱。役割名のクラスは変換のコード（`src/markdown.rs`）に書き、Tailwind がそこから拾う

## KaTeX の出力の形

比べたのは 2 つ。

- **HTML と CSS とフォント**（`htmlAndMathml`）: どのブラウザでも KaTeX の字形で同じに組める。代わりに KaTeX の CSS 1 枚と、式に出る書体ごとのフォント（KaTeX_Main・KaTeX_Math・KaTeX_Size1 など）を読む。HTML には大きさと位置の `style` 属性が並ぶ。置き場は、サイトの assets なら CSS が 2 つになり（SSG の検査は CSS を 1 つと見る）、CDN なら Google Fonts と別の外の読込が増える
- **MathML だけ**（`mathml`）: 足すものが無い。Chrome（109 から）・Safari・Firefox が MathML Core で組む。字形は OS の数式フォント（Windows の Cambria Math、Apple の STIX Two Math など）で、ブラウザごとに少し違う。読み上げにもそのまま届く

MathML だけを取った。サイトは「軽い」を性格に置き、重いものを入れない（`docs/design/README.md`「性格」）。説明書の式は分数・偏微分・上付きと下付きの程度で、MathML Core で足りる。字形がブラウザで揃わないことより、読込を増やさないことを取った。

## 変換の置き場

build.rs にした。`rustruct-web` は既に `build.rs` で tokens.json を読んで `$OUT_DIR` に書いており、同じ形で `content/` を読める。手続き的マクロにすると、別のクレートが要る。変換は `src/theme.rs` と同じく純粋関数（`src/markdown.rs`）で、`build.rs` が `#[path]` で入れ、lib はテストのときだけ入れて検査する。

## 残ること

- 画像と図は扱わない（後の Issue）
- 見出しに住所の `#id` を付けない。節へのリンクは要るときに足す
- KaTeX の版は `katex` クレートが同梱するもの（0.16.4）。上げるときはクレートを上げる
