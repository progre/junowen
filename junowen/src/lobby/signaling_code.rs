use clipboard_win::get_clipboard_string;
use junowen_lib::{
    Th19,
    connection::signaling::{CompressedSdp, SignalingCodeType, parse_signaling_code},
};

/// クリップボードから期待した種類のシグナリングコードを取り出す。
/// 取り出せなければブザーを鳴らして None を返す。
pub fn paste_signaling_code(th19: &Th19, expected: SignalingCodeType) -> Option<CompressedSdp> {
    let play = |sound| th19.play_sound(th19.sound_manager(), sound, 0);
    let Ok(clipboard) = get_clipboard_string() else {
        play(0x10);
        return None;
    };
    let Ok((code_type, sdp)) = parse_signaling_code(&clipboard) else {
        play(0x10);
        return None;
    };
    if code_type != expected {
        play(0x10);
        return None;
    }
    play(0x07);
    Some(sdp)
}
