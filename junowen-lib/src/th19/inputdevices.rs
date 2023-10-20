use flagset::{flags, FlagSet, InvalidBits};
use getset::{CopyGetters, Getters, MutGetters, Setters};

flags! {
    pub enum InputFlags: u32 {
        SHOT,
        CHARGE,
        BOMB,
        SLOW,
        UP,
        DOWN,
        LEFT,
        RIGHT,
        START,
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct InputValue(pub FlagSet<InputFlags>);

impl InputValue {
    pub fn full() -> Self {
        Self(FlagSet::full())
    }

    pub fn empty() -> Self {
        Self(None.into())
    }

    pub fn bits(&self) -> u32 {
        self.0.bits()
    }
}

impl TryFrom<u32> for InputValue {
    type Error = InvalidBits;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(Self(FlagSet::<InputFlags>::new(value)?))
    }
}

impl From<InputFlags> for InputValue {
    fn from(flag: InputFlags) -> Self {
        Self(flag.into())
    }
}

#[derive(CopyGetters, Setters)]
#[repr(C)]
pub struct Input {
    #[getset(get_copy = "pub", set = "pub")]
    current: InputValue,
    #[getset(get_copy = "pub")]
    prev: InputValue,
    #[getset(get_copy = "pub")]
    repeat: InputValue,
    _repeat2: InputValue,
    _unknown: [u8; 0x18],
    #[getset(get_copy = "pub")]
    up_repeat_count: u32,
    #[getset(get_copy = "pub")]
    down_repeat_count: u32,
    #[getset(get_copy = "pub")]
    left_repeat_count: u32,
    #[getset(get_copy = "pub")]
    right_repeat_count: u32,
    // ambiguous remains...
}

/// 0x3d4
#[derive(Getters, MutGetters)]
#[repr(C)]
pub struct InputDevice {
    _unknown1: [u8; 0x010],
    #[getset(get = "pub", get_mut = "pub")]
    input: Input,
    _unknown2: [u8; 0x278],
    _unknown3: [u8; 0x010],
    #[getset(get = "pub")]
    raw_keys: [u8; 0x104],
}

#[derive(CopyGetters, Getters, Setters)]
#[repr(C)]
pub struct InputDevices {
    _unknown1: [u8; 0x20],
    #[getset(get = "pub")]
    input_device_array: [InputDevice; 3 + 9],
    _unknown2: [u8; 0x14],
    #[getset(get_copy = "pub", set = "pub")]
    p1_idx: u32,
    p2_idx: u32,
    // unknown remains...
}

impl InputDevices {
    pub fn p1_input(&self) -> &Input {
        &self.input_device_array[self.p1_idx as usize].input
    }
    pub fn p1_input_mut(&mut self) -> &mut Input {
        &mut self.input_device_array[self.p1_idx as usize].input
    }

    pub fn p2_input(&self) -> &Input {
        &self.input_device_array[self.p2_idx as usize].input
    }
    pub fn p2_input_mut(&mut self) -> &mut Input {
        &mut self.input_device_array[self.p2_idx as usize].input
    }

    pub fn is_conflict_input_device(&self) -> bool {
        self.p1_idx == self.p2_idx
    }
}
