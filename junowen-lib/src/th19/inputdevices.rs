use getset::Getters;

#[derive(Clone, Copy, PartialEq)]
pub struct Input(pub u32);

impl Input {
    pub const NULL:  /*-*/u16 = 0b00000000_00000000;
    pub const SHOT:  /*-*/u16 = 0b00000000_00000001;
    pub const CHARGE:/*-*/u16 = 0b00000000_00000010;
    pub const BOMB:  /*-*/u16 = 0b00000000_00000100;
    pub const SLOW:  /*-*/u16 = 0b00000000_00001000;
    pub const UP:    /*-*/u16 = 0b00000000_00010000;
    pub const DOWN:  /*-*/u16 = 0b00000000_00100000;
    pub const LEFT:  /*-*/u16 = 0b00000000_01000000;
    pub const RIGHT: /*-*/u16 = 0b00000000_10000000;
    pub const START: /*-*/u16 = 0b00000001_00000000;
}

impl From<u16> for Input {
    fn from(value: u16) -> Self {
        Self(value as u32)
    }
}

/// 0x3d4
#[derive(Getters)]
#[repr(C)]
pub struct InputDevice {
    _unknown1: [u8; 0x010],
    pub input: Input,
    pub prev_input: Input,
    _unknown2: [u8; 0x2a8],
    _unknown3: [u8; 0x010],
    #[getset(get = "pub")]
    raw_keys: [u8; 0x104],
}

#[derive(Getters)]
#[repr(C)]
pub struct InputDevices {
    _unknown1: [u8; 0x20],
    #[getset(get = "pub")]
    input_device_array: [InputDevice; 3 + 9],
    _unknown2: [u8; 0x14],
    pub p1_idx: u32,
    p2_idx: u32,
    // unknown remains...
}

impl InputDevices {
    pub fn p1_input(&self) -> Input {
        self.input_device_array[self.p1_idx as usize].input
    }
    pub fn set_p1_input(&mut self, value: Input) {
        self.input_device_array[self.p1_idx as usize].input = value;
    }
    pub fn p1_prev_input(&self) -> Input {
        self.input_device_array[self.p1_idx as usize].prev_input
    }

    pub fn p2_input(&self) -> Input {
        self.input_device_array[self.p2_idx as usize].input
    }
    pub fn set_p2_input(&mut self, value: Input) {
        self.input_device_array[self.p2_idx as usize].input = value;
    }
    pub fn p2_prev_input(&self) -> Input {
        self.input_device_array[self.p2_idx as usize].prev_input
    }

    pub fn is_conflict_keyboard_full(&self) -> bool {
        self.p1_idx == 0 && self.p2_idx == 0
    }
}
