//! 観戦者が溜まった入力を早送りで追いかけるときに、画面の表示 (`Present`) を間引く
//!
//! 試合中は no wait が効かず、表示の垂直同期待ちで 60fps に制限されていると考えられるため、
//! 表示を間引いて待ちをなくす。

use std::sync::atomic::{AtomicU32, Ordering};

/// 早送りを要求されてから、表示を間引き続けるフレーム数。
/// 要求が途絶えたら (セッションが終了した場合など) すぐに通常の表示に戻す
const SKIP_FRAMES_PER_REQUEST: u32 = 2;
/// 間引いている間も、このフレーム数ごとに1回は表示する
const PRESENT_INTERVAL: u32 = 8;

static REMAINING_FRAMES: AtomicU32 = AtomicU32::new(0);
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// 早送り中は毎フレーム呼ぶ
pub fn request_skip() {
    REMAINING_FRAMES.store(SKIP_FRAMES_PER_REQUEST, Ordering::Relaxed);
}

/// 表示を間引く場合は `true` を返す
pub fn should_skip_present() -> bool {
    let remaining = REMAINING_FRAMES.load(Ordering::Relaxed);
    if remaining == 0 {
        return false;
    }
    REMAINING_FRAMES.store(remaining - 1, Ordering::Relaxed);
    !COUNTER
        .fetch_add(1, Ordering::Relaxed)
        .is_multiple_of(PRESENT_INTERVAL)
}
