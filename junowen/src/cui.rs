use std::{
    env::current_exe,
    io::{self, Write},
    process::exit,
    time::Duration,
};

use anyhow::{anyhow, bail, Result};
use junowen_lib::hook_utils::InjectDllError;
use junowen_lib::{hook_utils::inject_dll, lang::Lang};
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncReadExt, net::windows::named_pipe};
use tokio::{net::windows::named_pipe::NamedPipeClient, time::sleep};
use tracing::trace;

async fn check_version(pipe: &mut NamedPipeClient) -> Result<()> {
    let mut buf = [0u8; 4 * 1024];
    let len = pipe.read(&mut buf).await?;
    let msg: IpcMessageToCui = rmp_serde::from_slice(&buf[..len])
        .map_err(|err| anyhow!("parse failed (len={}): {}", len, err))?;
    let IpcMessageToCui::Version(version) = msg else {
        bail!("Unexpected message");
    };
    if version != env!("CARGO_PKG_VERSION") {
        bail!("Version mismatch");
    }
    Ok(())
}

async fn create_pipe(lang: &Lang) -> Option<NamedPipeClient> {
    let name = r"\\.\pipe\junowen";
    let named_pipe_client_option = named_pipe::ClientOptions::new();

    trace!("named pipe opening...");
    let mut pipe = if let Ok(pipe) = named_pipe_client_option.open(name) {
        trace!("named pipe opened");
        lang.println("Module already loaded by th19.exe.");
        pipe
    } else {
        trace!("named pipe opening failed");
        let dll_path = current_exe()
            .unwrap()
            .as_path()
            .parent()
            .unwrap()
            .join(concat!(env!("CARGO_PKG_NAME"), "_hook.dll"));

        let mut retry = false;
        loop {
            let Err(err) = inject_dll("th19.exe", &dll_path) else {
                break;
            };
            match err {
                InjectDllError::DllNotFound => {
                    lang.println("junowen_hook.dll not found.");
                    return None;
                }
                InjectDllError::ProcessNotFound(err) => {
                    if !retry {
                        retry = true;
                        lang.print("th19.exe process not found: ");
                        println!("{}", err);
                        lang.println("waiting for th19.exe process...");
                    }
                    trace!("waiting for process...");
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
        loop {
            if let Ok(ok) = named_pipe_client_option.open(name) {
                lang.println("Module loaded by th19.exe.");
                println!();
                break ok;
            };
            trace!("waiting for inject...");
            sleep(Duration::from_secs(1)).await;
        }
    };
    if let Err(err) = check_version(&mut pipe).await {
        lang.print("Failed to communicate with junowen_hook.dll: ");
        println!("{}", err);
        println!();
        return None;
    }
    Some(pipe)
}

#[derive(Clone, Deserialize, Serialize)]
pub enum IpcMessageToHook {
    Delay(u8),
}

#[derive(Deserialize, Serialize)]
pub enum IpcMessageToCui {
    Version(String),
    Connected,
    Disconnected,
}

fn read_line() -> String {
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap_or_else(|_| exit(1));
    buf.trim().to_owned()
}

#[allow(unused)]
pub async fn main_menu(lang: &Lang) -> Result<()> {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!();

    let _ = create_pipe(lang).await;
    lang.print("Press enter to exit...");
    read_line();
    Ok(())
}
