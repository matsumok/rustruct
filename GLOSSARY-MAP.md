# Glossary Map

## Contexts

- [サイト全体](./GLOSSARY.md): 四部構成と分野、利用者、Model と限定データ。どの Tool にも共通する言葉
- [材料](./docs/materials/GLOSSARY.md): コンクリートと鉄筋。複数の Tool が共有する
- [スラブ](./docs/slab/GLOSSARY.md): 長方形の RC スラブの形（内法スパン・スラブ厚）。RC スラブの Tool が共有する
- [必要スラブ厚さ](./docs/tools/slab-thickness/GLOSSARY.md): RC 二方向スラブ（四辺固定）の必要スラブ厚と等厚曲線
- [Kirchhoff 板](./docs/tools/kirchhoff-plate/GLOSSARY.md): 長方形の等厚な板（辺ごとに支持条件を選ぶ）の、等分布荷重によるたわみと応力の分布。材料に依らない
- [スラブ設計](./docs/tools/slab-design/GLOSSARY.md): RC 規準と Kirchhoff 板の解析で、スラブを配筋まで設計する（将来。まだ言葉は無い）
- [地震応答](./docs/tools/seismic-response/GLOSSARY.md): 多質点モデルの地震応答

材料の判断は `docs/materials/adr/`、スラブの共有の判断は `docs/slab/adr/`、各 Tool の固有の判断は `docs/tools/<tool>/adr/`、サイト全体の判断は `docs/adr/` に置く。

## Relationships

- **材料 → 各 Tool**: Tool は材料の言葉と型をそのまま使い、Tool の GLOSSARY で定義し直さない
- **スラブ → 各スラブ Tool**: 必要スラブ厚さとスラブ設計は、スラブの言葉と型をそのまま使い、Tool の GLOSSARY で定義し直さない
- **Kirchhoff 板 → スラブ設計**: スラブ設計は Kirchhoff 板の計算核と言葉（辺・支持条件・結果の量）を使い、定義し直さない。スラブの内法スパンを Kirchhoff 板の辺長として渡す（`docs/slab/adr/0001`）
- **サイト全体 → 各 Tool**: 各 Tool の Model は、サイト全体で定義する Model の一種。呼ぶときは「必要スラブ厚さ Model」「地震応答 Model」のように修飾する
- **必要スラブ厚さ と Kirchhoff 板**: 互いに使わない。記号 w は必要スラブ厚さでは全荷重、Kirchhoff 板ではたわみを指す
