#![no_main]

use alice_serial::{decode, decode_value, encode, encode_value};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok((v, used)) = decode_value(data) {
        assert!(used <= data.len());
        let again = encode_value(&v).expect("a decoded value encodes");
        assert_eq!(again, data[..used], "the accepted bytes are not the canonical encoding");
    }
    if let Ok(v) = decode(data) {
        assert_eq!(encode(&v).expect("a decoded value encodes"), data);
    }
});
