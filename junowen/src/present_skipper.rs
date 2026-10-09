//! 観戦者がホストに追いつくまでの早送り中は、画面の表示 (`Present`) を間引く
//!
//! 試合中は、ゲームの no wait 設定を有効にしても処理が速くならない。
//! 表示のたびに垂直同期を待つため、1 秒あたり 60 フレームに抑えられていると考えられる。
//! そこで表示を間引き、この待ちを減らす。

use std::sync::atomic::{AtomicU32, Ordering};

/// 早送りを要求されてから、表示を間引き続けるフレーム数。
/// 要求が途絶えたら (セッションが終了した場合など) すぐに通常の表示に戻す
const SKIP_FRAMES_PER_REQUEST: u32 = 2;
/// 間引いている間も、このフレーム数ごとに1回は表示する
const PRESENT_INTERVAL: u32 = 8;

#[derive(Default)]
pub struct PresentSkipper {
    remaining_frames: AtomicU32,
    counter: AtomicU32,
}

impl PresentSkipper {
    /// 早送り中は毎フレーム呼ぶ
    pub fn request_skip(&self) {
        self.remaining_frames
            .store(SKIP_FRAMES_PER_REQUEST, Ordering::Relaxed);
    }

    /// 表示を間引く場合は `true` を返す
    pub fn should_skip_present(&self) -> bool {
        let requested = self
            .remaining_frames
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_sub(1))
            .is_ok();
        if !requested {
            return false;
        }
        !self
            .counter
            .fetch_add(1, Ordering::Relaxed)
            .is_multiple_of(PRESENT_INTERVAL)
    }
}
