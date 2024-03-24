
use dioxus::prelude::*;
use dioxus_router::hooks::use_navigator;

use crate::model::{GameDir, ManifestPath};
use crate::Route;

#[component]
pub fn ManifestChoice(cx: Scope) -> Element {
    let manifest_path = use_shared_state::<ManifestPath>(cx).unwrap();
    let game_dir = use_shared_state::<GameDir>(cx).unwrap();
    let navigator = use_navigator(&cx);

    cx.render(rsx! {
        div {
            class: "manifest-choice",

            button {
                onclick: move |_| {},
                "Load YAML manifest file",
            }
            button {
                onclick: move |_| {
                    navigator.push(Route::Discover {});
                },
                disabled: !game_dir.read().path().is_none(),
                "Discover mods in the game directory",
            }
            button {
                onclick: move |_| { navigator.push(Route::Discover {}); },
                disabled: !game_dir.read().path().is_none(),
                "Build from weidu.log",
            }
        }
    })
}

