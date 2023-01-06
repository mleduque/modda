
use std::{path::PathBuf, borrow::Cow};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{archive_layout::Layout, patch_source::PatchDesc, replace::ReplaceSpec, download::Downloader, module::PrecopyCommand};

#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct Location {
    #[serde(flatten)]
    pub source: Source,
    /// Specifies which files from the archive will be copied to the game directory.
    /// Read as a Unix shell style glob pattern (https://docs.rs/glob/0.3.0/glob/struct.Pattern.html)
    #[serde(default)]
    pub layout: Layout,
    #[serde(default)]
    pub patch: Option<PatchDesc>,
    /// regex-based search and replace, runs after patch.
    #[serde(default)]
    pub replace: Option<Vec<ReplaceSpec>>,
    #[serde(default)]
    pub precopy: Option<PrecopyCommand>,
}


#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Source {
    Http { http: String, rename: Option<String> },
    Github(Github),
    Absolute { path: String },
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
            Http { ref http, .. } => {
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
            Absolute { .. } | Local { .. }=> Ok(PathBuf::new()),
            Github(self::Github { github_user, repository, .. }) =>
                Ok(PathBuf::from("github").join(github_user).join(repository)),
        }
    }

    pub fn save_name(&self, module_name:&str) -> Result<PathBuf> {
        use Source::*;
        match self {
            Http { ref http, ref rename } => {
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
                                                    Ok(PathBuf::from(format!("{}-{}.zip",module_name, commit))),
                GithubDescriptor::Branch { branch } =>
                                                    Ok(PathBuf::from(format!("{}-{}.zip",module_name, branch))),
                GithubDescriptor::Tag { tag } => Ok(PathBuf::from(format!("{}-{}.zip",module_name, tag))),
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct Github {
    pub github_user: String,
    pub repository: String,
    #[serde(flatten)]
    pub descriptor: GithubDescriptor
}
impl Github {
    pub async fn get_github(&self, downloader: &Downloader, dest: &PathBuf, save_name: PathBuf) -> Result<PathBuf> {
        downloader.download(
            &self.descriptor.get_url(&self.github_user, &self.repository),
            dest,
            save_name,
        ).await
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum GithubDescriptor {
    Release { release: Option<String>, asset: String },
    Commit { commit: String },
    Branch { branch: String },
    Tag { tag: String },
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
            Branch { branch } =>
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


#[cfg(test)]
impl Source {
    pub fn http_source() -> Source {
        Source::Http { http: "https://dummy.example".to_string(), rename: None }
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
            }
        )
    }
    pub fn gh_branch_source() -> Source {
        Source::Github(
            Github {
                github_user: "".to_string(),
                repository: "".to_string(),
                descriptor: GithubDescriptor::Branch {
                    branch: "".to_string(),
                },
            }
        )
    }
}


#[cfg(test)]
mod test_deserialize {
    use crate::location::{Github, Source, GithubDescriptor};

    #[test]
    fn deserialize_source_github_branch() {
        let yaml = r#"
        github_user: my_user
        repository: my_repo
        branch: main
        "#;
        let source: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            source,
            Source::Github(Github {
                github_user: "my_user".to_string(),
                repository: "my_repo".to_string(),
                descriptor: GithubDescriptor::Branch {
                    branch: "main".to_string(),
                },
            })
        );
    }

    #[test]
    fn deserialize_source_github_tag() {
        let yaml = r#"
        github_user: my_user
        repository: my_repo
        tag: v1.0
        "#;
        let source: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            source,
            Source::Github(Github {
                github_user: "my_user".to_string(),
                repository: "my_repo".to_string(),
                descriptor: GithubDescriptor::Tag {
                    tag: "v1.0".to_string(),
                },
            })
        );
    }

    #[test]
    fn deserialize_source_github_committag() {
        let yaml = r#"
        github_user: my_user
        repository: my_repo
        commit: 0123456789abcdef
        "#;
        let source: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            source,
            Source::Github(Github {
                github_user: "my_user".to_string(),
                repository: "my_repo".to_string(),
                descriptor: GithubDescriptor::Commit {
                    commit: "0123456789abcdef".to_string(),
                },
            })
        );
    }

    #[test]
    fn deserialize_source_github_release() {
        let yaml = r#"
        github_user: my_user
        repository: my_repo
        release: "1.0"
        asset: my_repo-1.0.zip
        "#;
        let source: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            source,
            Source::Github(Github {
                github_user: "my_user".to_string(),
                repository: "my_repo".to_string(),
                descriptor: GithubDescriptor::Release {
                    release: Some("1.0".to_string()),
                    asset: "my_repo-1.0.zip".to_string(),
                },
            })
        );
    }

    #[test]
    fn deserialize_source_github_branch_as_json() {
        let yaml = r#"{
        "github_user": "my_user",
        "repository": "my_repo",
        "branch": "main"
        }"#;
        let source: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            source,
            Source::Github(Github {
                github_user: "my_user".to_string(),
                repository: "my_repo".to_string(),
                descriptor: GithubDescriptor::Branch {
                    branch: "main".to_string(),
                },
            })
        );
    }

}
