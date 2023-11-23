
use std::error::Error;
use std::path::PathBuf;

use anyhow::{Result, bail, anyhow};
use lazy_static::lazy_static;
use log::info;
use reqwest::header::{AUTHORIZATION, HeaderMap, ACCEPT, HeaderValue, HeaderName, USER_AGENT};
use serde::{Deserialize, Serialize};

use crate::credentials::{Credentials, GithubCredentials};
use crate::download::{Downloader, DownloadOpts};
use crate::module::refresh::RefreshCondition;
use crate::progname::PROGNAME;

#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct Github {
    pub github_user: String,
    pub repository: String,
    #[serde(flatten)]
    pub descriptor: GithubDescriptor,
    /// If set, will not cache the artifact after downloading it
    #[serde(default)]
    pub no_cache: bool,
    /// None -> no auth needed, Some -> method + credentials for example `PAT <token_name>`
    pub auth: Option<String>,
}

impl Github {
    pub async fn get_github(&self, downloader: &Downloader, dest: &PathBuf, save_name: PathBuf) -> Result<PathBuf> {
        let url = self.descriptor.get_url(&self.github_user, &self.repository, &self.auth).await?;
        let opts = &DownloadOpts { no_cache: self.no_cache, refresh: self.refresh() };
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, self.descriptor.get_media_type());
        headers.insert(&*GITHUB_API_VERSION_NAME, (*GITHUB_API_VERSION_VALUE).clone());
        if let Some(auth_spec) = &self.auth {
            headers.extend(decode_auth(auth_spec)?);
        }

        downloader.download(&url, dest, save_name, opts, &Some(headers)).await
    }

    pub fn refresh(&self) -> RefreshCondition {
        match &self.descriptor {
            GithubDescriptor::Branch(GitBranch { refresh, .. }) => refresh.clone(),
            _ => RefreshCondition::Never,
        }
    }
}

lazy_static! {
    static ref GITHUB_API_VERSION_VALUE: HeaderValue = "2022-11-28".parse().unwrap();
    static ref GITHUB_API_VERSION_NAME: HeaderName = HeaderName::from_bytes(b"X-GitHub-Api-Version").unwrap();
    static ref GITHUB_CUSTOM_MEDIA_TYPE: HeaderValue = "application/vnd.github.v3.raw".parse().unwrap();
    static ref MEDIA_TYPE_OCTET_STREAM: HeaderValue = "application/octet-stream".parse().unwrap();
}

fn decode_auth(spec: &str) -> Result<HeaderMap> {
    let mut parts = spec.split_ascii_whitespace();
    let method = parts.next();
    let data = parts.next();
    match (method, data) {
        (Some("PAT"), Some(token_key)) if token_key.len() > 0 => {
            let cred_config = Credentials::read()?;
            let token = cred_config.github
                .map(|github_creds| match github_creds{
                    GithubCredentials::PersonalAccessToken { personal_tokens } =>
                        personal_tokens.get(token_key).cloned()
                }).flatten();
            let token = match token {
                None => bail!("Github personal access token named `{}` was not found in credentials file", token_key),
                Some(token) => token,
            };

            let mut result = HeaderMap::new();
            result.insert(AUTHORIZATION, format!("token {}", token).parse()?);
            Ok(result)
        }
        _ => bail!("Unrecognized github `auth` value")
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

    pub fn get_media_type(&self) -> HeaderValue {
        match self {
            GithubDescriptor::Release { .. } => (*MEDIA_TYPE_OCTET_STREAM).clone(),
            _ => (*GITHUB_CUSTOM_MEDIA_TYPE).clone()
        }
    }

    pub async fn get_url(&self, user: &str, repository: &str, auth: &Option<String>,) -> Result<String> {
        use GithubDescriptor::*;

        match self {
            Release { release, asset } => {
                let release = match &release {
                    None => String::from("latest"),
                    Some(release) => release.to_owned(),
                };
                // First search the release by tag-name
                let release_info = match GithubClient::new(auth)?.get_release_info(user, repository, &release).await {
                    Ok(value) => value,
                    Err(error) => bail!("Could not find release {release} in github repository {user}/{repository}\n{error}")
                };

                // Search a match in the listed assets
                let lookup = asset.replace("{{release}}", &release);
                release_info.assets.iter()
                    .find(|asset| asset.name == lookup)
                    .map(|asset| asset.url.to_owned())
                    .ok_or(anyhow!("No asset named {asset} found for release {release} in github repository {user}/{repository}"))
            }
            Tag { tag } =>
                Ok(format!("https://api.github.com/repos/{user}/{repository}/zipball/{tag}")),
            Branch(GitBranch { branch, refresh: _}) =>
                Ok(format!("https://api.github.com/repos/{user}/{repository}/zipball/{branch}")),
            Commit { commit } =>
                Ok(format!("https://api.github.com/repos/{user}/{repository}/zipball/{commit}")),
        }
    }

}

pub struct GithubClient {
    client: reqwest::Client,
}

impl GithubClient {

    pub fn new(auth: &Option<String>) -> Result<Self> {
        let client_builder = reqwest::ClientBuilder::new()
            .user_agent(PROGNAME);
        let client_builder = if let Some(auth_spec) = auth {
            client_builder.default_headers(decode_auth(auth_spec)?)
        } else {
            client_builder
        };
        Ok(Self { client: client_builder.build()? })
    }

    fn base() -> String { "https://api.github.com".to_string() }

    async fn get_release_info(&self, user: &str, repository: &str, tag: &str) -> Result<ReleaseInfo> {

        let url = format!("{base}/repos/{user}/{repository}/releases/tags/{tag}", base = GithubClient::base());
        let request = self.client.get(&url)
            .header(USER_AGENT, PROGNAME);

        let response = request.send().await?;
        info!("{:?}", response);
        let result = match response.json::<ReleaseInfo>().await {
            Ok(result) => result,
            Err(error) => {
                if error.is_connect() { bail!("connection error for get release endpoint\n{}", error) }
                if let Some(status) = error.status() {
                    bail!("get release endpoint returned HTTP error {}: {:?}\n{}", status.as_u16(), status.canonical_reason(), error)
                }
                if error.is_decode() { bail!("get release endpoint returned incorrect data\n{}\ncaused by\n{:?}", error, error.source()) }

                bail!("get release endpoint error\n{}", error)
            }
        };
        Ok(result)
    }
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
struct ReleaseInfo {
    pub url: String,
    pub html_url: String,
    pub assets_url: String,
    pub tarball_url: String,
    pub zipball_url: String,
    pub id: u32,
    pub tag_name: String,
    pub body: String,
    pub name: String,
    pub assets: Vec<Asset>,
}
#[derive(Deserialize, Debug, PartialEq, Clone)]
struct Asset {
    pub url: String,
    pub browser_download_url: String,
    pub id: u32,
    pub name: String,
    pub label: Option<String>,
    pub content_type: String,
    pub size: usize,

}
