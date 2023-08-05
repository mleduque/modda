

use std::{path::PathBuf, borrow::Cow};

use anyhow::{bail, Result};

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::lowercase::LwcString;
use crate::module::location::github::{GithubDescriptor, GitBranch, Github};

use super::http::Http;


#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Source {
    /// An artifact will be downloaded using HTTP/HTTPS
    Http(Http),
    /// The artifact is hosted on github an obtained by http request
    Github(Github),
    /// The artifact is on the local computer filesystem.\
    /// It can be either an archive (zip/tgz/...) or a directory
    Absolute { path: String },
    /// The artifact is on the local computer filesystem, the location is relative to the manifest file.\
    /// It can be either an archive (zip/tgz/...) or a directory
    Local { local: String },
}

impl Default for Source {
    fn default() -> Self {
        Source::Local { local: String::new() }
    }
}

impl Source {
    pub fn save_subdir(&self) -> Result<PathBuf> {
        use Source::*;
        use url::{Url, Host};
        match self {
            Http(self::Http { ref http, .. }) => {
                let url = match Url::parse(http) {
                    Ok(url) => url,
                    Err(error) => bail!("Couldn't parse location url {}\n -> {:?}", http, error),
                };
                let host = match url.host() {
                    None => bail!("Invalid http source {}", http),
                    Some(Host::Domain(ref domain)) => Cow::Borrowed(*domain),
                    Some(Host::Ipv6(ref ipv6)) => Cow::Owned(ipv6.to_string()),
                    Some(Host::Ipv4(ref ipv4)) => Cow::Owned(ipv4.to_string()),
                };
                Ok(PathBuf::from("http").join(&*host))
            }
            Absolute { .. } | Local { .. } => Ok(PathBuf::new()),
            Github(super::github::Github { github_user, repository, .. }) =>
                Ok(PathBuf::from("github").join(github_user).join(repository)),
        }
    }

    pub fn save_name(&self, module_name: &LwcString) -> Result<PathBuf> {
        use Source::*;
        match self {
            Http(super::http::Http { ref http, ref rename,.. }) => {
                match rename {
                    Some(rename) => Ok(PathBuf::from(rename)),
                    None => {
                        let url = match url::Url::parse(http) {
                            Err(error) => bail!("Couldn't parse url {}\n -> {:?}", http, error),
                            Ok(url) => url,
                        };
                        match url.path_segments() {
                            None => bail!("Couldn't decide archive name for url {} - provide one with 'rename' field", http),
                            Some(segments) => match segments.last() {
                                Some(seg) => Ok(PathBuf::from(
                                    percent_encoding::percent_decode_str(seg).decode_utf8_lossy().into_owned()
                                )),
                                None => bail!("Couldn't decide archive name for url {} - provide one with 'rename' field", http),
                            }
                        }
                    }
                }
            }
            Absolute { .. } | Local { .. } => Ok(PathBuf::new()),
            Github(self::Github { descriptor, .. }) => match descriptor {
                GithubDescriptor::Release { asset , ..} =>
                                                    Ok(PathBuf::from(asset.to_owned())),
                GithubDescriptor::Commit { commit } =>
                                                    Ok(PathBuf::from(format!("{}-{}.zip", module_name, commit))),
                GithubDescriptor::Branch(GitBranch { branch, .. }) =>
                                                    Ok(PathBuf::from(format!("{}-{}.zip", module_name, branch))),
                GithubDescriptor::Tag { tag } => Ok(PathBuf::from(format!("{}-{}.zip", module_name, tag))),
            }
        }
    }

    pub fn default_strip_leading(&self) -> usize {
        use GithubDescriptor::*;
        match self {
            Source::Github(Github { descriptor: Commit{..}, .. })
            | Source::Github(Github { descriptor: Tag{..}, .. })
            | Source::Github(Github { descriptor: Branch{..}, .. }) => 1,
            _ => 0,
        }
    }
}

#[cfg(test)]
impl Source {
    pub fn http_source() -> Source {
        Source::Http(Http { http: "https://dummy.example".to_string(), rename: None, ..Default::default() })
    }
    pub fn gh_release_source() -> Source {
        Source::Github(
            Github {
                github_user: "".to_string(),
                repository: "".to_string(),
                descriptor: GithubDescriptor::Release {
                    release: Some("".to_string()),
                    asset: "".to_string(),
                },
                ..Default::default()
            }
        )
    }
    pub fn gh_branch_source() -> Source {
        use crate::module::refresh::RefreshCondition::Never;
        Source::Github(
            Github {
                github_user: "".to_string(),
                repository: "".to_string(),
                descriptor: GithubDescriptor::Branch(GitBranch { branch: "".to_string(), refresh: Never }),
                ..Default::default()
            }
        )
    }
}

