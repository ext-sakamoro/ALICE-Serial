//! Integration tests.

#![allow(
    clippy::wildcard_imports,
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::indexing_slicing
)]

use crate::encoder::*;
use crate::errors::*;
use crate::value::*;
use crate::varint::*;
use crate::zigzag::*;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::*;

#[test]
fn varint_roundtrip() {
    for &n in &[0u64, 1, 127, 128, 300, 16384, u64::MAX] {
        let encoded = encode_varint(n);
        let (decoded, _) = decode_varint(&encoded).unwrap();
        assert_eq!(decoded, n);
    }
}

#[test]
fn varint_size() {
    assert_eq!(encode_varint(0).len(), 1);
    assert_eq!(encode_varint(127).len(), 1);
    assert_eq!(encode_varint(128).len(), 2);
}

#[test]
fn zigzag_roundtrip() {
    for &n in &[0i64, 1, -1, 100, -100, i64::MAX, i64::MIN] {
        assert_eq!(zigzag_decode(zigzag_encode(n)), n);
    }
}

#[test]
fn encode_decode_null() {
    let encoded = encode(&Value::Null);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::Null);
}

#[test]
fn encode_decode_bool() {
    let encoded = encode(&Value::Bool(true));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::Bool(true));
}

#[test]
fn encode_decode_u64() {
    let encoded = encode(&Value::U64(12345));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::U64(12345));
}

#[test]
fn encode_decode_fixint() {
    let encoded = encode(&Value::U64(42));
    assert_eq!(encoded.len(), 1); // fixint
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::U64(42));
}

#[test]
fn encode_decode_string() {
    let encoded = encode(&Value::Str(String::from("hello")));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::Str(String::from("hello")));
}

#[test]
fn encode_decode_array() {
    let arr = Value::Array(vec![Value::U64(1), Value::U64(2), Value::Bool(true)]);
    let encoded = encode(&arr);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, arr);
}

#[test]
fn encode_decode_map() {
    let map = Value::Map(vec![(Value::Str(String::from("key")), Value::U64(42))]);
    let encoded = encode(&map);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, map);
}

#[test]
#[allow(clippy::approx_constant)]
fn encode_decode_f64() {
    let encoded = encode(&Value::F64(3.14));
    let (decoded, _) = decode(&encoded).unwrap();
    if let Value::F64(f) = decoded {
        assert!((f - 3.14).abs() < 0.001);
    } else {
        panic!("expected f64");
    }
}

#[test]
fn encode_decode_bytes() {
    let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let encoded = encode(&Value::Bytes(data.clone()));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::Bytes(data));
}

// =========================================================================
// VarInt 追加テスト
// =========================================================================

#[test]
fn varint_zero() {
    // 0は1バイトで符号化される
    let enc = encode_varint(0);
    assert_eq!(enc, vec![0x00]);
    let (val, len) = decode_varint(&enc).unwrap();
    assert_eq!(val, 0);
    assert_eq!(len, 1);
}

#[test]
fn varint_one_byte_max() {
    // 1バイト最大値: 127 (0x7F)
    let enc = encode_varint(127);
    assert_eq!(enc, vec![0x7F]);
    assert_eq!(enc.len(), 1);
}

#[test]
fn varint_two_byte_min() {
    // 2バイト最小値: 128
    let enc = encode_varint(128);
    assert_eq!(enc.len(), 2);
    assert_eq!(enc, vec![0x80, 0x01]);
    let (val, len) = decode_varint(&enc).unwrap();
    assert_eq!(val, 128);
    assert_eq!(len, 2);
}

#[test]
fn varint_two_byte_max() {
    // 2バイト最大値: 16383
    let enc = encode_varint(16383);
    assert_eq!(enc.len(), 2);
    let (val, _) = decode_varint(&enc).unwrap();
    assert_eq!(val, 16383);
}

#[test]
fn varint_three_byte_min() {
    // 3バイト最小値: 16384
    let enc = encode_varint(16384);
    assert_eq!(enc.len(), 3);
    let (val, len) = decode_varint(&enc).unwrap();
    assert_eq!(val, 16384);
    assert_eq!(len, 3);
}

#[test]
fn varint_u32_max() {
    // u32::MAXは5バイト
    let n = u64::from(u32::MAX);
    let enc = encode_varint(n);
    assert_eq!(enc.len(), 5);
    let (val, _) = decode_varint(&enc).unwrap();
    assert_eq!(val, n);
}

#[test]
fn varint_u64_max_size() {
    // u64::MAXは10バイトになる
    let enc = encode_varint(u64::MAX);
    assert_eq!(enc.len(), 10);
    let (val, _) = decode_varint(&enc).unwrap();
    assert_eq!(val, u64::MAX);
}

#[test]
fn varint_decode_empty() {
    // 空スライスはNone
    assert!(decode_varint(&[]).is_none());
}

#[test]
fn varint_decode_incomplete() {
    // 継続ビットが立ったまま終了 → None
    assert!(decode_varint(&[0x80]).is_none());
    assert!(decode_varint(&[0x80, 0x80]).is_none());
}

#[test]
fn varint_decode_overflow() {
    // shift > 63 でオーバーフロー検出 → None
    let data = [0x80; 11];
    assert!(decode_varint(&data).is_none());
}

#[test]
fn varint_decode_consumes_correct_bytes() {
    // 後続データがあっても正しいバイト数だけ消費する
    let mut buf = encode_varint(300);
    buf.push(0xFF);
    let (val, consumed) = decode_varint(&buf).unwrap();
    assert_eq!(val, 300);
    assert_eq!(consumed, buf.len() - 1);
}

#[test]
fn varint_powers_of_two() {
    // 2の累乗値でのラウンドトリップ
    for exp in 0..64 {
        let n = 1u64 << exp;
        let enc = encode_varint(n);
        let (val, _) = decode_varint(&enc).unwrap();
        assert_eq!(val, n, "2^{exp} のラウンドトリップ失敗");
    }
}

#[test]
fn varint_sequential_decode() {
    // 連続してエンコードされた複数値をデコード
    let values = [1u64, 128, 300, 0, u64::MAX];
    let mut buf = Vec::new();
    for &v in &values {
        buf.extend(encode_varint(v));
    }
    let mut offset = 0;
    for &expected in &values {
        let (val, consumed) = decode_varint(&buf[offset..]).unwrap();
        assert_eq!(val, expected);
        offset += consumed;
    }
    assert_eq!(offset, buf.len());
}

#[test]
fn varint_single_byte_values() {
    // 0-127は全て1バイト
    for n in 0..=127u64 {
        let enc = encode_varint(n);
        assert_eq!(enc.len(), 1, "値{n}が1バイトでない");
    }
}

// =========================================================================
// ZigZag 追加テスト
// =========================================================================

#[test]
fn zigzag_known_values() {
    // ZigZag符号化の既知の変換テーブル
    assert_eq!(zigzag_encode(0), 0);
    assert_eq!(zigzag_encode(-1), 1);
    assert_eq!(zigzag_encode(1), 2);
    assert_eq!(zigzag_encode(-2), 3);
    assert_eq!(zigzag_encode(2), 4);
}

#[test]
fn zigzag_symmetry() {
    // 正と負の対称性: encode(n) = 2n, encode(-n) = 2n-1
    for n in 1..=100i64 {
        #[allow(clippy::cast_sign_loss)]
        {
            assert_eq!(zigzag_encode(n), (2 * n) as u64);
            assert_eq!(zigzag_encode(-n), (2 * n - 1) as u64);
        }
    }
}

#[test]
fn zigzag_i64_min() {
    // i64::MINの符号化と復号
    let encoded = zigzag_encode(i64::MIN);
    assert_eq!(zigzag_decode(encoded), i64::MIN);
    assert_eq!(encoded, u64::MAX);
}

#[test]
fn zigzag_i64_max() {
    // i64::MAXの符号化
    let encoded = zigzag_encode(i64::MAX);
    assert_eq!(zigzag_decode(encoded), i64::MAX);
    assert_eq!(encoded, u64::MAX - 1);
}

#[test]
fn zigzag_decode_known_values() {
    assert_eq!(zigzag_decode(0), 0);
    assert_eq!(zigzag_decode(1), -1);
    assert_eq!(zigzag_decode(2), 1);
    assert_eq!(zigzag_decode(3), -2);
    assert_eq!(zigzag_decode(4), 2);
}

#[test]
fn zigzag_small_negative_encodes_small() {
    // 小さい負の数は小さい符号なし整数になる（効率的）
    assert!(zigzag_encode(-1) < zigzag_encode(100));
    assert!(zigzag_encode(-10) < zigzag_encode(100));
}

#[test]
fn zigzag_all_single_byte_varint_range() {
    // ZigZag + VarInt で1バイトに収まる範囲: -64..=63
    for n in -64..=63i64 {
        let z = zigzag_encode(n);
        let enc = encode_varint(z);
        assert_eq!(enc.len(), 1, "ZigZag({n})={z} が1バイトVarIntでない");
    }
}

#[test]
fn zigzag_roundtrip_full_range() {
    // 広範囲のラウンドトリップ
    for n in -1000..=1000i64 {
        assert_eq!(zigzag_decode(zigzag_encode(n)), n);
    }
}

#[test]
fn zigzag_combined_varint() {
    // ZigZag + VarInt の組み合わせラウンドトリップ
    let values = [0i64, 1, -1, 1000, -1000, i64::MAX, i64::MIN];
    for &n in &values {
        let z = zigzag_encode(n);
        let enc = encode_varint(z);
        let (decoded_z, _) = decode_varint(&enc).unwrap();
        let result = zigzag_decode(decoded_z);
        assert_eq!(result, n);
    }
}

// =========================================================================
// Value encode/decode 追加テスト
// =========================================================================

#[test]
fn encode_decode_bool_false() {
    let encoded = encode(&Value::Bool(false));
    let (decoded, consumed) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::Bool(false));
    assert_eq!(consumed, 1);
}

#[test]
fn encode_decode_u64_zero() {
    // 0はfixint
    let encoded = encode(&Value::U64(0));
    assert_eq!(encoded.len(), 1);
    assert_eq!(encoded[0], 0);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::U64(0));
}

#[test]
fn encode_decode_u64_fixint_boundary() {
    // fixint境界: 127はfixint, 128はフル形式
    let enc_127 = encode(&Value::U64(127));
    assert_eq!(enc_127.len(), 1);
    let enc_128 = encode(&Value::U64(128));
    assert_eq!(enc_128.len(), 9);
}

#[test]
fn encode_decode_u64_max() {
    let encoded = encode(&Value::U64(u64::MAX));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::U64(u64::MAX));
}

#[test]
fn encode_decode_i64_positive() {
    let encoded = encode(&Value::I64(42));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::I64(42));
}

#[test]
fn encode_decode_i64_negative() {
    let encoded = encode(&Value::I64(-42));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::I64(-42));
}

#[test]
fn encode_decode_i64_zero() {
    let encoded = encode(&Value::I64(0));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::I64(0));
}

#[test]
fn encode_decode_i64_min() {
    let encoded = encode(&Value::I64(i64::MIN));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::I64(i64::MIN));
}

#[test]
fn encode_decode_i64_max() {
    let encoded = encode(&Value::I64(i64::MAX));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::I64(i64::MAX));
}

#[test]
fn encode_decode_f64_zero() {
    let encoded = encode(&Value::F64(0.0));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::F64(0.0));
}

#[test]
fn encode_decode_f64_negative_zero() {
    // -0.0 と 0.0 はビットパターンが異なる
    let encoded = encode(&Value::F64(-0.0));
    let (decoded, _) = decode(&encoded).unwrap();
    if let Value::F64(f) = decoded {
        assert!(f.to_bits() == (-0.0_f64).to_bits());
    } else {
        panic!("expected f64");
    }
}

#[test]
fn encode_decode_f64_nan() {
    let encoded = encode(&Value::F64(f64::NAN));
    let (decoded, _) = decode(&encoded).unwrap();
    if let Value::F64(f) = decoded {
        assert!(f.is_nan());
    } else {
        panic!("expected f64");
    }
}

#[test]
fn encode_decode_f64_infinity() {
    let encoded = encode(&Value::F64(f64::INFINITY));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::F64(f64::INFINITY));
}

#[test]
fn encode_decode_f64_neg_infinity() {
    let encoded = encode(&Value::F64(f64::NEG_INFINITY));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::F64(f64::NEG_INFINITY));
}

#[test]
fn encode_decode_f64_subnormal() {
    // 非正規化数のラウンドトリップ
    let subnormal = f64::MIN_POSITIVE / 2.0;
    let encoded = encode(&Value::F64(subnormal));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::F64(subnormal));
}

#[test]
fn encode_decode_f64_max() {
    let encoded = encode(&Value::F64(f64::MAX));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::F64(f64::MAX));
}

#[test]
fn encode_decode_f64_min() {
    let encoded = encode(&Value::F64(f64::MIN));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::F64(f64::MIN));
}

#[test]
fn encode_decode_str_empty() {
    let encoded = encode(&Value::Str(String::new()));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::Str(String::new()));
}

#[test]
fn encode_decode_str_multibyte_utf8() {
    // マルチバイトUTF-8文字列
    let s = String::from("\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\u{4e16}\u{754c}\u{1f30d}");
    let encoded = encode(&Value::Str(s.clone()));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::Str(s));
}

#[test]
fn encode_decode_str_long() {
    // 長い文字列（1024バイト）
    let s: String = (0..1024).map(|_| 'A').collect();
    let encoded = encode(&Value::Str(s.clone()));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::Str(s));
}

#[test]
fn encode_decode_bytes_empty() {
    let encoded = encode(&Value::Bytes(vec![]));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::Bytes(vec![]));
}

#[test]
fn encode_decode_bytes_large() {
    // 大きいバイト列（256バイト）
    let data: Vec<u8> = (0..=255).collect();
    let encoded = encode(&Value::Bytes(data.clone()));
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, Value::Bytes(data));
}

#[test]
fn encode_decode_array_empty() {
    let arr = Value::Array(vec![]);
    let encoded = encode(&arr);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, arr);
}

#[test]
fn encode_decode_array_single() {
    let arr = Value::Array(vec![Value::Null]);
    let encoded = encode(&arr);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, arr);
}

#[test]
fn encode_decode_array_nested() {
    // ネストした配列
    let inner = Value::Array(vec![Value::U64(1), Value::U64(2)]);
    let outer = Value::Array(vec![inner, Value::Bool(true)]);
    let encoded = encode(&outer);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, outer);
}

#[test]
fn encode_decode_array_mixed_types() {
    // 異なる型を混在させた配列
    let arr = Value::Array(vec![
        Value::Null,
        Value::Bool(false),
        Value::U64(999),
        Value::I64(-1),
        Value::F64(1.5),
        Value::Str(String::from("test")),
        Value::Bytes(vec![1, 2, 3]),
    ]);
    let encoded = encode(&arr);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, arr);
}

#[test]
fn encode_decode_array_many_elements() {
    // 要素数100の配列
    let arr = Value::Array((0..100).map(Value::U64).collect());
    let encoded = encode(&arr);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, arr);
}

#[test]
fn encode_decode_map_empty() {
    let map = Value::Map(vec![]);
    let encoded = encode(&map);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, map);
}

#[test]
fn encode_decode_map_multiple_entries() {
    let map = Value::Map(vec![
        (Value::Str(String::from("a")), Value::U64(1)),
        (Value::Str(String::from("b")), Value::U64(2)),
        (Value::Str(String::from("c")), Value::U64(3)),
    ]);
    let encoded = encode(&map);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, map);
}

#[test]
fn encode_decode_map_nested_value() {
    // MapのValue側にArrayを持たせる
    let map = Value::Map(vec![(
        Value::Str(String::from("list")),
        Value::Array(vec![Value::U64(1), Value::U64(2)]),
    )]);
    let encoded = encode(&map);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, map);
}

#[test]
fn encode_decode_map_non_string_keys() {
    // 文字列以外のキー
    let map = Value::Map(vec![
        (Value::U64(1), Value::Str(String::from("one"))),
        (Value::Bool(true), Value::Null),
    ]);
    let encoded = encode(&map);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, map);
}

#[test]
fn encode_decode_map_nested_map() {
    // ネストしたMap
    let inner = Value::Map(vec![(Value::Str(String::from("x")), Value::U64(10))]);
    let outer = Value::Map(vec![(Value::Str(String::from("inner")), inner)]);
    let encoded = encode(&outer);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, outer);
}

#[test]
fn fixint_all_values() {
    // fixint範囲 0-127 の全値を検証
    for n in 0..=127u64 {
        let encoded = encode(&Value::U64(n));
        assert_eq!(encoded.len(), 1, "fixint {n} が1バイトでない");
        let (decoded, consumed) = decode(&encoded).unwrap();
        assert_eq!(decoded, Value::U64(n));
        assert_eq!(consumed, 1);
    }
}

// =========================================================================
// decode エラーケース
// =========================================================================

#[test]
fn decode_empty_data() {
    assert!(decode(&[]).is_none());
}

#[test]
fn decode_unknown_tag() {
    // 未定義タグ → None
    assert!(decode(&[0xC1]).is_none());
    assert!(decode(&[0xC4]).is_none());
    assert!(decode(&[0xD0]).is_none());
}

#[test]
fn decode_u64_truncated() {
    // TAG_U64の後にバイトが足りない
    assert!(decode(&[0xCF]).is_none());
    assert!(decode(&[0xCF, 0x00, 0x00]).is_none());
    assert!(decode(&[0xCF, 0, 0, 0, 0, 0, 0, 0]).is_none());
}

#[test]
fn decode_i64_truncated() {
    assert!(decode(&[0xD3]).is_none());
    assert!(decode(&[0xD3, 0x00, 0x00, 0x00]).is_none());
}

#[test]
fn decode_f64_truncated() {
    assert!(decode(&[0xCB]).is_none());
    assert!(decode(&[0xCB, 0x00]).is_none());
}

#[test]
fn decode_str_truncated_length() {
    // TAG_STRの後に長さフィールドが足りない
    assert!(decode(&[0xDB]).is_none());
    assert!(decode(&[0xDB, 0x00, 0x00]).is_none());
}

#[test]
fn decode_str_truncated_body() {
    // 長さは5だが実データが3バイトしかない
    assert!(decode(&[0xDB, 0x00, 0x00, 0x00, 0x05, b'a', b'b', b'c']).is_none());
}

#[test]
fn decode_str_invalid_utf8() {
    // 不正なUTF-8バイト列 → None
    let mut buf = vec![0xDB, 0x00, 0x00, 0x00, 0x02];
    buf.extend_from_slice(&[0xFF, 0xFE]);
    assert!(decode(&buf).is_none());
}

#[test]
fn decode_bin_truncated_length() {
    assert!(decode(&[0xC6]).is_none());
    assert!(decode(&[0xC6, 0x00]).is_none());
}

#[test]
fn decode_bin_truncated_body() {
    assert!(decode(&[0xC6, 0x00, 0x00, 0x00, 0x03, 0x01]).is_none());
}

#[test]
fn decode_array_truncated_length() {
    assert!(decode(&[0xDD]).is_none());
    assert!(decode(&[0xDD, 0x00, 0x00]).is_none());
}

#[test]
fn decode_array_truncated_elements() {
    // 要素数2だが要素が1つしかない
    let mut buf = vec![0xDD, 0x00, 0x00, 0x00, 0x02];
    buf.push(0xC0);
    assert!(decode(&buf).is_none());
}

#[test]
fn decode_map_truncated_length() {
    assert!(decode(&[0xDF]).is_none());
}

#[test]
fn decode_map_truncated_entries() {
    // エントリ数1だがキーしかない
    let mut buf = vec![0xDF, 0x00, 0x00, 0x00, 0x01];
    buf.push(0xC0);
    assert!(decode(&buf).is_none());
}

#[test]
fn decode_map_missing_value() {
    // キーはあるが値のデータが不完全
    let mut buf = vec![0xDF, 0x00, 0x00, 0x00, 0x01];
    buf.push(0xC0);
    buf.push(0xCF); // TAG_U64だがデータなし
    assert!(decode(&buf).is_none());
}

// =========================================================================
// consumed バイト数の検証
// =========================================================================

#[test]
fn decode_consumed_null() {
    let (_, consumed) = decode(&[0xC0]).unwrap();
    assert_eq!(consumed, 1);
}

#[test]
fn decode_consumed_bool() {
    let (_, consumed) = decode(&[0xC2]).unwrap();
    assert_eq!(consumed, 1);
    let (_, consumed) = decode(&[0xC3]).unwrap();
    assert_eq!(consumed, 1);
}

#[test]
fn decode_consumed_u64() {
    let enc = encode(&Value::U64(1000));
    let (_, consumed) = decode(&enc).unwrap();
    assert_eq!(consumed, 9);
}

#[test]
fn decode_consumed_i64() {
    let enc = encode(&Value::I64(-1));
    let (_, consumed) = decode(&enc).unwrap();
    assert_eq!(consumed, 9);
}

#[test]
fn decode_consumed_f64() {
    let enc = encode(&Value::F64(1.0));
    let (_, consumed) = decode(&enc).unwrap();
    assert_eq!(consumed, 9);
}

#[test]
fn decode_consumed_str() {
    let enc = encode(&Value::Str(String::from("abc")));
    let (_, consumed) = decode(&enc).unwrap();
    assert_eq!(consumed, 1 + 4 + 3);
}

#[test]
fn decode_consumed_bytes() {
    let enc = encode(&Value::Bytes(vec![1, 2]));
    let (_, consumed) = decode(&enc).unwrap();
    assert_eq!(consumed, 1 + 4 + 2);
}

#[test]
fn decode_consumed_empty_array() {
    let enc = encode(&Value::Array(vec![]));
    let (_, consumed) = decode(&enc).unwrap();
    assert_eq!(consumed, 5);
}

#[test]
fn decode_consumed_empty_map() {
    let enc = encode(&Value::Map(vec![]));
    let (_, consumed) = decode(&enc).unwrap();
    assert_eq!(consumed, 5);
}

// =========================================================================
// encode出力のバイナリ形式検証
// =========================================================================

#[test]
fn encode_null_tag() {
    assert_eq!(encode(&Value::Null), vec![0xC0]);
}

#[test]
fn encode_false_tag() {
    assert_eq!(encode(&Value::Bool(false)), vec![0xC2]);
}

#[test]
fn encode_true_tag() {
    assert_eq!(encode(&Value::Bool(true)), vec![0xC3]);
}

#[test]
fn encode_u64_tag_and_bytes() {
    let enc = encode(&Value::U64(256));
    assert_eq!(enc[0], 0xCF);
    let val = u64::from_be_bytes([
        enc[1], enc[2], enc[3], enc[4], enc[5], enc[6], enc[7], enc[8],
    ]);
    assert_eq!(val, 256);
}

#[test]
fn encode_i64_tag_and_bytes() {
    let enc = encode(&Value::I64(-100));
    assert_eq!(enc[0], 0xD3);
    let val = i64::from_be_bytes([
        enc[1], enc[2], enc[3], enc[4], enc[5], enc[6], enc[7], enc[8],
    ]);
    assert_eq!(val, -100);
}

#[test]
fn encode_str_tag_and_length() {
    let enc = encode(&Value::Str(String::from("hi")));
    assert_eq!(enc[0], 0xDB);
    let len = u32::from_be_bytes([enc[1], enc[2], enc[3], enc[4]]);
    assert_eq!(len, 2);
    assert_eq!(&enc[5..], b"hi");
}

#[test]
fn encode_bin_tag_and_length() {
    let enc = encode(&Value::Bytes(vec![0xAA, 0xBB]));
    assert_eq!(enc[0], 0xC6);
    let len = u32::from_be_bytes([enc[1], enc[2], enc[3], enc[4]]);
    assert_eq!(len, 2);
    assert_eq!(&enc[5..], &[0xAA, 0xBB]);
}

#[test]
fn encode_array_tag_and_count() {
    let enc = encode(&Value::Array(vec![Value::Null, Value::Null]));
    assert_eq!(enc[0], 0xDD);
    let count = u32::from_be_bytes([enc[1], enc[2], enc[3], enc[4]]);
    assert_eq!(count, 2);
}

#[test]
fn encode_map_tag_and_count() {
    let enc = encode(&Value::Map(vec![(Value::U64(1), Value::U64(2))]));
    assert_eq!(enc[0], 0xDF);
    let count = u32::from_be_bytes([enc[1], enc[2], enc[3], enc[4]]);
    assert_eq!(count, 1);
}

// =========================================================================
// SerialError テスト
// =========================================================================

#[test]
fn serial_error_display_invalid_format() {
    let e = SerialError::InvalidFormat;
    assert_eq!(alloc::format!("{e}"), "invalid format");
}

#[test]
fn serial_error_display_buffer_too_short() {
    let e = SerialError::BufferTooShort;
    assert_eq!(alloc::format!("{e}"), "buffer too short");
}

#[test]
fn serial_error_display_utf8_error() {
    let e = SerialError::Utf8Error;
    assert_eq!(alloc::format!("{e}"), "utf8 error");
}

#[test]
fn serial_error_clone_and_eq() {
    let e1 = SerialError::InvalidFormat;
    let e2 = e1.clone();
    assert_eq!(e1, e2);
    assert_ne!(e1, SerialError::BufferTooShort);
}

#[test]
fn serial_error_debug() {
    let e = SerialError::Utf8Error;
    let debug = alloc::format!("{e:?}");
    assert!(debug.contains("Utf8Error"));
}

// =========================================================================
// Value の Clone / Debug / PartialEq テスト
// =========================================================================

#[test]
fn value_clone_equality() {
    let values = vec![
        Value::Null,
        Value::Bool(true),
        Value::U64(42),
        Value::I64(-1),
        Value::F64(2.5),
        Value::Str(String::from("test")),
        Value::Bytes(vec![1, 2, 3]),
        Value::Array(vec![Value::Null]),
        Value::Map(vec![(Value::U64(1), Value::U64(2))]),
    ];
    for v in &values {
        let cloned = v.clone();
        assert_eq!(v, &cloned);
    }
}

#[test]
fn value_debug_output() {
    let v = Value::Str(String::from("hello"));
    let debug = alloc::format!("{v:?}");
    assert!(debug.contains("hello"));
}

#[test]
fn value_inequality() {
    assert_ne!(Value::Null, Value::Bool(false));
    assert_ne!(Value::U64(1), Value::U64(2));
    assert_ne!(Value::I64(1), Value::I64(-1));
    assert_ne!(Value::Str(String::from("a")), Value::Str(String::from("b")));
}

// =========================================================================
// 複合テスト（深いネスト、大きなデータ）
// =========================================================================

#[test]
fn deeply_nested_structure() {
    // 5階層のネスト
    let mut v = Value::U64(42);
    for _ in 0..5 {
        v = Value::Array(vec![v]);
    }
    let encoded = encode(&v);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, v);
}

#[test]
#[allow(clippy::approx_constant)]
fn complex_document() {
    // 複雑なドキュメント構造
    let doc = Value::Map(vec![
        (
            Value::Str(String::from("name")),
            Value::Str(String::from("Alice")),
        ),
        (Value::Str(String::from("age")), Value::U64(30)),
        (Value::Str(String::from("active")), Value::Bool(true)),
        (
            Value::Str(String::from("scores")),
            Value::Array(vec![Value::F64(95.5), Value::F64(87.3), Value::F64(92.1)]),
        ),
        (
            Value::Str(String::from("metadata")),
            Value::Map(vec![
                (Value::Str(String::from("version")), Value::U64(1)),
                (
                    Value::Str(String::from("data")),
                    Value::Bytes(vec![0xCA, 0xFE]),
                ),
            ]),
        ),
        (Value::Str(String::from("nothing")), Value::Null),
    ]);
    let encoded = encode(&doc);
    let (decoded, _) = decode(&encoded).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn decode_trailing_data_ignored() {
    // デコード後に余分なデータがあっても正しくデコードし、consumedが正しい
    let mut buf = encode(&Value::Bool(true));
    buf.extend_from_slice(&[0xFF, 0xFF, 0xFF]);
    let (val, consumed) = decode(&buf).unwrap();
    assert_eq!(val, Value::Bool(true));
    assert_eq!(consumed, 1);
}

#[test]
fn sequential_values_decode() {
    // 連続したValueをデコード
    let values = vec![
        Value::Null,
        Value::U64(100),
        Value::Str(String::from("seq")),
        Value::Bool(false),
    ];
    let mut buf = Vec::new();
    for v in &values {
        buf.extend(encode(v));
    }
    let mut offset = 0;
    for expected in &values {
        let (val, consumed) = decode(&buf[offset..]).unwrap();
        assert_eq!(&val, expected);
        offset += consumed;
    }
    assert_eq!(offset, buf.len());
}

#[test]
fn encode_deterministic() {
    // 同じ入力に対して常に同じ出力
    let v = Value::Map(vec![(Value::Str(String::from("k")), Value::U64(1))]);
    let enc1 = encode(&v);
    let enc2 = encode(&v);
    assert_eq!(enc1, enc2);
}

#[test]
fn all_fixint_decode_as_u64() {
    // 0x00-0x7F タグはfixintとしてU64にデコードされる
    for tag in 0..=0x7Fu8 {
        let (val, consumed) = decode(&[tag]).unwrap();
        assert_eq!(val, Value::U64(u64::from(tag)));
        assert_eq!(consumed, 1);
    }
}
