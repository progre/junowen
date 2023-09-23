use std::{io, process::exit};

use anyhow::Result;
use async_trait::async_trait;
use clipboard_win::set_clipboard_string;

use crate::{lang::Lang, signaling::CompressedSessionDesc};

use super::PeerConnection;

fn read_line() -> String {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap_or_else(|_| exit(1));
    buf.trim().to_owned()
}

fn read_line_loop(lang: &Lang, msg: &str) -> String {
    loop {
        lang.println(msg);
        let buf = read_line();
        if !buf.trim().is_empty() {
            break buf;
        }
    }
}

pub struct StdioMockConnection<'a> {
    lang: &'a Lang,
}

impl<'a> StdioMockConnection<'a> {
    pub fn new(lang: &'a Lang) -> Self {
        Self { lang }
    }
}

#[async_trait]
impl<'a> PeerConnection for StdioMockConnection<'a> {
    async fn start_as_host(&mut self) -> Result<CompressedSessionDesc> {
        println!();
        Ok(CompressedSessionDesc(read_line_loop(
            self.lang,
            "Input host's signaling code:",
        )))
    }

    async fn start_as_guest(
        &mut self,
        offer_desc: CompressedSessionDesc,
    ) -> Result<CompressedSessionDesc> {
        println!();
        self.lang.println("Your signaling code:");
        println!();
        println!("{}", offer_desc.0);
        println!();
        set_clipboard_string(&offer_desc.0).unwrap();
        self.lang
            .println("It was copied to your clipboard. Share your signaling code with your guest.");
        let answer_desc =
            CompressedSessionDesc(read_line_loop(self.lang, "Input guest's signaling code:"));
        self.lang.println("Waiting for guest to connect...");
        Ok(answer_desc)
    }

    async fn set_answer_desc(&self, answer_desc: CompressedSessionDesc) -> Result<()> {
        println!();
        self.lang.println("Your signaling code:");
        println!();
        println!("{}", answer_desc.0);
        println!();
        set_clipboard_string(&answer_desc.0).unwrap();
        self.lang
            .println("It was copied to your clipboard. Share your signaling code with your host.");
        self.lang.println("Waiting for host to connect...");
        Ok(())
    }
}
