pub mod socket;
pub mod stdio_signaling_interface;

use std::io::Write;

use anyhow::{anyhow, bail, Result};
use base64::{prelude::BASE64_STANDARD_NO_PAD, Engine};
use flate2::{
    write::{DeflateDecoder, DeflateEncoder},
    Compression,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use webrtc::peer_connection::sdp::{
    sdp_type::RTCSdpType, session_description::RTCSessionDescription,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct CompressedSessionDesc(pub String);

impl CompressedSessionDesc {
    pub fn compress(desc: &RTCSessionDescription, is_spectator: bool) -> Self {
        let mut e = DeflateEncoder::new(Vec::new(), Compression::best());
        e.write_all(desc.sdp.as_bytes()).unwrap();
        let compressed_bytes = e.finish().unwrap();

        Self(format!(
            r#"<{}{}>{}</{}{}>"#,
            if is_spectator { "s-" } else { "" },
            desc.sdp_type,
            BASE64_STANDARD_NO_PAD.encode(compressed_bytes),
            if is_spectator { "s-" } else { "" },
            desc.sdp_type,
        ))
    }

    pub fn decompress(&self) -> Result<(RTCSessionDescription, bool)> {
        let captures = Regex::new(r#"<(.+?)>(.+?)</(.+?)>"#)
            .unwrap()
            .captures(&self.0)
            .ok_or_else(|| anyhow!("Failed to parse"))?;
        let sdp_type = &captures[1];
        let sdp_type_end = &captures[3];
        if sdp_type != sdp_type_end {
            bail!("unmatched tag: <{}></{}>", sdp_type, sdp_type_end);
        }
        let compressed_bytes =
            BASE64_STANDARD_NO_PAD.decode(captures[2].replace(['\n', ' '], ""))?;

        let mut d = DeflateDecoder::new(Vec::new());
        d.write_all(&compressed_bytes)?;
        let sdp = String::from_utf8_lossy(&d.finish()?).to_string();
        Ok((
            match RTCSdpType::from(sdp_type.replace("s-", "").as_str()) {
                RTCSdpType::Offer => RTCSessionDescription::offer(sdp),
                RTCSdpType::Pranswer => RTCSessionDescription::pranswer(sdp),
                RTCSdpType::Answer => RTCSessionDescription::answer(sdp),
                RTCSdpType::Unspecified | RTCSdpType::Rollback => {
                    bail!("Failed to parse from {:?}", self.0)
                }
            }?,
            self.0.starts_with("<s-offer>") || self.0.starts_with("<s-answer>"),
        ))
    }
}
