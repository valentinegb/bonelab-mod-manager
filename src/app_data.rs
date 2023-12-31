#[cfg(target_os = "windows")]
use std::fmt::{self, Display, Formatter};
use std::{
    collections::HashMap,
    env::{self, VarError},
    ffi::OsString,
    path::PathBuf,
};

#[cfg(target_os = "windows")]
use anyhow::anyhow;
use anyhow::Result;
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct AppData {
    #[cfg(target_os = "windows")]
    pub(crate) modio_token: Option<String>,
    #[cfg(target_os = "windows")]
    pub(crate) platform: Option<BonelabPlatform>,
    pub(crate) installed_mods: HashMap<u64, InstalledMod>,
}

#[cfg(target_os = "windows")]
#[derive(Serialize, Deserialize)]
pub(crate) enum BonelabPlatform {
    Windows,
    Quest,
}

#[cfg(target_os = "windows")]
impl TryFrom<usize> for BonelabPlatform {
    type Error = anyhow::Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Windows),
            1 => Ok(Self::Quest),
            other => Err(anyhow!(
                "BonelabPlatform only accepts values equal to 0 or 1, got {other}"
            )),
        }
    }
}

#[cfg(target_os = "windows")]
impl Display for BonelabPlatform {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Windows => "Windows",
                Self::Quest => "Quest",
            }
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct InstalledMod {
    pub(crate) date_updated: u64,
    pub(crate) folder: OsString,
}

impl AppData {
    #[cfg(target_os = "macos")]
    const REL_DIR_PATH: &str = "Library/Application Support/com.valentinegb.bonelab_mod_manager";
    #[cfg(target_os = "linux")]
    const REL_DIR_PATH: &str = "var/lib/bonelab_mod_manager";
    #[cfg(target_os = "windows")]
    const REL_DIR_PATH: &str = "bonelab_mod_manager";

    #[cfg(target_family = "unix")]
    fn dir_path() -> Result<PathBuf, VarError> {
        debug!("getting app data dir path");

        Ok(PathBuf::from(env::var("HOME")?).join(Self::REL_DIR_PATH))
    }

    #[cfg(target_os = "windows")]
    fn dir_path() -> Result<PathBuf, VarError> {
        debug!("getting app data dir path");

        Ok(PathBuf::from(env::var("AppData")?).join(Self::REL_DIR_PATH))
    }

    fn path() -> Result<PathBuf, VarError> {
        debug!("getting app data path");

        Ok(Self::dir_path()?.join("app_data"))
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn mods_dir_path(&self) -> Result<PathBuf> {
        debug!("getting mods dir path");

        match self
            .platform
            .as_ref()
            .ok_or(anyhow!("Platform is not set"))?
        {
            BonelabPlatform::Windows => Ok(PathBuf::from(env::var("AppData")?)
                .parent()
                .ok_or(anyhow!("AppData env var value does not have parent"))?
                .join("Locallow/Stress Level Zero/Bonelab/Mods")),
            BonelabPlatform::Quest => Ok(Self::dir_path()?.join("Mods")),
        }
    }

    #[cfg(target_family = "unix")]
    pub(crate) fn mods_dir_path(&self) -> Result<PathBuf> {
        debug!("getting mods dir path");

        Ok(Self::dir_path()?.join("Mods"))
    }

    async fn write_default() -> Result<Self> {
        let default = Self::default();

        default.write().await?;
        debug!("wrote default app data");

        Ok(default)
    }

    pub(crate) async fn read() -> Result<Self> {
        let path = Self::path()?;

        if !fs::try_exists(&path).await? {
            return Ok(Self::write_default().await?);
        }

        let app_data = postcard::from_bytes(&fs::read(path).await?);

        debug!("deserialized app data");

        if app_data
            .as_ref()
            .is_err_and(|err| *err == postcard::Error::DeserializeUnexpectedEnd)
            || app_data
                .as_ref()
                .is_err_and(|err| *err == postcard::Error::SerdeDeCustom)
        {
            debug!("app data is borked, resetting");

            return Ok(Self::write_default().await?);
        }

        debug!("read app data");

        Ok(app_data?)
    }

    pub(crate) async fn write(&self) -> Result<()> {
        let path = Self::path()?;

        if !fs::try_exists(&path).await? {
            debug!("app data dir does not exist");
            fs::create_dir_all(Self::dir_path()?).await?;
            debug!("created app data dir");
        }

        fs::write(path, postcard::to_stdvec(self)?).await?;
        debug!("wrote app data");

        Ok(())
    }
}
