
use std::path::PathBuf;

use anyhow::{bail, Result};
use chrono::Local;
use path_clean::PathClean;

use crate::apply_patch::patch_module;
use crate::archive_extractor::Extractor;
use crate::args::Install;
use crate::cache::Cache;
use crate::canon_path::CanonPath;
use crate::download::Downloader;
use crate::global::Global;
use crate::module::global_locations::GlobalLocations;
use crate::module::location::source::Source;
use crate::timeline::SetupTimeline;
use crate::module::location::{ConcreteLocation, Location};
use crate::lowercase::LwcString;
use crate::module::weidu_mod::WeiduMod;
use crate::replace::ReplaceSpec;
use crate::settings::Config;

pub struct ModuleDownload<'a> {
    pub global: &'a Global,
    pub global_locations: &'a GlobalLocations,
    pub opts: &'a Install,
    pub downloader: &'a Downloader,
    pub extractor: Extractor<'a>,
    pub cache: &'a Cache,
    pub game_dir: &'a CanonPath,
}

impl <'a> ModuleDownload<'a> {

    pub fn new(config: &'a Config, global: &'a Global, global_locations: &'a GlobalLocations,
                opts: &'a Install, downloader: &'a Downloader,
                game_dir: &'a CanonPath, cache:&'a Cache) -> Self {
        Self {
            global,
            global_locations,
            opts,
            downloader,
            extractor: Extractor::new(game_dir, config),
            cache,
            game_dir,
        }
    }

    // at some point, I'd like to have a pool of downloads with installations done
    // concurrently as soon as modules are there
    #[tokio::main]
    pub async fn get_module(&self, module: &WeiduMod) -> Result<SetupTimeline> {
        match &module.location {
            None => match self.global_locations.find(&module.name) {
                None => bail!("No location provided for missing mod {}", module.name),
                Some(found) => self.get_mod_from_concrete_location(found, &module.name).await,
            }
            Some(Location::Concrete { concrete }) =>
                    self.get_mod_from_concrete_location(concrete, &module.name).await,
            Some(Location::Ref { r#ref: reference }) => match self.global_locations.find(reference) {
                None => bail!("Provided location reference for  mod {} was not found (at location key {})", module.name, reference),
                Some(found) => self.get_mod_from_concrete_location(found, &module.name).await,
            }
        }
    }

    async fn get_mod_from_concrete_location(&self, location: &ConcreteLocation, mod_name: &LwcString) -> Result<SetupTimeline> {
        let start = Local::now();
        let archive = match self.retrieve_location(&location, &mod_name).await {
            Ok(archive) => archive,
            Err(error) => bail!("retrieve archive failed for module {}\n-> {:?}", mod_name, error),
        };
        let downloaded = Some(Local::now());

        let dest = std::env::current_dir()?;
        let dest = CanonPath::new(dest)?;
        self.extractor.extract_files(&archive, &mod_name , location)?;
        let copied = Some(Local::now());
        patch_module(&dest, &mod_name , &location.patch, &self.opts).await?;
        let patched = Some(Local::now());
        replace_module(&dest, &mod_name , &location.replace)?;
        let replaced = Some(Local::now());
        Ok(SetupTimeline { start, downloaded, copied, patched, replaced, configured: None })
    }

    pub async fn retrieve_location(&self, loc: &ConcreteLocation, mod_name: &LwcString) -> Result<PathBuf> {
        let dest = self.cache.join(loc.source.save_subdir()?);
        let save_name = loc.source.save_name(mod_name)?;
        match &loc.source {
            Source::Http(http) => http.download(self.downloader, &dest, save_name).await,
            Source::Github(github) => github.get_github(&self.downloader, &dest, save_name).await,
            Source::Absolute { path } => Ok(PathBuf::from(path)),
            Source::Local { local } => self.get_local_mod_path(local),
        }
    }

    fn get_local_mod_path(&self, local_mod_name: &String) -> Result<PathBuf, anyhow::Error> {
        let manifest_path = self.get_manifest_root().clean();
        let local_mods = match &self.global.local_mods {
            None => PathBuf::new(),
            Some(path) => PathBuf::from(path).clean(),
        };
        if local_mods.is_absolute() || local_mods.starts_with("..") {
            bail!("Invalid local_mods value");
        }
        let mod_name = PathBuf::from(local_mod_name).clean();
        if mod_name.is_absolute() || local_mods.starts_with("..") {
            bail!("Invalid local value");
        }
        Ok(manifest_path.join(local_mods).join(mod_name))
    }

    pub fn get_manifest_root(&self) -> PathBuf {
        let manifest = PathBuf::from(&self.opts.manifest_path);
        match manifest.parent() {
            None => PathBuf::from(&self.game_dir),
            Some(path) => PathBuf::from(path),
        }
    }
}


fn replace_module(game_dir: &CanonPath, module_name: &LwcString, replace: &Option<Vec<ReplaceSpec>>) -> Result<()> {
    if let Some(specs) = replace {
        for spec in specs {
            let mod_path = game_dir.join_path(module_name.as_ref());
            spec.exec(&mod_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test_retrieve_location {


    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::global::Global;
    use crate::download::Downloader;
    use crate::args::Install;
    use crate::get_module::ModuleDownload;
    use crate::module::global_locations::GlobalLocations;
    use crate::module::location::github::Github;
    use crate::module::location::github::GithubDescriptor::Release;
    use crate::module::location::http::Http;
    use crate::module::location::{ConcreteLocation, Location};
    use crate::module::location::source::Source;
    use crate::module::weidu_mod::WeiduMod;
    use crate:: settings::Config;
    use crate::canon_path::CanonPath;
    use crate::cache::Cache;

    use anyhow::bail;
    use faux::when;

    /**
     * Checks that for a github mod, retrieve_location(...) returns whetever is returned by download(...).
     */
    #[tokio::test]
    async fn retrieve_github_location() {

        let location = ConcreteLocation {
            source: Source::Github(Github {
                github_user: "username".to_string(),
                repository: "repository".to_string(),
                descriptor: Release {
                    release: Some("V1".to_string()),
                    asset: "repository_v1.zip".to_string(),
                },
                ..Default::default()
            }),
            ..ConcreteLocation::default()
        };
        let module = WeiduMod {
            location: Some(Location::Concrete { concrete: location.clone() }),
            ..WeiduMod::default()
        };
        let global = Global::default();
        let global_locations = GlobalLocations::default();
        let opts = Install::default();
        let config = Config {
            archive_cache: Some("/cache_path".to_string()),
            extract_location: Some("/tmp".to_string()),
            weidu_path: None,
            extractors: HashMap::new(),
        };

        let expected_dest = PathBuf::from("/cache_path/github/username/repository");

        let game_dir = CanonPath::new("some_dir").unwrap();
        let cache = Cache::Path(PathBuf::from("/cache_path"));

        let mut downloader = Downloader::faux();
        when!(
            downloader.download(_, {expected_dest}, _, _)
        ).then(|(_, _, _, _)| Ok(PathBuf::from("cache_dir/directory/filename.zip")));
        when!(
            downloader.download_partial(_, _, _)
        ).then(|(_, _, _)| bail!("Should not be called"));
        when!(
            downloader.rename_partial(_, _)
        ).then(|(_, _)| bail!("Should not be called"));

        let module_download: ModuleDownload<'_> = ModuleDownload::new(&config, &global, &global_locations, &opts,
                                                                            &downloader, &game_dir, &cache);

        let result = module_download.retrieve_location(&location, &module.name);
        assert_eq!(
            result.await.unwrap(),
            PathBuf::from("cache_dir/directory/filename.zip")
        )
    }

    /**
     * Check http location.
     * Should be <cache_path>/http/<host_name>/<file_name>
    * */
    #[tokio::test]
    async fn retrieve_http_location() {

        let location = ConcreteLocation {
            source: Source::Http(Http {
                http: "http://example.com/some_mod.zip".to_string(),
                ..Default::default()
            }),
            ..ConcreteLocation::default()
        };
        let module = WeiduMod {
            location: Some(Location::Concrete { concrete: location.clone() }),
            ..WeiduMod::default()
        };
        let global = Global::default();
        let global_locations = GlobalLocations::default();
        let opts = Install::default();
        let config = Config {
            archive_cache: Some("/cache_path".to_string()),
            extract_location: Some("/tmp".to_string()),
            weidu_path: None,
            extractors: HashMap::new(),
        };

        let expected_dest = PathBuf::from("/cache_path/http/example.com");

        let game_dir = CanonPath::new("some_dir").unwrap();
        let cache = Cache::Path(PathBuf::from("/cache_path"));

        let mut downloader = Downloader::faux();
        when!(
            downloader.download(_, {expected_dest}, _, _)
        ).then(|(_, _, _, _)| Ok(PathBuf::from("/cache_path/http/example.com/some_mod.zip")));
        when!(
            downloader.download_partial(_, _, _)
        ).then(|(_, _, _)| bail!("Should not be called"));
        when!(
            downloader.rename_partial(_, _)
        ).then(|(_, _)| bail!("Should not be called"));

        let module_download: ModuleDownload<'_> = ModuleDownload::new(&config, &global, &global_locations, &opts,
                                                                            &downloader, &game_dir, &cache);

        let result = module_download.retrieve_location(&location, &module.name);
        assert_eq!(
            result.await.unwrap(),
            PathBuf::from("/cache_path/http/example.com/some_mod.zip")
        )
    }

    /**
     * Check absolute location.
     * Should just be the path in the location.
     */
    #[tokio::test]
    async fn retrieve_absolute_location() {
        let location = ConcreteLocation {
            source: Source::Absolute { path: "/some/path/file.zip".to_string() },
            ..ConcreteLocation::default()
        };
        let module = WeiduMod {
            location: Some(Location::Concrete { concrete: location.clone() }),
            ..WeiduMod::default()
        };
        let global = Global {
            local_mods: Some("my_mods".to_string()),
            ..Default::default()
        };
        let global_locations = GlobalLocations::default();
        let opts = Install {
            manifest_path: "/home/me/my_install.yaml".to_string(),
            ..Install::default()
        };
        let config = Config::default();

        let game_dir = CanonPath::new("some_dir").unwrap();
        let cache = Cache::Path(PathBuf::from("/cache_path"));

        let downloader = Downloader::faux();

        let module_download = ModuleDownload::new(&config, &global, &global_locations, &opts,
                                                                            &downloader, &game_dir, &cache);

        let result = module_download.retrieve_location(&location, &module.name,);
        assert_eq!(
            result.await.unwrap(),
            PathBuf::from("/some/path/file.zip")
        );
    }

    /**
     * Checks local mods.
     * Result should be <manifest_location>/<local_mods>/<mod_path>
     */
    #[tokio::test]
    async fn retrieve_local_location() {
        let location = ConcreteLocation {
            source: Source:: Local { local: "some/path/file.zip".to_string() },
            ..ConcreteLocation::default()
        };
        let module = WeiduMod {
            location: Some(Location::Concrete { concrete: location.clone() }),
            ..WeiduMod::default()
        };
        let global = Global {
            local_mods: Some("my_mods".to_string()),
            ..Default::default()
        };
        let global_locations = GlobalLocations::default();
        let opts = Install {
            manifest_path: "/home/me/my_install.yaml".to_string(),
            ..Install::default()
        };
        let config = Config::default();

        let game_dir = CanonPath::new("some_dir").unwrap();
        let cache = Cache::Path(PathBuf::from("/cache_path"));

        let downloader = Downloader::faux();

        let module_download = ModuleDownload::new(&config, &global, &global_locations, &opts,
                                                                            &downloader, &game_dir, &cache);

        let result = module_download.retrieve_location(&location, &module.name);
        assert_eq!(
            result.await.unwrap(),
            PathBuf::from("/home/me/my_mods/some/path/file.zip")
        );
    }
}
