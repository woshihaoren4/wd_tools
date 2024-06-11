mod common;
mod id_generator;
mod net;

#[cfg(feature = "ptr")]
pub mod ptr;

#[cfg(feature = "time")]
pub mod time;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "fs")]
pub mod fs;

#[cfg(feature = "pool")]
pub mod pool;

#[cfg(feature = "chan")]
pub mod channel;

#[cfg(feature = "coll")]
pub mod coll;

#[allow(unused_imports)]
pub use common::*;

#[allow(unused_imports)]
pub use id_generator::*;

#[allow(unused_imports)]
pub use net::*;


#[cfg(test)]
// #[cfg(feature = "full")]
mod test {
    use super::*;
    use super::common::EncodeHex;
    use std::sync::Arc;
    use crate::coll::ByteMap;
    use super::common::SimpleRegexMatch;

    #[test]
    fn test_hex() {
        let data: Vec<u8> = vec![101, 201, 30, 40, 50, 60, 70, 80];
        assert_eq!(
            "65c91e28323c4650",
            data.to_hex().as_str(),
            "test_hex failed"
        )
    }
    #[test]
    fn test_base64_url() {
        let data = "@hello, wo/-rld*";
        let ciphertext = data.base64_encode_url().expect("base64_encode_url error");
        assert_eq!(
            r#"QGhlbGxvLCB3by8tcmxkKg"#,
            ciphertext.as_str(),
            "test_base64_url encode failed"
        );
        let plaintext = ciphertext
            .try_base64_decode_url()
            .expect("try_base64_decode_url error");
        assert_eq!(Vec::from(data), plaintext, "test_base64_url decode failed");
    }

    #[test]
    fn test_base64_std() {
        let data = "@hello, wo/-rld*";
        let ciphertext = data.base64_encode_std();
        assert_eq!(
            r#"QGhlbGxvLCB3by8tcmxkKg=="#,
            ciphertext.as_str(),
            "test_base64_std encode failed"
        );
        let plaintext = ciphertext
            .try_base64_decode_std()
            .expect("try_base64_decode_std error");
        assert_eq!(Vec::from(data), plaintext, "test_base64_std decode failed");
    }

    #[test]
    fn test_md5() {
        let data = "hello world";
        let md5 = data.md5().to_hex();
        assert_eq!(
            r#"5eb63bbbe01eeed093cb22bb8f5acdc3"#,
            md5.as_str(),
            "test_md5 failed"
        );
    }

    #[test]
    fn test_pf() {
        let data = 10u8;
        assert_eq!(data.ok(), Ok::<u8, ()>(10), "test_pf ok");
        assert_eq!(data.err(), Err::<(), u8>(10), "test_pf err");
        assert_eq!(data.to_box(), Box::new(10), "test_pf box");
        assert_eq!(data.arc(), Arc::new(10), "test_pf arc");
        assert_eq!(data.some(), Some(10), "test_pf option");
    }

    #[test]
    fn test_ptr() {
        let src = 129u8;
        let des:i8 = ptr::unsafe_must_take(src);
        assert_eq!(des, -127, "force_arc_to_var failed");
        let src = 129u8;
        let des:i8 = ptr::unsafe_must_downcast::<_, i8>(src);
        assert_eq!(des, -127, "force_box_to_var failed");
    }

    #[test]
    fn test_snowflake() {
        let id = snowflake_id();
        println!("id --> {}", id);
    }

    #[test]
    fn test_uuid_v4_v5() {
        let uuid = uuid::v4();
        println!("uuid v4 --> {}", uuid);
        let uuid = uuid::v5(uuid::UuidV5Namespace::DNS, b"hello world");
        println!("uuid v5 --> {}", uuid);
    }

    #[test]
    fn test_time_utc_timestamp() {
        let ts = time::utc_timestamp();
        println!("{}", ts);
        let mts = time::utc_timestamp_millis();
        println!("{}", mts)
    }

    #[test]
    fn test_less_lock() {
        let lkv = sync::Acl::new(0);
        let one = lkv.share();
        assert_eq!(Arc::new(0), one, "test_less_lock one failed");
        lkv.update(|i| &*i + 1);
        assert_eq!(Arc::new(1), lkv.share(), "test_less_lock two failed");
        lkv.update(|i| &*i + 1);
        assert_eq!(Arc::new(2), lkv.share(), "test_less_lock three failed");
    }

    // #[derive(Clone, Eq, PartialEq, Debug, Default)]
    // struct NLTest(usize);
    // impl Drop for NLTest {
    //     fn drop(&mut self) {
    //         println!("drop NLTest {}", self.0)
    //     }
    // }

    // #[test]
    // fn test_null_lock(){
    //     let nl = NullLock::<NLTest>::new();
    //     let nu = nl.get();
    //     assert_eq!(None,nu,"test_null_lock null failed");
    //
    //     let nu = nl.get_unwrap();
    //     assert_eq!(NLTest::default(),nu,"test_null_lock default failed");
    //
    //     nl.init(NLTest(1));
    //     let i = nl.map(|x| {
    //         x.0 + 1
    //     }).unwrap();
    //     assert_eq!(2,i,"test_null_lock non null failed")
    // }

    #[test]
    fn byte_map_chinese(){
        let mut map = ByteMap::new();
        map.insert(&("你好".chars().collect::<Vec<char>>()),"你好");
        map.insert(&("hello".chars().collect::<Vec<char>>()),"hello");
        map.insert(&("123".chars().collect::<Vec<char>>()),"123");

        let target = "飞流之下，123，hello，你好".chars().collect::<Vec<char>>();

        for i in 0..target.len(){
            if let Some(s) = map.match_first(&target[i..]){
                println!("match ok ---> {}",s);
            }
        }
    }

    #[test]
    fn regex(){
        let re = r#"\[(.*?)\]"#;
        let s = r#"a[b][c]d[1[2]3]"#;

        let vec = s.regex(re).unwrap();
        println!("simple regex match : {:?}",vec);
    }
}
