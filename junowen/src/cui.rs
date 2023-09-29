use std::{
    io::{self, Write},
    process::exit,
};

use anyhow::Result;
use junowen_lib::lang::Lang;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub enum IpcMessageToHook {
    StartAsHost,
    StartAsGuest,
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

pub async fn main_menu(lang: &Lang) -> Result<()> {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    loop {
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

        todo!();
    }
    Ok(())
}
