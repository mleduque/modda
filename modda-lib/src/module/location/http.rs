
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::download::{Downloader, DownloadOpts};
use crate::module::location::size_hint::SizeHint;
use crate::module::refresh::RefreshCondition;


#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct Http {
    pub http: String,
    pub rename: Option<String>,
    #[serde(default)]
    pub no_cache: bool,
    #[serde(default)]
    #[serde(with = "crate::module::refresh::RefreshConditionAsString")]
    pub refresh: RefreshCondition,
    /// Approximation of the expected size, only used for progress display abd on ly if actual size is unknown.
    #[serde(default)]
    pub size_hint: Option<SizeHint>,
}

impl Http {
    pub fn from(http: &str) -> Self { Self { http: http.to_owned(), ..Self::default() } }

    pub async fn download(&self, downloader: &Downloader, dest: &PathBuf, save_name: PathBuf) -> Result<PathBuf> {
        let opts = &DownloadOpts { no_cache: self.no_cache, refresh: self.refresh.clone(), size_hint: self.size_hint.clone() };
        downloader.download(&self.http, dest, save_name, opts, &None).await
    }
}
