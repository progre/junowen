use std::{
    env::{self, current_exe},
    ffi::OsStr,
    io::Write,
};

use anyhow::Result;
use interprocess::os::windows::named_pipe::ByteWriterPipeStream;
use junowen_lib::hook_utils::inject_dll;

fn main() -> Result<()> {
    let name = OsStr::new("th19netdelayemulate");
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

        let name = OsStr::new("th19netdelayemulate");
        ByteWriterPipeStream::connect(name).unwrap()
    };

    let buf = [env::args().nth(1).unwrap().parse::<u8>().unwrap(); 1];
    pipe.write_all(&buf).unwrap();

    Ok(())
}
