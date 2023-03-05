use std::borrow::Cow;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::lowercase::LwcString;
use crate::post_install::{PostInstallExec, PostInstallOutcome};

use super::file_mod::FileModule;
use super::gen_mod::GeneratedMod;
use super::weidu_mod::WeiduMod;


#[derive(Debug, PartialEq)]
pub enum Module {
    Mod { weidu_mod: WeiduMod },
    File { file: FileModule },
    Generated { gen: GeneratedMod }
}

impl Module {
    pub fn get_name(&self) -> &LwcString {
        match self {
            Module::Mod { weidu_mod } => &weidu_mod.name,
            Module::File { file } => &file.file_mod,
            Module::Generated { gen } => &gen.gen_mod,
        }
    }

    pub fn get_description(&self) -> &Option<String> {
        match self {
            Module::Mod { weidu_mod } => &weidu_mod.description,
            Module::File { file } => &file.description,
            Module::Generated { gen } => &gen.description,
        }
    }

    pub fn describe(&self) -> Cow<String> {
        match &self.get_description() {
            None => Cow::Borrowed(self.get_name().as_ref()),
            Some(desc) => Cow::Owned(format!("{} ({})", self.get_name().as_ref(), desc)),
        }
    }

    pub fn exec_post_install(&self, mod_name: &LwcString) -> PostInstallOutcome {
        match self {
            Module::Mod { weidu_mod } => weidu_mod.post_install.exec(mod_name),
            Module::File { file } => file.post_install.exec(mod_name),
            Module::Generated { gen } => gen.post_install.exec(mod_name),
        }
    }
}

#[derive(Deserialize, Debug)]
struct ModuleHelper {
    #[serde(flatten)]
    weidu: Option<WeiduMod>,
    #[serde(flatten)]
    file: Option<FileModule>,
    #[serde(flatten)]
    gen_mod: Option<GeneratedMod>,
    #[serde(flatten)]
    unknown: HashMap<String, Value>,
}

impl <'de> Deserialize<'de> for Module {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: serde::Deserializer<'de> {
        let helper = ModuleHelper::deserialize(deserializer)?;
        match helper {
            ModuleHelper { weidu: None, file: None, gen_mod: None, unknown } => Err(serde::de::Error::custom(
                format!("Incorrect module definition found ; could not recognize weidu_mod or file_module or gen_mod definition in [{:?}]", unknown)
            )),
            ModuleHelper { file: Some(file), weidu: None, gen_mod:None, .. } => Ok(Module::File { file }),
            ModuleHelper { weidu: Some(weidu_mod), file: None, gen_mod: None, .. } => Ok(Module::Mod { weidu_mod }),
            ModuleHelper { gen_mod: Some(gen), file: None, weidu: None, .. } => Ok(Module::Generated { gen }),
            ModuleHelper { weidu, file, gen_mod, unknown } =>
                Err(serde::de::Error::custom(
                    format!("Incorrect module definition found ; could not decide module kind,
                                weidu={:?} or file={:?} or gen_mod={:?} with additional data {:?}",
                                weidu, file, gen_mod, unknown)
                )),
        }
    }
}

impl Serialize for Module {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: serde::Serializer {
        match self {
            Module::Mod { weidu_mod } => weidu_mod.serialize(serializer),
            Module::File { file } => file.serialize(serializer),
            Module::Generated { gen } => gen.serialize(serializer),
        }
    }
}

