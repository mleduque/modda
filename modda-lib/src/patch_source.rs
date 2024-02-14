

use std::{borrow::Cow, fmt::Debug};

use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct PatchDesc {
    #[serde(flatten)]
    pub patch_source: PatchSource,
    #[serde(default)]
    pub encoding: PatchEncoding,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum PatchSource {
    /// Inline content inside the yaml file
    Inline { inline: String },
    /// Downloaded from an HTTP resource
    Http { http: String },
    /// From a local file ; path is relative and searched in the order:
    /// - in the same location as the manifest file (or subdirs)
    /// - in the game dir (or subdirs)
    Relative { relative: String },
    // /// From any file on the filesystem
    //Path { path: String },
}

// Come one, people, we're 21th century now!
#[derive(Deserialize, Serialize, Debug, PartialEq, Copy, Clone)]
pub enum PatchEncoding {
    /// Default value, works for correctly encoded text files (utf8) but also for ASCII (7 bits)
    UTF8,
    /// Windows specific charset which includes ASCII plus cyrillic chars, the original reason I
    /// made it possible to choose the charset al tall.
    WIN1251,
    /// Windows specific charset, includes  ASCII for use in western Europe
    WIN1252,
    /* I see no point in defining more. Who uses UTF-16 anyway? */
}

impl Default for PatchEncoding {
    fn default() -> Self {
        PatchEncoding::UTF8
    }
}

impl PatchEncoding {
    pub fn decode<'a> (&self, bytes: &'a [u8]) -> (Cow<'a, str>, &'static Encoding, bool) {
        match self {
            PatchEncoding::UTF8 => encoding_rs::UTF_8.decode(bytes),
            PatchEncoding::WIN1252 => encoding_rs::WINDOWS_1252.decode(bytes),
            PatchEncoding::WIN1251 => encoding_rs::WINDOWS_1251.decode(bytes),
        }
    }
}
