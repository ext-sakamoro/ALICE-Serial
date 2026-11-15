//! The serde deserializer for the bincode 1 default layout.

use serde::de::{self, DeserializeSeed, IntoDeserializer, Visitor};

use super::{Bincode1Error, BoxedError};
use crate::bounded::len_within;

type Result<T> = core::result::Result<T, BoxedError>;

/// Reads from `input`, never past `limit` bytes nor deeper than `max_depth`.
pub(super) struct Deserializer<'de> {
    input: &'de [u8],
    pos: usize,
    limit: usize,
    depth: usize,
    max_depth: usize,
}

impl<'de> Deserializer<'de> {
    pub(super) const fn new(input: &'de [u8], limit: usize, max_depth: usize) -> Self {
        Self {
            input,
            pos: 0,
            limit,
            depth: 0,
            max_depth,
        }
    }

    /// Run `f` one level deeper: an enum, a `Some`, a sequence, a map, a tuple, a struct
    /// or a newtype struct. A type can recurse only through these, so counting them
    /// bounds the stack a decode can use, whether its `Deserialize` is derived or not.
    fn nested<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        if self.depth >= self.max_depth {
            return Err(BoxedError::new(Bincode1Error::TooDeep {
                max_depth: self.max_depth,
            }));
        }
        self.depth += 1;
        let out = f(self);
        self.depth -= 1;
        out
    }

    pub(super) const fn position(&self) -> usize {
        self.pos
    }

    /// The next `n` bytes: the end of the input is checked first, then the limit.
    fn take(&mut self, n: usize) -> Result<&'de [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.input.len())
            .ok_or(Bincode1Error::UnexpectedEnd)?;
        if end > self.limit {
            return Err(BoxedError::new(Bincode1Error::LimitExceeded {
                limit: self.limit,
            }));
        }
        let bytes = &self.input[self.pos..end];
        self.pos = end;
        Ok(bytes)
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    fn read_u8(&mut self) -> Result<u8> {
        Ok(self.take_array::<1>()?[0])
    }

    fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take_array()?))
    }

    /// A `u64` length or count, accepted only when that many bytes (at least one
    /// per element) remain in the input and under the limit, so whatever it sizes is
    /// no larger than the bytes the decode may still read.
    fn read_len(&mut self) -> Result<usize> {
        let len = u64::from_le_bytes(self.take_array()?);
        let n = len_within(len, 1, self.input.len() - self.pos)?;
        if n > self.limit.saturating_sub(self.pos) {
            return Err(BoxedError::new(Bincode1Error::LimitExceeded {
                limit: self.limit,
            }));
        }
        Ok(n)
    }
}

/// The width of a UTF-8 sequence from its first byte; 0 for a byte that cannot start one.
const fn utf8_width(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => 0,
    }
}

macro_rules! fixed_int {
    ($de:ident, $visit:ident, $ty:ty) => {
        fn $de<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
            visitor.$visit(<$ty>::from_le_bytes(self.take_array()?))
        }
    };
}

impl<'de> de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = BoxedError;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(BoxedError::new(Bincode1Error::DeserializeAnyNotSupported))
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.read_u8()? {
            0 => visitor.visit_bool(false),
            1 => visitor.visit_bool(true),
            b => Err(BoxedError::new(Bincode1Error::InvalidBool(b))),
        }
    }

    fixed_int!(deserialize_i8, visit_i8, i8);
    fixed_int!(deserialize_i16, visit_i16, i16);
    fixed_int!(deserialize_i32, visit_i32, i32);
    fixed_int!(deserialize_i64, visit_i64, i64);
    fixed_int!(deserialize_i128, visit_i128, i128);
    fixed_int!(deserialize_u8, visit_u8, u8);
    fixed_int!(deserialize_u16, visit_u16, u16);
    fixed_int!(deserialize_u32, visit_u32, u32);
    fixed_int!(deserialize_u64, visit_u64, u64);
    fixed_int!(deserialize_u128, visit_u128, u128);

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_f32(f32::from_bits(u32::from_le_bytes(self.take_array()?)))
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_f64(f64::from_bits(u64::from_le_bytes(self.take_array()?)))
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let first = self.read_u8()?;
        let width = utf8_width(first);
        if width == 0 {
            return Err(BoxedError::new(Bincode1Error::InvalidChar));
        }
        let mut buf = [first, 0, 0, 0];
        if width > 1 {
            // input that ends inside the char (or the limit) is reported as such
            buf[1..width].copy_from_slice(self.take(width - 1)?);
        }
        let c = core::str::from_utf8(&buf[..width])
            .ok()
            .and_then(|s| s.chars().next())
            .ok_or(Bincode1Error::InvalidChar)?;
        visitor.visit_char(c)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.read_len()?;
        let bytes = self.take(len)?;
        let s = core::str::from_utf8(bytes).map_err(|_| Bincode1Error::InvalidUtf8)?;
        visitor.visit_borrowed_str(s)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.read_len()?;
        visitor.visit_borrowed_bytes(self.take(len)?)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.read_u8()? {
            0 => visitor.visit_none(),
            1 => self.nested(|de| visitor.visit_some(de)),
            t => Err(BoxedError::new(Bincode1Error::InvalidOptionTag(t))),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        self.nested(|de| visitor.visit_newtype_struct(de))
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.read_len()?;
        self.nested(|de| visitor.visit_seq(Access { de, len }))
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        self.nested(|de| visitor.visit_seq(Access { de, len }))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.read_len()?;
        self.nested(|de| visitor.visit_map(Access { de, len }))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_tuple(fields.len(), visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        self.nested(|de| visitor.visit_enum(de))
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(BoxedError::new(Bincode1Error::DeserializeAnyNotSupported))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(BoxedError::new(Bincode1Error::DeserializeAnyNotSupported))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// The elements of a tuple, struct or seq, or the entries of a map: `len` of them.
struct Access<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    len: usize,
}

impl<'de> de::SeqAccess<'de> for Access<'_, 'de> {
    type Error = BoxedError;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.len == 0 {
            return Ok(None);
        }
        self.len -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de> de::MapAccess<'de> for Access<'_, 'de> {
    type Error = BoxedError;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.len == 0 {
            return Ok(None);
        }
        self.len -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        seed.deserialize(&mut *self.de)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de> de::EnumAccess<'de> for &mut Deserializer<'de> {
    type Error = BoxedError;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self)> {
        let index = self.read_u32()?;
        let value = seed.deserialize(IntoDeserializer::<BoxedError>::into_deserializer(index))?;
        Ok((value, self))
    }
}

impl<'de> de::VariantAccess<'de> for &mut Deserializer<'de> {
    type Error = BoxedError;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self)
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_tuple(self, len, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        de::Deserializer::deserialize_tuple(self, fields.len(), visitor)
    }
}
