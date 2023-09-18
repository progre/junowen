use std::{
    env::{self, current_exe},
    ffi::OsStr,
    io::Write,
    thread::sleep,
    time::Duration,
};

use anyhow::Result;
use bytes::{BufMut, BytesMut};
use interprocess::os::windows::named_pipe::ByteWriterPipeStream;
use junowen_lib::inject_dll::inject_dll;

fn main() -> Result<()> {
    let replay_file = env::args().nth(1).unwrap();

    let pkg_name = env!("CARGO_PKG_NAME");
    let name = OsStr::new(pkg_name);
    let mut pipe = if let Ok(pipe) = ByteWriterPipeStream::connect(name) {
        println!("フック済みのDLLに接続しました");
        pipe
    } else {
        let dll_path = current_exe()?
            .as_path()
            .parent()
            .unwrap()
            .join(concat!(env!("CARGO_PKG_NAME"), "_hook.dll"));

        inject_dll("th19.exe", &dll_path)?;

        let name = OsStr::new(pkg_name);
        loop {
            if let Ok(pipe) = ByteWriterPipeStream::connect(name) {
                break pipe;
            }
            println!("waiting for pipe...");
            sleep(Duration::from_secs(3));
        }
    };

    let replay_file_bytes = replay_file.as_bytes();
    let mut buf = BytesMut::with_capacity(4);
    buf.put_u32_le(replay_file_bytes.len() as u32);
    pipe.write_all(&buf).unwrap();
    pipe.write_all(replay_file_bytes).unwrap();

    Ok(())
}
