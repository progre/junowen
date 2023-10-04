use junowen_lib::DevicesInput;

pub fn inputed_number(input: &DevicesInput) -> Option<u8> {
    let raw_keys = &input.input_device_array[0].raw_keys;
    (0..=9).find(|i| raw_keys[(b'0' + i) as usize] & 0x80 != 0)
}
