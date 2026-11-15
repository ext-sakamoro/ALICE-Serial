//! The serde serializer for the bincode 1 default layout.

use alloc::vec::Vec;

use serde::ser::{self, Serialize};

use super::Bincode1Error;

type Result<T> = core::result::Result<T, Bincode1Error>;

/// Appends to `out`, never past `limit` bytes from where it started.
pub(super) struct Serializer<'a> {
    out: &'a mut Vec<u8>,
    start: usize,
    limit: usize,
    /// For each sequence or map being written: its element count and where its
    /// elements start, to refuse one whose elements take less than a byte each.
    open: Vec<(usize, usize)>,
}

impl<'a> Serializer<'a> {
    pub(super) const fn new(out: &'a mut Vec<u8>, limit: usize) -> Self {
        let start = out.len();
        Self {
            out,
            start,
            limit,
            open: Vec::new(),
        }
    }

    /// Start a sequence or map of `len` elements.
    fn open(&mut self, len: usize) -> Result<()> {
        self.write_len(len)?;
        self.open.push((len, self.out.len()));
        Ok(())
    }

    /// End the innermost sequence or map: a decode needs a byte per element to bound
    /// the count, so elements written in fewer bytes than their count are refused.
    fn close(&mut self) -> Result<()> {
        let Some((count, at)) = self.open.pop() else {
            return Ok(());
        };
        if self.out.len() - at < count {
            return Err(Bincode1Error::ZeroSizeElements { count });
        }
        Ok(())
    }

    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let written = self.out.len() - self.start;
        match written.checked_add(bytes.len()) {
            Some(n) if n <= self.limit => {
                self.out.extend_from_slice(bytes);
                Ok(())
            }
            _ => Err(Bincode1Error::LimitExceeded { limit: self.limit }),
        }
    }

    fn write_len(&mut self, len: usize) -> Result<()> {
        self.write(&(len as u64).to_le_bytes())
    }
}

impl ser::Serializer for &mut Serializer<'_> {
    type Ok = ();
    type Error = Bincode1Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.write(&[u8::from(v)])
    }
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.write(&v.to_le_bytes())
    }
    fn serialize_i16(self, v: i16) -> Result<()> {
        self.write(&v.to_le_bytes())
    }
    fn serialize_i32(self, v: i32) -> Result<()> {
        self.write(&v.to_le_bytes())
    }
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.write(&v.to_le_bytes())
    }
    fn serialize_i128(self, v: i128) -> Result<()> {
        self.write(&v.to_le_bytes())
    }
    fn serialize_u8(self, v: u8) -> Result<()> {
        self.write(&[v])
    }
    fn serialize_u16(self, v: u16) -> Result<()> {
        self.write(&v.to_le_bytes())
    }
    fn serialize_u32(self, v: u32) -> Result<()> {
        self.write(&v.to_le_bytes())
    }
    fn serialize_u64(self, v: u64) -> Result<()> {
        self.write(&v.to_le_bytes())
    }
    fn serialize_u128(self, v: u128) -> Result<()> {
        self.write(&v.to_le_bytes())
    }
    fn serialize_f32(self, v: f32) -> Result<()> {
        self.write(&v.to_bits().to_le_bytes())
    }
    fn serialize_f64(self, v: f64) -> Result<()> {
        self.write(&v.to_bits().to_le_bytes())
    }
    fn serialize_char(self, v: char) -> Result<()> {
        let mut buf = [0u8; 4];
        self.write(v.encode_utf8(&mut buf).as_bytes())
    }
    fn serialize_str(self, v: &str) -> Result<()> {
        self.write_len(v.len())?;
        self.write(v.as_bytes())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.write_len(v.len())?;
        self.write(v)
    }
    fn serialize_none(self) -> Result<()> {
        self.write(&[0])
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<()> {
        self.write(&[1])?;
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.serialize_u32(variant_index)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()> {
        self.serialize_u32(variant_index)?;
        value.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self> {
        let len = len.ok_or(Bincode1Error::SequenceMustHaveLength)?;
        self.open(len)?;
        Ok(self)
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self> {
        self.serialize_u32(variant_index)?;
        Ok(self)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self> {
        let len = len.ok_or(Bincode1Error::SequenceMustHaveLength)?;
        self.open(len)?;
        Ok(self)
    }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self> {
        self.serialize_u32(variant_index)?;
        Ok(self)
    }
    fn is_human_readable(&self) -> bool {
        false
    }
}

impl ser::SerializeSeq for &mut Serializer<'_> {
    type Ok = ();
    type Error = Bincode1Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        self.close()
    }
}

impl ser::SerializeTuple for &mut Serializer<'_> {
    type Ok = ();
    type Error = Bincode1Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut Serializer<'_> {
    type Ok = ();
    type Error = Bincode1Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeTupleVariant for &mut Serializer<'_> {
    type Ok = ();
    type Error = Bincode1Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeMap for &mut Serializer<'_> {
    type Ok = ();
    type Error = Bincode1Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<()> {
        key.serialize(&mut **self)
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        self.close()
    }
}

impl ser::SerializeStruct for &mut Serializer<'_> {
    type Ok = ();
    type Error = Bincode1Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut Serializer<'_> {
    type Ok = ();
    type Error = Bincode1Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        Ok(())
    }
}
