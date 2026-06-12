pub trait AsBytes {
    fn as_byte(&self) -> &[u8] {
        value_as_bytes(self)
    }
}

#[inline]
fn value_as_bytes<T: ?Sized>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            (value as *const T).cast::<u8>(),
            std::mem::size_of_val(value),
        )
    }
}

#[inline]
fn slice_as_bytes<T>(value: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), std::mem::size_of_val(value)) }
}

macro_rules! primitive_default_for_as_bytes {
    ($($b:ty);*) => {
        $(
        impl AsBytes for $b {}

        impl AsBytes for [$b]
        {
            fn as_byte(&self) -> &[u8] {
                slice_as_bytes(self)
            }
        }

        impl<const N: usize> AsBytes for [$b; N]
        {
            fn as_byte(&self) -> &[u8] {
                self.as_slice().as_byte()
            }
        }

        impl AsBytes for Vec<$b>
        {
            fn as_byte(&self) -> &[u8] {
                self.as_slice().as_byte()
            }
        }
        )*

    };
}

primitive_default_for_as_bytes!(u8;u16;u32;u64;u128;usize;i8;i16;i32;i64;i128;isize;f32;f64;bool;char);

impl AsBytes for str {
    fn as_byte(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsBytes for String {
    fn as_byte(&self) -> &[u8] {
        self.as_str().as_byte()
    }
}

impl<T> AsBytes for &T
where
    T: AsBytes + ?Sized,
{
    fn as_byte(&self) -> &[u8] {
        (*self).as_byte()
    }
}

impl<T> AsBytes for &mut T
where
    T: AsBytes + ?Sized,
{
    fn as_byte(&self) -> &[u8] {
        (**self).as_byte()
    }
}

pub fn bytes_to_usize(bytes: &[u8]) -> usize {
    bytes.iter().fold(5381usize, |acc, &b| {
        // (acc << 5) + acc + byte 相当于 acc * 33 + byte
        // 使用 wrapping_add 防止溢出 panic
        (acc.wrapping_shl(5)).wrapping_add(acc) ^ (b as usize)
    })
}

#[cfg(test)]
mod tests {
    use super::AsBytes;
    use std::mem::{size_of, size_of_val};

    #[test]
    fn primitive_values_keep_their_full_byte_width() {
        assert_eq!(1u8.as_byte().len(), size_of::<u8>());
        assert_eq!(1u16.as_byte().len(), size_of::<u16>());
        assert_eq!(1u32.as_byte().len(), size_of::<u32>());
        assert_eq!(1u64.as_byte().len(), size_of::<u64>());
        assert_eq!(1u128.as_byte().len(), size_of::<u128>());
        assert_eq!(1usize.as_byte().len(), size_of::<usize>());
        assert_eq!((-1i8).as_byte().len(), size_of::<i8>());
        assert_eq!((-1i16).as_byte().len(), size_of::<i16>());
        assert_eq!((-1i32).as_byte().len(), size_of::<i32>());
        assert_eq!((-1i64).as_byte().len(), size_of::<i64>());
        assert_eq!((-1i128).as_byte().len(), size_of::<i128>());
        assert_eq!((-1isize).as_byte().len(), size_of::<isize>());
        assert_eq!(1.0f32.as_byte().len(), size_of::<f32>());
        assert_eq!(1.0f64.as_byte().len(), size_of::<f64>());
        assert_eq!(true.as_byte().len(), size_of::<bool>());
        assert_eq!('你'.as_byte().len(), size_of::<char>());
    }

    #[test]
    fn primitive_collections_keep_their_full_byte_width() {
        let words = [1u32, 2, 3];
        assert_eq!(words.as_byte().len(), size_of_val(&words));
        assert_eq!((&words[..]).as_byte().len(), size_of_val(&words[..]));

        let chars = vec!['你', '好'];
        assert_eq!(chars.as_byte().len(), chars.len() * size_of::<char>());
        assert_eq!((&chars[..]).as_byte(), chars.as_byte());

        let bytes = [1u8, 2, 3];
        assert_eq!(bytes.as_byte(), &[1, 2, 3]);
    }

    #[test]
    fn strings_and_references_delegate_to_inner_bytes() {
        assert_eq!("hello".as_byte(), b"hello");

        let text = String::from("你好");
        assert_eq!(text.as_byte(), text.as_bytes());
        assert_eq!((&text).as_byte(), text.as_bytes());

        let mut data = [1u16, 2, 3];
        let data_ref = &mut data[..];
        assert_eq!(data_ref.as_byte().len(), 3 * size_of::<u16>());
    }

    #[test]
    fn custom_struct_can_expose_its_raw_bytes() {
        #[repr(C, packed)]
        struct PacketHeader {
            tag: u8,
            size: u16,
            flags: u8,
        }

        impl AsBytes for PacketHeader {}

        let header = PacketHeader {
            tag: 0xab,
            size: 0x1234,
            flags: 0xcd,
        };

        let mut expected = Vec::new();
        expected.push(0xab);
        expected.extend_from_slice(&0x1234u16.to_ne_bytes());
        expected.push(0xcd);

        assert_eq!(header.as_byte().len(), size_of::<PacketHeader>());
        assert_eq!(header.as_byte(), expected);
    }
}
