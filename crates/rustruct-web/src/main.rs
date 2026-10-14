fn main() {
    let builder = dioxus::LaunchBuilder::new();
    // server だけ: 描いたページを public/ に書く（`dx build --ssg`）。web はブラウザで hydration するだけ
    #[cfg(feature = "server")]
    let builder = builder.with_cfg(rustruct_web::serve_config());
    builder.launch(rustruct_web::App);
}
