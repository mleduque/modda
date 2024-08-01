use std::time::Duration;

use nu_ansi_term::Color::Green;
use log::info;
use serde::{Deserialize, Serialize};

use crate::lowercase::LwcString;


mod post_install_variants {
    named_unit_variant!(interrupt);
    named_unit_variant!(none);
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum PostInstall {
    #[serde(with = "post_install_variants::none")]
    None,
    #[serde(with = "post_install_variants::interrupt")]
    Interrupt,
    WaitSeconds { wait_seconds: u16 },
}

impl Default for PostInstall {
    fn default() -> Self {
        PostInstall::None
    }
}

pub trait PostInstallExec {
    fn exec(&self, mod_name: &LwcString) -> PostInstallOutcome;
}

impl PostInstallExec for PostInstall {
    fn exec(&self, mod_name: &LwcString) -> PostInstallOutcome {
        match self {
            PostInstall::None => PostInstallOutcome::Continue,
            PostInstall::Interrupt => PostInstallOutcome::Stop,
            PostInstall::WaitSeconds { wait_seconds } => {
                // would be nice to implement a countdown and a hotkey to interrupt install
                info!("{}", Green.bold().paint(format!("Post-install wait of {} s for mod {}",
                                                        wait_seconds, mod_name)));
                info!("Ctrl+C to stop the installation");
                wait(*wait_seconds);
                PostInstallOutcome::Continue
            }
        }
    }
}

impl PostInstallExec for Option<PostInstall> {
    fn exec(&self, mod_name: &LwcString) -> PostInstallOutcome {
        match self {
            None => PostInstallOutcome::Continue,
            Some(post_install) => post_install.exec(mod_name),
        }
    }
}

fn wait(seconds: u16) {
    std::thread::sleep(Duration::from_secs(seconds as u64))
}

pub enum PostInstallOutcome {
    Stop,
    Continue,
}
