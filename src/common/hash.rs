use crypto::digest::Digest;
use crypto::md5::Md5;
use std::iter::repeat;

pub trait MD5 {
    fn md5(self) -> Vec<u8>;
}

impl<T: AsRef<[u8]>> MD5 for T {
    fn md5(self) -> Vec<u8> {
        let mut md5 = Md5::new();
        md5.input(self.as_ref());
        let mut key: Vec<u8> = repeat(0).take((md5.output_bits() + 7) / 8).collect();
        md5.result(key.as_mut_slice());
        key
    }
}
