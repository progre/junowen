use anyhow::{Result, bail};
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// 圧縮済み SDP は数 KB に収まるため、これを超える長さは壊れたフレームとみなす
const MAX_FRAME_LEN: u32 = 64 * 1024;

/// メッセージを u32 ビッグエンディアンの長さ接頭辞付きで書き込む。
/// TCP はストリームであり、書き込み単位が読み込み単位と一致しないため境界を明示する。
pub async fn write_frame<T>(write: &mut (impl AsyncWrite + Unpin + Send), msg: T) -> Result<()>
where
    T: Serialize + Send,
{
    let body = rmp_serde::to_vec(&msg)?;
    let len = u32::try_from(body.len())?;
    write.write_all(&len.to_be_bytes()).await?;
    write.write_all(&body).await?;
    write.flush().await?;
    Ok(())
}

pub async fn read_frame<T>(read: &mut (impl AsyncRead + Unpin + Send)) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut len = [0u8; 4];
    read.read_exact(&mut len).await?;
    let len = u32::from_be_bytes(len);
    if len > MAX_FRAME_LEN {
        bail!("frame too large: {}", len);
    }
    let mut body = vec![0u8; len as usize];
    read.read_exact(&mut body).await?;
    Ok(rmp_serde::from_slice(&body)?)
}
