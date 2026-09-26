use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::OneOrMany;
use serde_with::formats::PreferOne;


#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum PostPopulateAction {
    Symlink(SymlinkAction),
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct SymlinkAction {
    // Paths of the symlinks to create, from game dir
    #[serde_as(as = "OneOrMany<_, PreferOne>")]
    pub symlinks: Vec<String>,
    // Path of the target of the symlinks, from game dir
    pub symlink_target: String,
}

#[cfg(test)]
mod tests {
    use crate::module::location::github::{Github, GithubDescriptor};
    use crate::module::location::location::{ConcreteLocation, Location};
    use crate::module::location::post_install_action::{PostPopulateAction, SymlinkAction};
    use crate::module::location::source::Source;


    #[test]
    fn deserialize_source_github_with_symlink_post_action() {
        let yaml = r#"
        github_user: my_user
        repository: my_repo
        tag: v1.0
        post_populate:
            - symlinks: [a, b]
              symlink_target: target
        "#;
        let location: Location = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            location,
            Location::Concrete {
                concrete: ConcreteLocation {
                    source: Source::Github(Github {
                        github_user: "my_user".to_string(),
                        repository: "my_repo".to_string(),
                        descriptor: GithubDescriptor::Tag {
                            tag: "v1.0".to_string(),
                        },
                        ..Default::default()
                    }),
                    post_populate: vec![
                        PostPopulateAction::Symlink(
                            SymlinkAction{
                                symlinks: vec!["a".to_string(), "b".to_string()],
                                symlink_target: "target".to_string(),
                            },
                        ),
                    ],
                    ..Default::default()
                },
            }
        );
    }

    #[test]
    fn deserialize_source_github_with_symlink_post_action_single() {
        let yaml = r#"
        github_user: my_user
        repository: my_repo
        tag: v1.0
        post_populate:
            - symlinks: a
              symlink_target: target
        "#;
        let location: Location = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            location,
            Location::Concrete {
                concrete: ConcreteLocation {
                    source: Source::Github(Github {
                        github_user: "my_user".to_string(),
                        repository: "my_repo".to_string(),
                        descriptor: GithubDescriptor::Tag {
                            tag: "v1.0".to_string(),
                        },
                        ..Default::default()
                    }),
                    post_populate: vec![
                        PostPopulateAction::Symlink(
                            SymlinkAction{ symlinks: vec!["a".to_string()], symlink_target: "target".to_string() },
                        ),
                    ],
                    ..Default::default()
                },
            }
        );
    }
    #[test]
    fn deserialize_source_path_with_symlink_post_action() {
        let yaml = r#"
        path: /somewhere/else/mod.zip
        post_populate:
            - symlinks: [a]
              symlink_target: target
        "#;
        let location: Location = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            location,
            Location::Concrete {
                concrete: ConcreteLocation {
                    source: Source::Absolute {
                        path: "/somewhere/else/mod.zip".to_string(),
                    },
                    post_populate: vec![
                        PostPopulateAction::Symlink(
                            SymlinkAction{ symlinks: vec!["a".to_string()], symlink_target: "target".to_string() },
                        ),
                    ],
                    ..Default::default()
                },
            }
        );
    }
}
