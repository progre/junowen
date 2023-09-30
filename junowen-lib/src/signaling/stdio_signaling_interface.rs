use std::{io, process::exit};

use anyhow::Result;
use clipboard_win::set_clipboard_string;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::NamedPipeClient,
};

use crate::{lang::Lang, signaling::CompressedSessionDesc};

use super::socket::async_read_write_socket::{SignalingClientMessage, SignalingServerMessage};

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

fn offer_desc(lang: &Lang) -> CompressedSessionDesc {
    println!();
    CompressedSessionDesc(read_line_loop(lang, "Input host's signaling code:"))
}

fn print_offer_desc_and_get_answer_desc(
    lang: &Lang,
    offer_desc: CompressedSessionDesc,
) -> CompressedSessionDesc {
    println!();
    lang.println("Your signaling code:");
    println!();
    println!("{}", offer_desc.0);
    println!();
    set_clipboard_string(&offer_desc.0).unwrap();
    lang.println("It was copied to your clipboard. Share your signaling code with your guest.");
    let answer_desc = CompressedSessionDesc(read_line_loop(lang, "Input guest's signaling code:"));
    lang.println("Waiting for guest to connect...");
    answer_desc
}

fn print_answer_desc(lang: &Lang, answer_desc: CompressedSessionDesc) {
    println!();
    lang.println("Your signaling code:");
    println!();
    println!("{}", answer_desc.0);
    println!();
    set_clipboard_string(&answer_desc.0).unwrap();
    lang.println("It was copied to your clipboard. Share your signaling code with your host.");
    lang.println("Waiting for host to connect...");
}

async fn send(pipe: &mut NamedPipeClient, msg: SignalingServerMessage) -> Result<(), io::Error> {
    pipe.write_all(&rmp_serde::to_vec(&msg).unwrap()).await
}

async fn recv(pipe: &mut NamedPipeClient) -> Result<SignalingClientMessage, io::Error> {
    let mut buf = [0u8; 4 * 1024];
    let len = pipe.read(&mut buf).await?;
    Ok(rmp_serde::from_slice(&buf[..len]).unwrap())
}

pub async fn connect_as_offerer(
    client_pipe: &mut NamedPipeClient,
    lang: &Lang,
) -> Result<(), io::Error> {
    let SignalingClientMessage::OfferDesc(offer_desc) = recv(client_pipe).await? else {
        panic!("unexpected message");
    };

    let answer_desc = print_offer_desc_and_get_answer_desc(lang, offer_desc);
    send(
        client_pipe,
        SignalingServerMessage::SetAnswerDesc(answer_desc),
    )
    .await?;
    Ok(())
}

pub async fn connect_as_answerer(
    client_pipe: &mut NamedPipeClient,
    lang: &Lang,
) -> Result<(), io::Error> {
    let SignalingClientMessage::OfferDesc(_) = recv(client_pipe).await? else {
        panic!("unexpected message");
    };
    let offer_desc = offer_desc(lang);
    send(
        client_pipe,
        SignalingServerMessage::RequestAnswer(offer_desc),
    )
    .await?;
    let SignalingClientMessage::AnswerDesc(answer_desc) = recv(client_pipe).await? else {
        panic!("unexpected message");
    };
    print_answer_desc(lang, answer_desc);
    Ok(())
}
