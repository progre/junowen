use std::{io::ErrorKind, path::PathBuf};

use derive_new::new;
use tokio::{
    fs::{self, read_to_string},
    io,
};
use toml_edit::{Formatted, Item, Value};
use tracing::error;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HANDLE, HMODULE},
        System::LibraryLoader::GetModuleFileNameW,
        UI::Shell::{FOLDERID_RoamingAppData, SHGetKnownFolderPath, KNOWN_FOLDER_FLAG},
    },
};

pub async fn ini_file_path_log_dir_path_log_file_name_old_log_path(
    module: HMODULE,
) -> (String, String, String, String) {
    let dll_path = {
        let mut buf = [0u16; u16::MAX as usize];
        if unsafe { GetModuleFileNameW(module, &mut buf) } == 0 {
            panic!();
        }
        let dll_path = unsafe { PCWSTR::from_raw(buf.as_ptr()).to_string() }.unwrap();
        PathBuf::from(dll_path)
    };

    let module_dir = {
        let guid = FOLDERID_RoamingAppData;
        let res = unsafe { SHGetKnownFolderPath(&guid, KNOWN_FOLDER_FLAG(0), HANDLE::default()) };
        let app_data_dir = unsafe { res.unwrap().to_string() }.unwrap();
        format!("{}/ShanghaiAlice/th19/modules", app_data_dir)
    };

    let dll_stem = dll_path.file_stem().unwrap().to_string_lossy();
    let ini_file_path = format!("{}/{}.ini", module_dir, dll_stem);
    let log_file_name = format!("{}.log", dll_stem);
    let dll_dir_path = dll_path.parent().unwrap().to_string_lossy();
    let old_log_path = format!("{}/{}", dll_dir_path, log_file_name);

    (ini_file_path, module_dir, log_file_name, old_log_path)
}

pub async fn move_old_log_to_new_path(old_log_path: &str, module_dir: &str, log_file_name: &str) {
    let new_log_path = format!("{}/{}", module_dir, log_file_name);
    if let Err(err) = (async {
        let result = fs::OpenOptions::new().read(true).open(old_log_path).await;
        let mut old_file = match result {
            Ok(file) => file,
            Err(err) => {
                if err.kind() != ErrorKind::NotFound {
                    return Err(err);
                }
                return Ok(());
            }
        };
        let result = fs::OpenOptions::new().write(true).open(&new_log_path).await;
        let mut new_file = result?;
        if new_file.metadata().await?.len() > 0 {
            return Err(io::Error::new(
                ErrorKind::AlreadyExists,
                format!("{} already exists", new_log_path),
            ));
        }
        io::copy(&mut old_file, &mut new_file).await?;
        fs::remove_file(old_log_path).await?;
        Ok(())
    })
    .await
    {
        error!(
            "Failed to mv {} {} Reason: {}",
            old_log_path, new_log_path, err
        );
    }
}

#[derive(new)]
pub struct SettingsRepo {
    path: String,
}

impl SettingsRepo {
    async fn load(&self) -> toml_edit::Document {
        read_to_string(&self.path)
            .await
            .unwrap_or_default()
            .parse()
            .unwrap_or_default()
    }

    async fn read_string(&self, key: &str) -> Option<String> {
        self.load()
            .await
            .get(key)
            .and_then(|x| x.as_str())
            .map(|x| x.to_owned())
    }

    async fn write_string(&self, key: &str, value: String) {
        let mut doc = self.load().await;
        if let Some(item) = doc.as_table_mut().get_mut(key) {
            *item = Item::Value(Value::String(Formatted::new(value)));
        } else {
            let _ = doc.insert(key, Item::Value(Value::String(Formatted::new(value))));
        }
        doc.sort_values();
        if let Err(err) = tokio::fs::write(&self.path, doc.to_string()).await {
            error!("{}", err);
        }
    }
}
