pub fn as_u32_be(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 24) +
    ((array[1] as u32) << 16) +
    ((array[2] as u32) <<  8) +
    ((array[3] as u32) <<  0)
}

pub fn as_u32_le(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) <<  0) +
    ((array[1] as u32) <<  8) +
    ((array[2] as u32) << 16) +
    ((array[3] as u32) << 24)
}

pub fn as_u32(array: &[u8]) -> u32 {
    let mut byte_array: [u8; 4] = [0; 4];
    byte_array[0] = array[0];
    byte_array[1] = array[1];
    byte_array[2] = array[2];
    byte_array[3] = array[3];

    as_u32_be(&byte_array)
}