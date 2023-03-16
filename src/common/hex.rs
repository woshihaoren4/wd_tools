use rustc_serialize::hex::ToHex;

pub trait EncodeHex {
    fn to_hex(self) -> String;
}
impl<T: AsRef<[u8]>> EncodeHex for T {
    fn to_hex(self) -> String {
        ToHex::to_hex(&self.as_ref())
    }
}
