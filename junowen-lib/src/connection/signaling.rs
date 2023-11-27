pub mod socket;
#[cfg(target_os = "windows")]
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

#[derive(Clone, Copy, PartialEq)]
pub enum SignalingCodeType {
    BattleOffer,
    BattleAnswer,
    SpectatorOffer,
    SpectatorAnswer,
}

impl SignalingCodeType {
    pub fn to_string(&self, desc: &CompressedSdp) -> String {
        let tag = match self {
            Self::BattleOffer => "offer",
            Self::BattleAnswer => "answer",
            Self::SpectatorOffer => "s-offer",
            Self::SpectatorAnswer => "s-answer",
        };
        format!("<{}>{}</{}>", tag, desc.0, tag,)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompressedSdp(String);

impl CompressedSdp {
    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn compress(desc: &RTCSessionDescription) -> Self {
        let mut e = DeflateEncoder::new(Vec::new(), Compression::best());
        e.write_all(desc.sdp.as_bytes()).unwrap();
        let compressed_bytes = e.finish().unwrap();
        Self(BASE64_STANDARD_NO_PAD.encode(compressed_bytes))
    }
}

pub fn parse_signaling_code(code: &str) -> Result<(SignalingCodeType, CompressedSdp)> {
    let code = Regex::new(r"\s").unwrap().replace_all(code, "");
    let captures = Regex::new(r#"<(.+?)>(.+?)</(.+?)>"#)
        .unwrap()
        .captures(&code)
        .ok_or_else(|| anyhow!("Failed to parse"))?;
    let tag = &captures[1];
    let tag_end = &captures[3];
    let desc = &captures[2];
    if tag != tag_end {
        bail!("unmatched tag: <{}></{}>", tag, tag_end);
    }
    let sct = match tag {
        "offer" => SignalingCodeType::BattleOffer,
        "answer" => SignalingCodeType::BattleAnswer,
        "s-offer" => SignalingCodeType::SpectatorOffer,
        "s-answer" => SignalingCodeType::SpectatorAnswer,
        _ => bail!("unknown tag: {}", tag),
    };
    Ok((sct, CompressedSdp(desc.to_owned())))
}

pub fn decompress_session_description(
    sdp_type: RTCSdpType,
    csdp: CompressedSdp,
) -> Result<RTCSessionDescription> {
    let compressed_bytes = BASE64_STANDARD_NO_PAD.decode(csdp.0)?;
    let mut d = DeflateDecoder::new(Vec::new());
    d.write_all(&compressed_bytes)?;
    let sdp = String::from_utf8_lossy(&d.finish()?).to_string();
    Ok(match sdp_type {
        RTCSdpType::Offer => RTCSessionDescription::offer(sdp)?,
        RTCSdpType::Answer => RTCSessionDescription::answer(sdp)?,
        RTCSdpType::Pranswer | RTCSdpType::Unspecified | RTCSdpType::Rollback => unreachable!(),
    })
}
