//! SSG が書き出す住所を 1 行 1 つで出す。`scripts/check-ssg.sh` が期待する住所の集合をここから取り、
//! ルーターと検査が別々の一覧を持たないようにする（CODING_STANDARDS「導出できる値は保存しない」）。
//!   cargo run -q -p rustruct-web --example static_paths --locked

fn main() {
    for path in rustruct_web::static_paths() {
        println!("{path}");
    }
}
