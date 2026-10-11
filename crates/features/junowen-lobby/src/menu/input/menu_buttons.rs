/// メニュー操作に使うボタンの押下状態
///
/// 決定とキャンセルは、対応するボタンのいずれかが新たに押された時点で反応する。
/// そのため、ボタンごとに 1 ビットを使う
#[derive(Clone, Copy, Debug, Default)]
pub struct MenuButtons {
    pub up: bool,
    pub down: bool,
    pub decide: u8,
    pub cancel: u8,
}

impl MenuButtons {
    pub fn decide_pulsed(self, prev: Self) -> bool {
        self.decide & !prev.decide != 0
    }

    pub fn cancel_pulsed(self, prev: Self) -> bool {
        self.cancel & !prev.cancel != 0
    }
}

#[cfg(test)]
mod tests {
    use super::MenuButtons;

    fn cancel(bits: u8) -> MenuButtons {
        MenuButtons {
            cancel: bits,
            ..Default::default()
        }
    }

    #[test]
    fn test_cancel_pulsed() {
        assert!(cancel(0b01).cancel_pulsed(cancel(0b00)));
        assert!(!cancel(0b01).cancel_pulsed(cancel(0b01)));
        assert!(cancel(0b11).cancel_pulsed(cancel(0b01)));
        assert!(!cancel(0b01).cancel_pulsed(cancel(0b11)));
    }
}
