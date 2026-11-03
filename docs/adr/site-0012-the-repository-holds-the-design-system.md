# デザインシステムの正本はリポジトリの docs/design に置き、claude.ai のアーティファクトは更新しない

デザインシステムの正本を `docs/design/README.md` と `docs/design/tokens.json` にする（2026-10-23、本人が決めた）。9 月にデザインシステムを作った本人の claude.ai のアーティファクトは、そのときの見本として残すが、更新しない。

## 理由

9 月から、決め事はまずリポジトリに書き、claude.ai のアーティファクトへは後で反映する、という順で進めてきた。表と列の単位（ADR site-0006）、画面幅（ADR site-0007）、分布図（ADR site-0011）は、どれも「アーティファクトへはまだ反映していない」のまま残った。アーティファクトはチャットで直すので、決め事を一つずつ打ち込むのは手間が大きい。値はもともとコードが `tokens.json` から読み、文章もエージェントが読むのは `docs/design/README.md` なので、リポジトリだけで足りる。二か所に持てば、いずれ食い違う。

## 結果

- 見た目の決め事は `docs/design/README.md`・`docs/design/tokens.json`・`docs/ui-decisions.md`・ADR に書けば済む。アーティファクトへの反映は要らない
- ADR site-0006・site-0007・site-0011 の「アーティファクトへの反映が残っている」は、これで要らなくなった
- 書き分けは、`docs/design/README.md` と `docs/design/tokens.json` が今の規則、ADR が決定と理由、`docs/ui-decisions.md` は手順と指し先（2026-11-04、#97）
- アーティファクトの URL と ID を公開リポジトリに書かない規則（`AGENTS.md`）は、アーティファクトが残る限りそのまま
