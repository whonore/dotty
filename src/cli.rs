use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

use crate::path::expand_env;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
struct AppsWrap(HashMap<String, AppConfig>);

#[derive(Debug, Deserialize)]
struct AppConfig {
    srcdir: Option<String>,
    dstdir: Option<String>,
    description: Option<String>,
    links: Option<Vec<Vec<String>>>,
}

#[derive(Debug)]
pub struct App {
    pub srcdir: PathBuf,
    pub dstdir: PathBuf,
    pub description: Option<String>,
    pub links: Vec<(String, String)>,
}

impl App {
    fn new<P: AsRef<Path>>(base_dir: P, name: &str, app: AppConfig) -> Result<Self> {
        Ok(App {
            srcdir: app
                .srcdir
                .as_ref()
                .map(expand_env)
                .unwrap_or_else(|| Ok(dirs::home_dir().unwrap()))?,
            dstdir: expand_env(
                &base_dir
                    .as_ref()
                    .join(app.dstdir.as_deref().unwrap_or(name)),
            )?,
            description: app.description,
            links: app
                .links
                .map(|links| {
                    links
                        .into_iter()
                        .map(|link| {
                            App::normalize_link(link)
                                .ok_or_else(|| anyhow!("{}: links must be length 1 or 2", name))
                        })
                        .collect()
                })
                .unwrap_or_else(|| Ok(vec![]))?,
        })
    }

    fn normalize_link(mut link: Vec<String>) -> Option<(String, String)> {
        let mut x = link.drain(..);
        let first = x.next()?;
        let second = x.next().unwrap_or_else(|| first.clone());
        if x.next().is_none() {
            Some((first, second))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Apps(pub HashMap<String, App>);

impl Apps {
    fn new<P: AsRef<Path>>(base_dir: P, apps: HashMap<String, AppConfig>) -> Result<Apps> {
        let apps = apps
            .into_iter()
            .map(|(name, app)| {
                let app = App::new(&base_dir, &name, app)?;
                Ok((name, app))
            })
            .collect::<Result<_>>()?;
        Ok(Apps(apps))
    }

    pub fn dir(&self, name: &str) -> Option<PathBuf> {
        self.0.get(name).map(|app| app.dstdir.clone())
    }
}

#[derive(Debug, StructOpt)]
#[structopt(about)]
pub struct Cli {
    #[structopt(parse(from_os_str))]
    base_dir: Option<PathBuf>,
    #[structopt(short = "c", long = "config-file", parse(from_os_str))]
    config_file: Option<PathBuf>,
    #[structopt(short = "L", long = "link")]
    link: bool,
    #[structopt(short = "a", long = "include-app")]
    include_apps: Option<Vec<String>>,
    #[structopt(short = "A", long = "exclude-app")]
    exclude_apps: Option<Vec<String>>,
}

fn find_config<P: AsRef<Path>>(base_dir: P) -> PathBuf {
    base_dir.as_ref().join("peridot.toml")
}

#[derive(Debug)]
pub struct Config {
    pub base_dir: PathBuf,
    pub apps: Apps,
    pub link: bool,
}

impl Config {
    pub fn new(args: Cli) -> Result<Config> {
        let base_dir = args
            .base_dir
            .unwrap_or_else(|| dirs::home_dir().unwrap().join(".dotfiles"))
            .canonicalize()?;
        let config_file = args
            .config_file
            .unwrap_or_else(|| find_config(&base_dir))
            .canonicalize()?;
        let mut apps: AppsWrap = toml::from_str(&fs::read_to_string(&config_file)?)?;

        if let Some(f) = Config::app_filter(args.include_apps, args.exclude_apps) {
            apps.0 = apps.0.into_iter().filter(|(name, _)| f(name)).collect();
        }

        Ok(Config {
            apps: Apps::new(&base_dir, apps.0)?,
            base_dir,
            link: args.link,
        })
    }

    fn app_filter(
        incl: Option<Vec<String>>,
        excl: Option<Vec<String>>,
    ) -> Option<Box<dyn Fn(&String) -> bool>> {
        match (incl, excl) {
            (Some(incl), Some(excl)) => {
                Some(Box::new(move |x| incl.contains(x) && !excl.contains(x)))
            }
            (Some(incl), None) => Some(Box::new(move |x| incl.contains(x))),
            (None, Some(excl)) => Some(Box::new(move |x| !excl.contains(x))),
            (None, None) => None,
        }
    }
}
