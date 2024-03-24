
use dioxus::prelude::*;

use crate::model::GameDir;

#[component]
pub fn GameDirSelection(cx: Scope) -> Element {
    let game_dir = use_shared_state::<GameDir>(cx).unwrap();
    cx.render(rsx! {
        style { include_str!("./style.css") }
        div {
            display: "flex",
            flex_direction: "row",
            class: "game-dir-selection",
            label {
                margin_right: "2em",
                "Game location"
            }
            label {
                r#for: "game-dir",
                margin_right: "1em",
                hidden: game_dir.read().path().is_none(),
                match &game_dir.read().path() {
                    None => "Select the game directory".to_string(),
                    Some(dir) => dir.as_os_str().to_string_lossy().to_string(),
                }
            }
            button {
                hidden: game_dir.read().path().is_none(),
                onclick: move |_| {
                    *game_dir.write() = GameDir::none()
                },
                "❌"
            }
            button {
                id: "game-dir",
                hidden: game_dir.read().path().is_some(),
                onclick: move |_| {
                    to_owned!(game_dir);
                    let start = match game_dir.read().path() {
                        None => match directories::UserDirs::new() {
                            Some(user_dirs) => user_dirs.home_dir().to_owned(),
                            None => match current_dir() {
                                Ok(dir) => dir,
                                Err(_) => PathBuf::from("/"),
                            }
                        }
                        Some(ref loc) => PathBuf::from(loc),
                    };
                    async move {
                        let selection = rfd::AsyncFileDialog::new()
                            .set_directory(&start)
                            .pick_folder()
                            .await;
                        let new_value =match selection {
                            Some(folder) => GameDir::some(folder.path().to_owned()),
                            None => GameDir::none(),
                        };
                        if let Some(path) = new_value {
                            match has_chitin_key(path) {
                                Ok(true) => *game_dir.write() = new_value,
                                Ok(false) =>
                            }
                        }

                    }
                },
                "Select game directory",
            }
        }
    })
}
