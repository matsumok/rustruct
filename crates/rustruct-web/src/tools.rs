//! Tools（`GLOSSARY.md`）: Tool の一覧。Tool は分野（`docs/adr/0010`）の見出しの下に並べ、住所は平ら
//! （`/tools/kirchhoff-plate`。`docs/adr/0005`）。分野は見出しでしかないので、住所にもルートにも入れない。

use dioxus::prelude::*;

use crate::{Page, Route, RouteList};

/// Tools の一覧の見出しにする分野。Tool の無い分野は見出しも出さない。分野を足すときは `ALL` に一覧の順で並べる
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Field {
    /// 鉄筋コンクリート
    Rc,
    /// 材料に依らない梁・板の弾性解（`docs/adr/0010`）
    Mechanics,
}

impl Field {
    const ALL: [Self; 2] = [Self::Rc, Self::Mechanics];

    fn name(self) -> &'static str {
        match self {
            Self::Rc => "RC",
            Self::Mechanics => "Mechanics",
        }
    }
}

/// `/tools`。分野ごとに、見出しと Tool の一覧（名前＋一行＋パス）。見出しはセクションの見出しなので、
/// 上に `ink` の線を引く（デザインシステムの SectionHead）。見出しと一覧の間は `space-3`、分野の間は `space-7`。
#[component]
pub(crate) fn ToolsIndex() -> Element {
    rsx! {
        Page { route: Route::Tools {},
            div { class: "flex flex-col gap-7",
                for field in Field::ALL {
                    if Route::tools_in(field).next().is_some() {
                        section { class: "flex flex-col gap-3",
                            h2 { class: "border-t border-ink pt-2 text-heading", {field.name()} }
                            RouteList { routes: Route::tools_in(field).collect::<Vec<_>>() }
                        }
                    }
                }
            }
        }
    }
}
