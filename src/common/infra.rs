pub trait AsBytes {
    fn as_byte(&self) -> &[u8];
}

macro_rules! number_default_for_as_bytes {
    ($($b:tt,$n:tt);*) => {
        $(
        impl AsBytes for $b
        {
            fn as_byte(&self) -> &[u8] {
                unsafe {
                    &*(self as *const $b as *const [u8;$n])
                }
            }
        }
        )*

    };
}

number_default_for_as_bytes!(u8,1;u16,2;u32,4;u64,8;u128,16;i8,1;i16,2;i32,4;i64,8;i128,16);

#[cfg(target_pointer_width = "64")]
impl AsBytes for usize {
    fn as_byte(&self) -> &[u8] {
        unsafe { &*(self as *const usize as *const [u8; 8]) }
    }
}
#[cfg(target_pointer_width = "32")]
impl AsBytes for usize {
    fn as_byte(&self) -> &[u8] {
        unsafe { &*(self as *const usize as *const [u8; 4]) }
    }
}
#[cfg(target_pointer_width = "64")]
impl AsBytes for isize {
    fn as_byte(&self) -> &[u8] {
        unsafe { &*(self as *const isize as *const [u8; 8]) }
    }
}
#[cfg(target_pointer_width = "32")]
impl AsBytes for isize {
    fn as_byte(&self) -> &[u8] {
        unsafe { &*(self as *const isize as *const [u8; 4]) }
    }
}

impl AsBytes for Vec<u8> {
    fn as_byte(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsBytes for [u8] {
    fn as_byte(&self) -> &[u8] {
        self
    }
}
impl AsBytes for &str {
    fn as_byte(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl AsBytes for String {
    fn as_byte(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}
impl AsBytes for &[char] {
    fn as_byte(&self) -> &[u8] {
        unsafe { std::mem::transmute(*self) }
    }
}
impl AsBytes for Vec<char> {
    fn as_byte(&self) -> &[u8] {
        let cs = self.as_slice();
        unsafe { std::mem::transmute(cs) }
    }
}

impl<T> AsBytes for &T
where
    T: AsBytes,
{
    fn as_byte(&self) -> &[u8] {
        (*self).as_byte()
    }
}

pub fn bytes_to_usize(bytes: &[u8]) -> usize {
    bytes.iter().fold(5381usize, |acc, &b| {
        // (acc << 5) + acc + byte 相当于 acc * 33 + byte
        // 使用 wrapping_add 防止溢出 panic
        (acc.wrapping_shl(5)).wrapping_add(acc) ^ (b as usize)
    })
}
