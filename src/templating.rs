use std::{env, path::PathBuf};

use handlebars::Handlebars;
use handlebars_switch::SwitchHelper;
use lazy_static::lazy_static;
use miette::{Context, IntoDiagnostic, Result};
use serde::Serialize;

pub fn render<T: Serialize>(template: &str, ctx: T) -> Result<String> {
    engine()
        .render_template(template, &context(ctx))
        .into_diagnostic()
        .context("rendering to path")
}

fn engine<'a>() -> Handlebars<'a> {
    let mut hb = Handlebars::new();
    hb.register_helper("switch", Box::new(SwitchHelper));
    hb
}

pub fn context<'a, T: Serialize>(ctx: T) -> WrappedContext<'a, T> {
    lazy_static! {
        static ref CTX: ContextData = ContextData::default();
    }
    WrappedContext { data: &*CTX, ctx }
}

#[derive(Serialize)]
pub struct WrappedContext<'a, T: Serialize> {
    #[serde(flatten)]
    data: &'a ContextData,
    ctx: T,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct ContextData {
    pub dirs: ContextDirs,
    pub system: SystemData,
    pub flags: Flags,
}

#[derive(Clone, Debug, Serialize)]
pub struct SystemData {
    pub os: String,
    pub family: String,
    pub os_release: String,
    pub arch: String,
    pub hostname: String,
    pub cpu_num: u32,
}

impl Default for SystemData {
    fn default() -> Self {
        Self {
            os: env::consts::OS.into(),
            family: env::consts::FAMILY.into(),
            arch: env::consts::ARCH.into(),
            hostname: sys_info::hostname().unwrap(),
            cpu_num: sys_info::cpu_num().unwrap(),
            os_release: sys_info::os_release().unwrap(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ContextDirs {
    pub home: PathBuf,
    pub config: PathBuf,
    pub data: PathBuf,
    pub global_config: PathBuf,
    pub global_data: PathBuf,
}

impl Default for ContextDirs {
    fn default() -> Self {
        Self {
            home: dirs::home_dir().unwrap_or_else(|| PathBuf::from("~")),
            config: dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config")),
            data: dirs::data_dir().unwrap_or_else(|| PathBuf::from("~/.local/share")),
            global_config: if cfg!(windows) {
                PathBuf::from("~/AppData/Roaming")
            } else {
                PathBuf::from("/etc")
            },
            global_data: if cfg!(windows) {
                PathBuf::from("~/AppData/Local")
            } else {
                PathBuf::from("/usr/share")
            },
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Flags {
    windows: bool,
    unix: bool,
    linux: bool,
    macos: bool,
}

impl Default for Flags {
    fn default() -> Self {
        Self {
            windows: cfg!(windows),
            unix: cfg!(unix),
            linux: cfg!(target_os = "linux"),
            macos: cfg!(target_os = "macos"),
        }
    }
}
