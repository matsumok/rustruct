//! `docs/design/tokens.json` の色と書体を Rust の表（`$OUT_DIR/tokens.rs`）に写す。
//!
//! 値の源は tokens.json の一箇所（`docs/stack.md`）。`{ink}` のような参照はここで解決し、
//! 実行時には役割名で引くだけにする。JSON の読み手は build 時だけで、配信物には入らない。

use std::path::Path;

use serde_json::Value;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let tokens_path = Path::new(&manifest_dir).join("../../docs/design/tokens.json");
    println!("cargo:rerun-if-changed={}", tokens_path.display());
    println!("cargo:rerun-if-changed=build.rs");

    let text = std::fs::read_to_string(&tokens_path)
        .unwrap_or_else(|e| panic!("{} を読めない: {e}", tokens_path.display()));
    let tokens: Value = serde_json::from_str(&text).expect("tokens.json を JSON として読めない");

    let colors = tokens["color"]["tokens"]
        .as_array()
        .expect("tokens.json の color.tokens が配列でない");
    let raw: Vec<(&str, &str, &str)> = colors
        .iter()
        .map(|c| {
            (
                c["name"].as_str().expect("color.tokens の name"),
                c["value"]["light"]
                    .as_str()
                    .expect("color.tokens の value.light"),
                c["value"]["dark"]
                    .as_str()
                    .expect("color.tokens の value.dark"),
            )
        })
        .collect();

    let mut out = String::from(
        "/// tokens.json の色。(役割名, ライト, ダーク)。参照は解決済み。\n\
         pub(super) const COLORS: &[(&str, &str, &str)] = &[\n",
    );
    for (name, light, dark) in &raw {
        let light = resolve(light, |n| raw.iter().find(|c| c.0 == n).map(|c| c.1));
        let dark = resolve(dark, |n| raw.iter().find(|c| c.0 == n).map(|c| c.2));
        out.push_str(&format!("    ({name:?}, {light:?}, {dark:?}),\n"));
    }
    out.push_str("];\n");

    let family = |name: &str| {
        tokens["type"]["families"][name]
            .as_str()
            .unwrap_or_else(|| panic!("tokens.json の type.families.{name} が無い"))
            .to_string()
    };
    out.push_str(&format!(
        "pub(super) const FONT_SANS: &str = {:?};\npub(super) const FONT_MONO: &str = {:?};\n",
        family("sans"),
        family("mono")
    ));

    std::fs::write(Path::new(&out_dir).join("tokens.rs"), out).expect("tokens.rs を書けない");
}

/// `{ink}` のような参照を同じテーマの値で置き換える。先が参照でもよい。
fn resolve<'a>(value: &'a str, lookup: impl Fn(&str) -> Option<&'a str>) -> &'a str {
    let mut current = value;
    let mut seen = Vec::new();
    while let Some(name) = current.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        assert!(
            !seen.contains(&name),
            "tokens.json の参照 {value} が循環している"
        );
        seen.push(name);
        current = lookup(name).unwrap_or_else(|| panic!("tokens.json の参照先 {name} が無い"));
    }
    current
}
