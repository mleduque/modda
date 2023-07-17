
pub mod github;
pub mod http;
pub mod source;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::module::pre_copy_command::PrecopyCommand;
use crate::{archive_layout::Layout, patch_source::PatchDesc, replace::ReplaceSpec};

use self::source::Source;


#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub enum Location {
    Ref { r#ref: String },
    Concrete { concrete: ConcreteLocation },
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct ConcreteLocation {
    #[serde(flatten)]
    pub source: Source,
    /// Specifies which files from the archive will be copied to the game directory.
    /// Read as a Unix shell style glob pattern (https://docs.rs/glob/0.3.0/glob/struct.Pattern.html)
    #[serde(default)]
    pub layout: Layout,
    pub patch: Option<PatchDesc>,
    /// regex-based search and replace, runs after patch.
    pub replace: Option<Vec<ReplaceSpec>>,
    pub precopy: Option<PrecopyCommand>,
}

#[cfg(test)]
mod test_deserialize {
    use crate::module::location::github::{GitBranch, Github, GithubDescriptor};
    use crate::module::location::source::Source;
    use crate::replace::ReplaceSpec;
    use crate::module::refresh::RefreshCondition::Never;

    use super::ConcreteLocation;

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
                descriptor: GithubDescriptor::Branch(GitBranch { branch: "main".to_string(), refresh: Never }),
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                descriptor: GithubDescriptor::Branch(GitBranch { branch: "main".to_string(), refresh: Never }),
                ..Default::default()
            })
        );
    }

    #[test]
    fn deserialize_location_with_replace_property() {
        let yaml = r#"
            github_user: "pseudo"
            repository: my-big-project
            tag: v1
            replace:
                - file_globs: [README.md]
                  replace: typpo
                  with: typo
        "#;
        let location : ConcreteLocation = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            location,
            ConcreteLocation {
                source: Source::Github(Github {
                    github_user: "pseudo".to_string(),
                    repository: "my-big-project".to_string(),
                    descriptor: GithubDescriptor::Tag { tag: "v1".to_string() },
                    ..Default::default()
                }),
                replace: Some(vec![
                    ReplaceSpec {
                        file_globs: vec!["README.md".to_string()],
                        replace: "typpo".to_string(),
                        with: "typo".to_string(),
                        ..Default::default()
                    }
                ]),
                ..Default::default()
            }
        )
    }
}
