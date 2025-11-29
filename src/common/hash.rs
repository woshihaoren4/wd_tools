use crypto::digest::Digest;
use crypto::md5::Md5;
use crypto::sha1::Sha1 as ShaOne;
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

#[allow(dead_code)]
pub trait Sha1 {
    fn sha1(self) -> String;
}

impl<T: AsRef<[u8]>> Sha1 for T {
    fn sha1(self) -> String {
        let mut s1 = ShaOne::new();
        s1.input(self.as_ref());
        s1.result_str()
    }
}
