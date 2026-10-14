# Glossary Map

## Contexts

- [サイト全体](./GLOSSARY.md): 三部構成、利用者、Model と限定データ。どの Tool にも共通する言葉
- [材料](./docs/materials/GLOSSARY.md): コンクリートと鉄筋。複数の Tool が共有する
- [スラブたわみ](./docs/tools/slab/GLOSSARY.md): RC 二方向スラブ（四辺固定）の必要厚さと等厚曲線
- [地震応答](./docs/tools/seismic-response/GLOSSARY.md): 多質点モデルの地震応答

材料の判断は `docs/materials/adr/`、各 Tool の固有の判断は `docs/tools/<tool>/adr/`、サイト全体の判断は `docs/adr/` に置く。

## Relationships

- **材料 → 各 Tool**: Tool は材料の言葉と型をそのまま使い、Tool の GLOSSARY で定義し直さない
- **サイト全体 → 各 Tool**: 各 Tool の Model は、サイト全体で定義する Model の一種。呼ぶときは「スラブたわみ Model」「地震応答 Model」のように修飾する
