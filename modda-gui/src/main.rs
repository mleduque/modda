

mod game_dir_selection;
mod manifest_choice;
mod model;

// import the prelude to get access to the `rsx!` macro and the `Scope` and `Element` types
use dioxus::prelude::*;
use dioxus_router::components::{Outlet, Router};
use dioxus_router::hooks::use_navigator;
use dioxus_router::prelude::Routable;

use crate::game_dir_selection::GameDirSelection;
use crate::model::GameDir;
use crate::manifest_choice::ManifestChoice;

fn main() {
    // launch the dioxus app in a webview
    dioxus_desktop::launch(App);
}

#[derive(Clone, Debug, PartialEq, Routable)]
pub enum Route {
    #[layout(HeaderWrapper)]
        #[route("/")]
        Home {},
        #[route("/discover")]
        Discover {},
}

#[component]
fn HeaderWrapper(cx: Scope) -> Element {
    let navigator = use_navigator(&cx);
    cx.render(rsx! {
        GameDirSelection(cx)
        Outlet::<Route> { }
    })
}

#[component]
fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, || GameDir::none());
    cx.render(
        rsx! {
            Router::<Route> {}
        }
    )
}

#[component]
fn Home(cx: Scope) -> Element {
    let game_dir = use_shared_state::<GameDir>(cx).unwrap();
    let navigator = use_navigator(&cx);
    cx.render(rsx! {
        ManifestChoice(cx)
    })
}


#[component]
fn Discover(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            "Nothing yet"
        }
    })
}
