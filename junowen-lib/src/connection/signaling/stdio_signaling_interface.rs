use std::{io, process::exit};

use anyhow::Result;
use clipboard_win::set_clipboard_string;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::NamedPipeClient,
};

use crate::{connection::signaling::SignalingCodeType, lang::Lang};

use super::{
    socket::async_read_write_socket::{SignalingClientMessage, SignalingServerMessage},
    CompressedSdp,
};

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

fn offer_desc(lang: &Lang) -> CompressedSdp {
    CompressedSdp(read_line_loop(lang, "Input host's signaling code:"))
}

fn print_offer_desc_and_get_answer_desc(lang: &Lang, offer_desc: CompressedSdp) -> CompressedSdp {
    lang.println("Your signaling code:");
    let offer_str = SignalingCodeType::BattleOffer.to_string(&offer_desc);
    println!();
    println!("{}", offer_str);
    println!();
    set_clipboard_string(&offer_str).unwrap();
    lang.println("It was copied to your clipboard. Share your signaling code with your guest.");
    println!();
    let answer_desc = CompressedSdp(read_line_loop(lang, "Input guest's signaling code:"));
    lang.println("Waiting for guest to connect...");
    answer_desc
}

fn print_answer_desc(lang: &Lang, answer_desc: CompressedSdp) {
    lang.println("Your signaling code:");
    let answer_str = SignalingCodeType::BattleAnswer.to_string(&answer_desc);
    println!();
    println!("{}", answer_str);
    println!();
    set_clipboard_string(&answer_str).unwrap();
    lang.println("It was copied to your clipboard. Share your signaling code with your host.");
    println!();
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
