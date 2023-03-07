mod common;
mod id_generator;

#[cfg(feature = "ptr")]
pub mod ptr;

#[cfg(feature = "time")]
mod time;

pub use id_generator::*;
pub use common::*;

#[cfg(test)]
mod test{
    use std::sync::Arc;
    use crate::*;
    #[test]
    fn test_hex(){
        let data:Vec<u8> = vec![101,201,30,40,50,60,70,80];
        assert_eq!("65c91e28323c4650",data.to_hex().as_str(),"test_hex failed")
    }
    #[test]
    fn test_base64_url(){
        let data = "@hello, wo/-rld*";
        let ciphertext = data.base64_encode_url().expect("base64_encode_url error");
        assert_eq!(r#"QGhlbGxvLCB3by8tcmxkKg"#,ciphertext.as_str(),"test_base64_url encode failed");
        let plaintext = ciphertext.try_base64_decode_url().expect("try_base64_decode_url error");
        assert_eq!(Vec::from(data),plaintext,"test_base64_url decode failed");
    }

    #[test]
    fn test_base64_std(){
        let data = "@hello, wo/-rld*";
        let ciphertext = data.base64_encode_std();
        assert_eq!(r#"QGhlbGxvLCB3by8tcmxkKg=="#,ciphertext.as_str(),"test_base64_std encode failed");
        let plaintext = ciphertext.try_base64_decode_std().expect("try_base64_decode_std error");
        assert_eq!(Vec::from(data),plaintext,"test_base64_std decode failed");
    }

    #[test]
    fn test_md5(){
        let data = "hello world";
        let md5 = data.md5().to_hex();
        assert_eq!(r#"5eb63bbbe01eeed093cb22bb8f5acdc3"#,md5.as_str(),"test_md5 failed");
    }

    #[test]
    fn test_pf(){
        let data = 10u8;
        assert_eq!(data.ok(),Ok::<u8,()>(10),"test_pf ok");
        assert_eq!(data.err(),Err::<(),u8>(10),"test_pf err");
        assert_eq!(data.to_box(),Box::new(10),"test_pf box");
        assert_eq!(data.arc(),Arc::new(10),"test_pf arc");
        assert_eq!(data.some(),Some(10),"test_pf option");
    }

    #[test]
    fn test_ptr(){
        let src = 129u8;
        let des = ptr::force_arc_to_var::<_,i8>(Arc::new(src));
        assert_eq!(des,-127,"force_arc_to_var failed");
        let des = ptr::force_box_to_var::<_,i8>(src.to_box());
        assert_eq!(des,-127,"force_box_to_var failed");

    }

    #[test]
    fn test_snowflake(){
        let id = snowflake_id();
        println!("id --> {}",id);
    }

    #[test]
    fn test_uuid_v4_v5(){
        let uuid = uuid::v4();
        println!("uuid v4 --> {}",uuid);
        let uuid = uuid::v5(uuid::UuidV5Namespace::DNS,b"hello world");
        println!("uuid v5 --> {}",uuid);
    }

    #[test]
    fn test_time_utc_timestamp(){
        let ts = time::utc_timestamp();
        println!("{}",ts);
        let mts = time::utc_timestamp_millis();
        println!("{}",mts)
    }
}