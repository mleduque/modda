

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
    /// made it possible to choose the charset at all.
    ///
    /// https://encoding.spec.whatwg.org/windows-1251.html
    WIN1251,
    /// Windows specific charset, includes  ASCII for use in western Europe<br>
    /// encoding-rs conflates ISO-8859-1 with this
    ///
    /// https://encoding.spec.whatwg.org/windows-1252.html
    WIN1252,
    LATIN1,
    ISO8859_1,
    /// Revised western european, latin-9
    ///
    /// https://encoding.spec.whatwg.org/iso-8859-15.html
    ISO8859_15,
    LATIN9,

    // The encodings below are added because weidu supports them by default
    // and because Hyrum's law  says they are used somewhere.
    //But they are completely untested so probably completely broken

    /// Charset used by weidu for `infer_charsets` for schinese
    CP936,
    /// Charset used by weidu for `infer_charsets` for tchinese
    CP950,
    /// Charset used by weidu for `infer_charsets` for czech and polish
    CP1250,
    /// Charset used by weidu for `infer_charsets` for japanese
    CP932,
    /// Charset used by weidu for `infer_charsets` for korean
    CP949,

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
            PatchEncoding::WIN1252
                | PatchEncoding::ISO8859_1
                | PatchEncoding::LATIN1 => encoding_rs::WINDOWS_1252.decode(bytes),
            PatchEncoding::WIN1251 => encoding_rs::WINDOWS_1251.decode(bytes),
            PatchEncoding::ISO8859_15
                | PatchEncoding::LATIN9 => encoding_rs::ISO_8859_15.decode(bytes),
            PatchEncoding::CP936 => encoding_rs::GB18030.decode(bytes),
            PatchEncoding::CP950 => encoding_rs::BIG5.decode(bytes),
            PatchEncoding::CP1250 => encoding_rs::WINDOWS_1250.decode(bytes),
            PatchEncoding::CP932 => encoding_rs::SHIFT_JIS.decode(bytes),
            PatchEncoding::CP949 => encoding_rs::EUC_KR.decode(bytes),
        }
    }
}
