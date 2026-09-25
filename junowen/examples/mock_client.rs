//! ゲームを起動せずに対戦相手や観戦者として振る舞う、テスト用のクライアント
//!
//! ```sh
//! cargo run -p junowen --example mock_client -- <host|guest|spectator> [--name <NAME>]
//! ```
//!
//! - `host`: Pure P2P の「Connect as a Host」として振る舞う。ゲーム側は「Connect as a Guest」で接続する
//! - `guest`: Pure P2P の「Connect as a Guest」として振る舞う。ゲーム側は「Connect as a Host」で接続する
//! - `spectator`: Pure P2P の「Connect as a Spectator」として振る舞う。ゲーム側は対戦中に F1 で受け入れる
//!
//! 対戦者は、受け取った相手の入力をそのまま自分の入力として送り返す(ミラー入力)。
//! ホストとして送る試合設定は `GameSettings` の既定値で、乱数シードはラウンドごとに現在時刻から作る。
//!
//! シグナリングコードはクリップボードにコピーする。相手のシグナリングコードは、
//! 待ち始めた後にクリップボードにコピーされたものか、標準入力に貼り付けたものを使う。

// junowen は cdylib なので直接参照できない。`#[path]` で session.rs を直接読み込むと
// 子モジュールが src/ 直下から探されるため、inline モジュールで囲んで src/session/ から読み込ませる
#[path = "../src"]
mod app {
    #[allow(dead_code)]
    pub mod session;
}

use std::{
    env, io,
    sync::mpsc::{self, TryRecvError},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Result, bail};
use bytes::Bytes;
use clipboard_win::{get_clipboard_string, seq_num, set_clipboard_string};
use junowen_lib::{
    connection::{
        BATTLE_PROTOCOL, DataChannel, PeerConnection, SPECTATOR_PROTOCOL,
        signaling::{
            CompressedSdp, SignalingCodeType, parse_signaling_code, socket::DEFAULT_STUN_SERVER_URL,
        },
    },
    structs::settings::GameSettings,
};
use serde::Serialize;
use tracing_subscriber::EnvFilter;

use app::session::{
    MatchInitial, RoundInitial, session_message::SessionMessage, spectator::SpectatorSessionMessage,
};

const TIMEOUT: Duration = Duration::from_secs(20 * 60);
/// 試合中に経過を表示する間隔(フレーム数)
const PROGRESS_INTERVAL: u32 = 600;

#[derive(Clone, Copy)]
enum Role {
    Host,
    Guest,
    Spectator,
}

struct Args {
    role: Role,
    name: String,
}

fn parse_args() -> Result<Args> {
    let mut role = None;
    let mut name = "Mock".to_owned();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "host" => role = Some(Role::Host),
            "guest" => role = Some(Role::Guest),
            "spectator" => role = Some(Role::Spectator),
            "--name" => {
                let Some(value) = args.next() else {
                    bail!("--name requires a value");
                };
                name = value;
            }
            _ => bail!("unknown argument: {}", arg),
        }
    }
    let Some(role) = role else {
        bail!("usage: mock_client <host|guest|spectator> [--name <NAME>]");
    };
    Ok(Args { role, name })
}

fn spawn_stdin_reader() -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for line in io::stdin().lines() {
            let Ok(line) = line else {
                return;
            };
            if tx.send(line).is_err() {
                return;
            }
        }
    });
    rx
}

fn print_and_copy_code(code_type: SignalingCodeType, desc: &CompressedSdp) {
    let code = code_type.to_string(desc);
    println!();
    println!("{}", code);
    println!();
    match set_clipboard_string(&code) {
        Ok(()) => println!("Your signaling code has been copied to the clipboard."),
        Err(err) => println!("Failed to copy to the clipboard: {}", err),
    }
}

/// 相手のシグナリングコードを、クリップボードか標準入力から受け取る
///
/// 前回の接続のコードを使わないように、待ち始めた後にコピーされたものだけを使う
async fn wait_for_code(
    code_type: SignalingCodeType,
    stdin_rx: &mpsc::Receiver<String>,
) -> Result<CompressedSdp> {
    println!("Waiting for the opponent's signaling code (copy it or paste it here)...");
    let mut clipboard_seq_num = seq_num();
    loop {
        match stdin_rx.try_recv() {
            Ok(line) if !line.trim().is_empty() => match parse_signaling_code(&line) {
                Ok((t, desc)) if t == code_type => return Ok(desc),
                Ok(_) => println!("Unexpected type of signaling code."),
                Err(err) => println!("Invalid signaling code: {}", err),
            },
            Ok(_) | Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => bail!("stdin closed"),
        }
        let current_seq_num = seq_num();
        if current_seq_num != clipboard_seq_num {
            clipboard_seq_num = current_seq_num;
            if let Ok(clipboard) = get_clipboard_string()
                && let Ok((t, desc)) = parse_signaling_code(&clipboard)
                && t == code_type
            {
                return Ok(desc);
            }
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

async fn connect_as_offerer(
    offer_type: SignalingCodeType,
    answer_type: SignalingCodeType,
    protocol: &'static str,
    stdin_rx: &mpsc::Receiver<String>,
) -> Result<(PeerConnection, DataChannel)> {
    let mut conn =
        PeerConnection::new(TIMEOUT, vec![DEFAULT_STUN_SERVER_URL.to_owned()], protocol).await?;
    let offer = conn.start_as_offerer().await?;
    print_and_copy_code(offer_type, &offer);
    let answer = wait_for_code(answer_type, stdin_rx).await?;
    conn.set_answer_desc(answer).await?;
    println!("Waiting for connection...");
    let data_channel = conn.wait_for_open_data_channel().await?;
    Ok((conn, data_channel))
}

async fn connect_as_answerer(
    offer_type: SignalingCodeType,
    answer_type: SignalingCodeType,
    protocol: &'static str,
    stdin_rx: &mpsc::Receiver<String>,
) -> Result<(PeerConnection, DataChannel)> {
    let offer = wait_for_code(offer_type, stdin_rx).await?;
    let mut conn =
        PeerConnection::new(TIMEOUT, vec![DEFAULT_STUN_SERVER_URL.to_owned()], protocol).await?;
    let answer = conn.start_as_answerer(offer).await?;
    print_and_copy_code(answer_type, &answer);
    println!("Waiting for connection...");
    let data_channel = conn.wait_for_open_data_channel().await?;
    Ok((conn, data_channel))
}

fn random_round_initial() -> RoundInitial {
    let mut x = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos()
        | 1;
    let mut next = || {
        // xorshift32
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        x
    };
    RoundInitial {
        seed1: next(),
        seed2: next(),
        seed3: next(),
        seed4: next(),
    }
}

async fn send<T: Serialize>(data_channel: &DataChannel, msg: &T) -> Result<()> {
    let data = Bytes::from(rmp_serde::to_vec(msg)?);
    data_channel.message_sender.send(data).await?;
    Ok(())
}

/// 相手のメッセージに応答して対戦を進める
///
/// 相手の入力をそのまま送り返すので、両プレイヤーは同じフレームに同じ入力をする
async fn run_battle(mut data_channel: DataChannel, host: bool, name: &str) -> Result<()> {
    let mut frames = 0;
    while let Some(data) = data_channel.recv().await {
        let reply = match rmp_serde::from_slice(&data)? {
            SessionMessage::InitMatch((remote_name, init)) => {
                println!("InitMatch: remote={:?}, initial={:?}", remote_name, init);
                if host == init.is_some() {
                    println!("warning: the opponent has the same role");
                }
                let init = host.then(|| MatchInitial {
                    game_settings: GameSettings::default(),
                });
                SessionMessage::InitMatch((name.to_owned(), init))
            }
            SessionMessage::InitRound(init) => {
                println!("InitRound: frames={}, initial={:?}", frames, init);
                if host == init.is_some() {
                    println!("warning: the opponent has the same role");
                }
                frames = 0;
                SessionMessage::InitRound(host.then(random_round_initial))
            }
            SessionMessage::Delay(delay) => {
                println!("Delay: {}", delay);
                continue;
            }
            SessionMessage::Input(input) => {
                frames += 1;
                if frames % PROGRESS_INTERVAL == 0 {
                    println!("frames={}, input={:#06x}", frames, input);
                }
                SessionMessage::Input(input)
            }
        };
        send(&data_channel, &reply).await?;
    }
    println!("Disconnected.");
    Ok(())
}

/// ホストから受け取ったメッセージを表示する
async fn run_spectator(mut data_channel: DataChannel) -> Result<()> {
    let mut frames = 0;
    while let Some(data) = data_channel.recv().await {
        match rmp_serde::from_slice(&data)? {
            SpectatorSessionMessage::InitSpectator(init) => {
                println!(
                    "InitSpectator: p1={:?}, p2={:?}, settings={:?}",
                    init.p1_name(),
                    init.p2_name(),
                    init.game_settings()
                );
            }
            SpectatorSessionMessage::InitGame(init) => {
                println!(
                    "InitGame: frames={}, difficulty={}, p1={:?}, p2={:?}, round={:?}",
                    frames,
                    init.difficulty(),
                    init.p1(),
                    init.p2(),
                    init.round_initial()
                );
                frames = 0;
            }
            SpectatorSessionMessage::InitRound(init) => {
                println!("InitRound: frames={}, initial={:?}", frames, init);
                frames = 0;
            }
            SpectatorSessionMessage::Inputs(p1, p2) => {
                frames += 1;
                if frames % PROGRESS_INTERVAL == 0 {
                    println!("frames={}, p1={:#06x}, p2={:#06x}", frames, p1, p2);
                }
            }
        }
    }
    println!("Disconnected.");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();
    let args = parse_args()?;
    let stdin_rx = spawn_stdin_reader();

    match args.role {
        Role::Host => {
            let (_conn, data_channel) = connect_as_offerer(
                SignalingCodeType::BattleOffer,
                SignalingCodeType::BattleAnswer,
                BATTLE_PROTOCOL,
                &stdin_rx,
            )
            .await?;
            println!("Connected.");
            run_battle(data_channel, true, &args.name).await
        }
        Role::Guest => {
            let (_conn, data_channel) = connect_as_answerer(
                SignalingCodeType::BattleOffer,
                SignalingCodeType::BattleAnswer,
                BATTLE_PROTOCOL,
                &stdin_rx,
            )
            .await?;
            println!("Connected.");
            run_battle(data_channel, false, &args.name).await
        }
        Role::Spectator => {
            let (_conn, data_channel) = connect_as_offerer(
                SignalingCodeType::SpectatorOffer,
                SignalingCodeType::SpectatorAnswer,
                SPECTATOR_PROTOCOL,
                &stdin_rx,
            )
            .await?;
            println!("Connected.");
            run_spectator(data_channel).await
        }
    }
}
