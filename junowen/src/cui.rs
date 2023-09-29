use std::{
    env::current_exe,
    io::{self, Write},
    process::exit,
    time::Duration,
};

use anyhow::{bail, Result};
use junowen_lib::{inject_dll::inject_dll, lang::Lang};
use tokio::net::windows::named_pipe;
use tokio::{net::windows::named_pipe::NamedPipeClient, time::sleep};

async fn create_pipe(lang: &Lang) -> Result<NamedPipeClient> {
    let name = r"\\.\pipe\junowen";
    let named_pipe_client_option = named_pipe::ClientOptions::new();

    let pipe = if let Ok(pipe) = named_pipe_client_option.open(name) {
        pipe
    } else {
        let dll_path = current_exe()
            .unwrap()
            .as_path()
            .parent()
            .unwrap()
            .join(concat!(env!("CARGO_PKG_NAME"), "_hook.dll"));

        inject_dll("th19.exe", &dll_path)?;
        loop {
            if let Ok(ok) = named_pipe_client_option.open(name) {
                lang.println("Module loaded by th19.");
                break ok;
            };
            sleep(Duration::from_secs(1)).await;
        }
    };
    Ok(pipe)
}

fn read_line() -> String {
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap_or_else(|_| exit(1));
    buf.trim().to_owned()
}

async fn host(_pipe: &mut NamedPipeClient, _lang: &Lang) -> Result<()> {
    bail!("Not implemented")
}

async fn guest(_pipe: &mut NamedPipeClient, _lang: &Lang) -> Result<()> {
    bail!("Not implemented")
}

pub async fn main_menu(lang: &Lang) -> Result<()> {
    loop {
        println!();
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        println!();
        lang.println("1) Connect as Host");
        lang.println("2) Connect as Guest");
        println!();
        lang.println("0) Exit");
        println!();

        let select = loop {
            lang.print("Select (0-2): ");
            let buf = read_line();
            let Ok(select) = buf.trim().parse::<u8>() else {
                continue;
            };
            if !(0..=2).contains(&select) {
                continue;
            }
            break select;
        };

        if select == 0 {
            break;
        }

        let mut pipe = match create_pipe(lang).await {
            Ok(ok) => ok,
            Err(err) => {
                lang.print("Hook module not found: ");
                println!("{}", err);
                println!();
                continue;
            }
        };

        match select {
            1 => {
                if let Err(err) = host(&mut pipe, lang).await {
                    lang.print("th19 disconnected: ");
                    println!("{}", err);
                    println!();
                }
            }
            2 => {
                if let Err(err) = guest(&mut pipe, lang).await {
                    lang.print("th19 disconnected: ");
                    println!("{}", err);
                    println!();
                }
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}
