use junowen_lib::InputDevices;

pub fn inputed_number(input_devices: &InputDevices) -> Option<u8> {
    let raw_keys = input_devices.keyboard_input().raw_keys();
    (0..=9).find(|i| raw_keys[(b'0' + i) as usize] & 0x80 != 0)
}

pub fn pushed_f1(input_devices: &InputDevices) -> bool {
    let raw_keys = input_devices.keyboard_input().raw_keys();
    raw_keys[0x70] & 0x80 != 0
}
