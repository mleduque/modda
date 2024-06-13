use std::borrow::Cow;

use anyhow::Result;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::canon_path::CanonPath;
use crate::module::components::{Components, Component};
use crate::lowercase::LwcString;
use crate::post_install::{PostInstallExec, PostInstallOutcome};

use super::disable_condition::{DisableCheck, DisableOutCome};
use super::gen_mod::GeneratedMod;
use super::weidu_mod::WeiduMod;


#[derive(Debug, PartialEq, Clone)]
pub enum Module {
    Mod { weidu_mod: WeiduMod },
    Generated { gen: GeneratedMod }
}

impl Module {
    pub fn get_name(&self) -> &LwcString {
        match self {
            Module::Mod { weidu_mod } => &weidu_mod.name,
            Module::Generated { gen } => &gen.gen_mod,
        }
    }

    pub fn get_description(&self) -> &Option<String> {
        match self {
            Module::Mod { weidu_mod } => &weidu_mod.description,
            Module::Generated { gen } => &gen.description,
        }
    }

    pub fn get_components(&self) -> Components {
        match self {
            Module::Mod { weidu_mod } => weidu_mod.components.clone(),
            Module::Generated { gen } => Components::List(vec![Component::Simple(gen.component.index)]),
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
            Module::Generated { gen } => gen.post_install.exec(mod_name),
        }
    }

    pub fn check_disabled(&self, manifest_root: &CanonPath) -> Result<DisableOutCome> {
        match self {
            Module::Mod { weidu_mod } => weidu_mod.disabled_if.check(manifest_root),
            Module::Generated { gen } => gen.disabled_if.check(manifest_root),
        }
    }
}

impl <'de> Deserialize<'de> for Module {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: serde::Deserializer<'de> {
        let helper = Value::deserialize(deserializer)?;
        match helper {
            Value::Mapping(ref mapping) => {
                let has_name = mapping.get(Value::String("name".to_string())).is_some();
                let has_gen_mod = mapping.get(Value::String("gen_mod".to_string())).is_some();
                match (has_name, has_gen_mod) {
                    (false, false) =>
                        Err(serde::de::Error::custom("'modules' item doesn't have a 'name' or 'gen_mod' field")),
                    (true, false) => {
                        WeiduMod::deserialize(helper.into_deserializer())
                            .map(|weidu_mod| Module::Mod { weidu_mod })
                            .map_err(serde::de::Error::custom)
                    }
                    (false, true) => {
                        GeneratedMod::deserialize(helper.into_deserializer())
                            .map(|gen| Module::Generated { gen })
                            .map_err(serde::de::Error::custom)
                    }
                    _ => Err(serde::de::Error::custom("'modules' item must have only one of 'name' or 'gen_mod'")),
                }
            }
            _ => Err(serde::de::Error::custom("'modules' item is not a map"))
        }
    }

}

impl Serialize for Module {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: serde::Serializer {
        match self {
            Module::Mod { weidu_mod } => weidu_mod.serialize(serializer),
            Module::Generated { gen } => gen.serialize(serializer),
        }
    }
}
