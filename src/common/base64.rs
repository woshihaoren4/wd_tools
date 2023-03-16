use ::base64::Engine;
use std::panic;

pub trait Base64URLEncode {
    fn base64_encode_url(self) -> anyhow::Result<String>;
}

pub trait Base64URLDecode {
    fn try_base64_decode_url(self) -> anyhow::Result<Vec<u8>>;
}

pub trait Base64StdEncode {
    fn base64_encode_std(self) -> String;
}

pub trait Base64StdDecode {
    fn try_base64_decode_std(self) -> anyhow::Result<Vec<u8>>;
}

impl<T: AsRef<[u8]> + panic::UnwindSafe> Base64URLEncode for T {
    fn base64_encode_url(self) -> anyhow::Result<String> {
        encode(self.as_ref())
    }
}

impl<T: AsRef<[u8]>> Base64URLDecode for T {
    fn try_base64_decode_url(self) -> anyhow::Result<Vec<u8>> {
        decode(self.as_ref())
    }
}

fn encode<T: AsRef<[u8]> + panic::UnwindSafe>(data: T) -> anyhow::Result<String> {
    let result = panic::catch_unwind(move || {
        ::base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
    });
    match result {
        Ok(o) => Ok(o),
        Err(e) => Err(anyhow::anyhow!("base64 encode panic:{:?}", e)),
    }
}

fn decode<T: AsRef<[u8]>>(data: T) -> anyhow::Result<Vec<u8>> {
    let buf = ::base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(data)?;
    Ok(buf)
}

impl<T: AsRef<[u8]> + panic::UnwindSafe> Base64StdEncode for T {
    fn base64_encode_std(self) -> String {
        ::base64::engine::general_purpose::STANDARD.encode(self)
    }
}

impl<T: AsRef<[u8]>> Base64StdDecode for T {
    fn try_base64_decode_std(self) -> anyhow::Result<Vec<u8>> {
        let buf = ::base64::engine::general_purpose::STANDARD.decode(self)?;
        Ok(buf)
    }
}
