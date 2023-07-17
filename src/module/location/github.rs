
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::download::{Downloader, DownloadOpts};
use crate::module::refresh::RefreshCondition;

#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct Github {
    pub github_user: String,
    pub repository: String,
    #[serde(flatten)]
    pub descriptor: GithubDescriptor,
    /// If set, will not cache the artifact after downloading it
    #[serde(default)]
    pub no_cache: bool,
}
impl Github {
    pub async fn get_github(&self, downloader: &Downloader, dest: &PathBuf, save_name: PathBuf) -> Result<PathBuf> {
        let url = self.descriptor.get_url(&self.github_user, &self.repository);
        let opts = &DownloadOpts { no_cache: self.no_cache, refresh: self.refresh() };
        downloader.download(&url, dest, save_name, opts).await
    }

    pub fn refresh(&self) -> RefreshCondition {
        match &self.descriptor {
            GithubDescriptor::Branch(GitBranch { refresh, .. }) => refresh.clone(),
            _ => RefreshCondition::Never,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum GithubDescriptor {
    Release { release: Option<String>, asset: String },
    Commit { commit: String },
    Branch(GitBranch),
    Tag { tag: String },
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct GitBranch {
    pub branch: String,
    #[serde(default)]
    #[serde(with = "crate::module::refresh::RefreshConditionAsString")]
    pub refresh: RefreshCondition,
}

impl Default for GithubDescriptor {
    fn default() -> Self {
        GithubDescriptor::Tag { tag: "".to_string() }
    }
}

impl GithubDescriptor {

    pub fn get_url(&self, user: &str, repository: &str) -> String {
        use GithubDescriptor::*;

        match self {
            Release { release, asset } => {
                let release = match &release {
                    None => String::from("latest"),
                    Some(release) => release.to_owned(),
                };
                format!("https://github.com/{user}/{repo}/releases/download/{release}/{asset}",
                    user = user,
                    repo = repository,
                    release = release,
                    asset = asset.replace("{{release}}", &release),
                )
            }
            Tag { tag } =>
                format!("https://github.com/{user}/{repo}/archive/refs/tags/{tag}.zip",
                    user = user,
                    repo = repository,
                    tag = tag,
                ),
            Branch(GitBranch { branch, refresh}) =>
                format!("https://github.com/{user}/{repo}/archive/refs/heads/{branch}.zip",
                    user = user,
                    repo = repository,
                    branch = branch,
                ),
            Commit { commit } =>
                format!("https://github.com/{user}/{repo}/archive/{commit}.zip",
                    user = user,
                    repo = repository,
                    commit = commit,
                ),
        }
    }
}
