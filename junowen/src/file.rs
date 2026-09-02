use std::path::PathBuf;

use derive_new::new;
use junowen_lib::Th19;
use serde::Deserialize;
use tokio::fs::read_to_string;
use toml_edit::{Formatted, Item, Value};
use tracing::error;
use windows::{
    Win32::{
        Foundation::{HMODULE, MAX_PATH},
        System::LibraryLoader::GetModuleFileNameW,
        UI::Shell::{FOLDERID_RoamingAppData, KNOWN_FOLDER_FLAG, SHGetKnownFolderPath},
    },
    core::PCWSTR,
};

pub fn to_dll_path(module: HMODULE) -> PathBuf {
    let mut buf = [0u16; MAX_PATH as usize];
    if unsafe { GetModuleFileNameW(Some(module), &mut buf) } == 0 {
        panic!();
    }
    let dll_path = unsafe { PCWSTR::from_raw(buf.as_ptr()).to_string() }.unwrap();
    PathBuf::from(dll_path)
}

pub fn to_ini_file_path_log_dir_path_log_file_name(dll_stem: &str) -> (String, String, String) {
    let module_dir = {
        let guid = FOLDERID_RoamingAppData;
        let res = unsafe { SHGetKnownFolderPath(&guid, KNOWN_FOLDER_FLAG(0), None) };
        let app_data_dir = unsafe { res.unwrap().to_string() }.unwrap();
        format!("{}/ShanghaiAlice/th19/modules", app_data_dir)
    };

    let ini_file_path = format!("{}/{}.ini", module_dir, dll_stem);
    let log_file_name = format!("{}.log", dll_stem);

    (ini_file_path, module_dir, log_file_name)
}

#[derive(Debug, Deserialize, PartialEq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Features {
    ShowSettings,
}

const FEATURES: &str = "features";
const LAN_ADDRESS: &str = "lan_address";
const LAN_OFFLINE: &str = "lan_offline";
const SHARED_ROOM_NAME: &str = "shared_room_name";
const RESERVED_ROOM_NAME: &str = "reserved_room_name";

const DEFAULT_LAN_ADDRESS: &str = "127.0.0.1:19000";
/// STUN サーバーが到達できない環境でも待ち時間なく接続できるよう、既定はオフライン(STUN 無効)とする
const DEFAULT_LAN_OFFLINE: bool = true;

#[derive(new)]
pub struct SettingsRepo {
    path: String,
}

impl SettingsRepo {
    async fn load(&self) -> toml_edit::DocumentMut {
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

    async fn read_bool(&self, key: &str) -> Option<bool> {
        self.load().await.get(key).and_then(|x| x.as_bool())
    }

    async fn write_bool(&self, key: &str, value: bool) {
        let mut doc = self.load().await;
        if let Some(item) = doc.as_table_mut().get_mut(key) {
            *item = Item::Value(Value::Boolean(Formatted::new(value)));
        } else {
            let _ = doc.insert(key, Item::Value(Value::Boolean(Formatted::new(value))));
        }
        doc.sort_values();
        if let Err(err) = tokio::fs::write(&self.path, doc.to_string()).await {
            error!("{}", err);
        }
    }

    pub async fn features(&self) -> Vec<Features> {
        self.load()
            .await
            .get(FEATURES)
            .and_then(|x| x.as_array())
            .map(|x| {
                x.iter()
                    .flat_map(|x| x.as_str())
                    .flat_map(|x| serde_json::from_str(&format!("\"{x}\"")).ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    pub async fn lan_address(&self) -> String {
        match self.read_string(LAN_ADDRESS).await {
            Some(value) => value,
            None => {
                let value = DEFAULT_LAN_ADDRESS.to_owned();
                self.set_lan_address(value.clone()).await;
                value
            }
        }
    }
    pub async fn set_lan_address(&self, value: String) {
        self.write_string(LAN_ADDRESS, value).await;
    }

    pub async fn lan_offline(&self) -> bool {
        match self.read_bool(LAN_OFFLINE).await {
            Some(value) => value,
            None => {
                self.set_lan_offline(DEFAULT_LAN_OFFLINE).await;
                DEFAULT_LAN_OFFLINE
            }
        }
    }
    pub async fn set_lan_offline(&self, value: bool) {
        self.write_bool(LAN_OFFLINE, value).await;
    }

    pub async fn reserved_room_name(&self, th19: &Th19) -> String {
        match self.read_string(RESERVED_ROOM_NAME).await {
            Some(value) => value,
            None => {
                let value = th19.vs_mode().room_name().to_owned();
                self.set_reserved_room_name(value.clone()).await;
                value
            }
        }
    }
    pub async fn set_reserved_room_name(&self, value: String) {
        self.write_string(RESERVED_ROOM_NAME, value).await;
    }

    pub async fn shared_room_name(&self, th19: &Th19) -> String {
        match self.read_string(SHARED_ROOM_NAME).await {
            Some(value) => value,
            None => {
                let value = th19.vs_mode().room_name().to_owned();
                self.set_shared_room_name(value.clone()).await;
                value
            }
        }
    }
    pub async fn set_shared_room_name(&self, value: String) {
        self.write_string(SHARED_ROOM_NAME, value).await;
    }
}
