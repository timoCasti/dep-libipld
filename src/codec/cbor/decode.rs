//! CBOR decoder
#![allow(missing_docs)]
use crate::error::Result;
use crate::ipld::Ipld;
use byteorder::{BigEndian, ByteOrder};
use cid::Cid;
use core::convert::TryFrom;
use failure::Fail;
use std::collections::BTreeMap;
use std::io::Read;

#[derive(Debug, Fail)]
pub enum CborError {
    #[fail(display = "Length out of range.")]
    LengthOutOfRange,
    #[fail(display = "Unexpected code.")]
    UnexpectedCode,
    #[fail(display = "Unkown tag.")]
    UnknownTag,
    #[fail(display = "{}", _0)]
    Io(std::io::Error),
}

impl From<std::io::Error> for CborError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

pub trait ReadExt {
    fn read_n(&mut self, len: usize) -> Result<Vec<u8>>;
}

impl<T: Read> ReadExt for T {
    fn read_n(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(len);
        let reader_ref = self.by_ref();
        let mut taken = reader_ref.take(len as u64);
        taken.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

#[inline]
pub fn read_u8<R: Read>(r: &mut R) -> Result<u8> {
    let mut buf = [0; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

#[inline]
pub fn read_u16<R: Read>(r: &mut R) -> Result<u16> {
    let mut buf = [0; 2];
    r.read_exact(&mut buf)?;
    Ok(BigEndian::read_u16(&buf))
}

#[inline]
pub fn read_u32<R: Read>(r: &mut R) -> Result<u32> {
    let mut buf = [0; 4];
    r.read_exact(&mut buf)?;
    Ok(BigEndian::read_u32(&buf))
}

#[inline]
pub fn read_u64<R: Read>(r: &mut R) -> Result<u64> {
    let mut buf = [0; 8];
    r.read_exact(&mut buf)?;
    Ok(BigEndian::read_u64(&buf))
}

#[inline]
pub fn read_f32<R: Read>(r: &mut R) -> Result<f32> {
    let mut buf = [0; 4];
    r.read_exact(&mut buf)?;
    Ok(BigEndian::read_f32(&buf))
}

#[inline]
pub fn read_f64<R: Read>(r: &mut R) -> Result<f64> {
    let mut buf = [0; 8];
    r.read_exact(&mut buf)?;
    Ok(BigEndian::read_f64(&buf))
}

#[inline]
pub fn read_bytes<R: Read>(r: &mut R, len: usize) -> Result<Vec<u8>> {
    r.read_n(len)
}

#[inline]
pub fn read_str<R: Read>(r: &mut R, len: usize) -> Result<String> {
    let bytes = r.read_n(len)?;
    let string = std::str::from_utf8(&bytes)?;
    Ok(string.to_string())
}

#[inline]
pub fn read_list<R: Read, T: ReadCbor>(r: &mut R, len: usize) -> Result<Vec<T>> {
    let mut list: Vec<T> = Vec::with_capacity(len);
    for _ in 0..len {
        list.push(T::read_cbor(r)?);
    }
    Ok(list)
}

#[inline]
pub fn read_map<R: Read, T: ReadCbor>(r: &mut R, len: usize) -> Result<BTreeMap<String, T>> {
    let mut map: BTreeMap<String, T> = BTreeMap::new();
    for _ in 0..len {
        let key = String::read_cbor(r)?;
        let value = T::read_cbor(r)?;
        map.insert(key, value);
    }
    Ok(map)
}

#[inline]
pub fn read_link<R: Read>(r: &mut R) -> Result<Cid> {
    let tag = read_u8(r)?;
    if tag != 42 {
        return Err(CborError::UnknownTag.into());
    }
    let ty = read_u8(r)?;
    if ty != 0x58 {
        return Err(CborError::UnknownTag.into());
    }
    let len = read_u8(r)?;
    let bytes = read_bytes(r, len as usize)?;
    Ok(Cid::try_from(bytes)?)
}

pub trait ReadCbor: Sized {
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self>;
}

impl ReadCbor for bool {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        match read_u8(r)? {
            0xf4 => Ok(false),
            0xf5 => Ok(true),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl ReadCbor for u8 {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0x00..=0x17 => Ok(major),
            0x18 => read_u8(r),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl ReadCbor for u16 {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0x00..=0x17 => Ok(major as u16),
            0x18 => Ok(read_u8(r)? as u16),
            0x19 => read_u16(r),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl ReadCbor for u32 {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0x00..=0x17 => Ok(major as u32),
            0x18 => Ok(read_u8(r)? as u32),
            0x19 => Ok(read_u16(r)? as u32),
            0x1a => read_u32(r),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl ReadCbor for u64 {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0x00..=0x17 => Ok(major as u64),
            0x18 => Ok(read_u8(r)? as u64),
            0x19 => Ok(read_u16(r)? as u64),
            0x1a => Ok(read_u32(r)? as u64),
            0x1b => read_u64(r),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl ReadCbor for i8 {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0x20..=0x37 => Ok(-1 - (major - 0x20) as i8),
            0x38 => Ok(-1 - read_u8(r)? as i8),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl ReadCbor for i16 {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0x20..=0x37 => Ok(-1 - (major - 0x20) as i16),
            0x38 => Ok(-1 - read_u8(r)? as i16),
            0x39 => Ok(-1 - read_u16(r)? as i16),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl ReadCbor for i32 {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0x20..=0x37 => Ok(-1 - (major - 0x20) as i32),
            0x38 => Ok(-1 - read_u8(r)? as i32),
            0x39 => Ok(-1 - read_u16(r)? as i32),
            0x3a => Ok(-1 - read_u32(r)? as i32),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl ReadCbor for i64 {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0x20..=0x37 => Ok(-1 - (major - 0x20) as i64),
            0x38 => Ok(-1 - read_u8(r)? as i64),
            0x39 => Ok(-1 - read_u16(r)? as i64),
            0x3a => Ok(-1 - read_u32(r)? as i64),
            0x3b => Ok(-1 - read_u64(r)? as i64),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl ReadCbor for f32 {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0xfa => read_f32(r),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl ReadCbor for f64 {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0xfa => Ok(read_f32(r)? as f64),
            0xfb => read_f64(r),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

/*impl ReadCbor for Vec<u8> {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}*/

impl ReadCbor for String {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        let len = match major {
            0x60..=0x77 => major as usize - 0x60,
            0x78 => read_u8(r)? as usize,
            0x79 => read_u16(r)? as usize,
            0x7a => read_u32(r)? as usize,
            0x7b => {
                let len = read_u64(r)?;
                if len > usize::max_value() as u64 {
                    return Err(CborError::LengthOutOfRange.into());
                }
                len as usize
            }
            _ => return Err(CborError::UnexpectedCode.into()),
        };
        read_str(r, len)
    }
}

impl ReadCbor for Cid {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0xd8 => read_link(r),
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl<T: ReadCbor> ReadCbor for Option<T> {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        match major {
            0xf6 => Ok(None),
            0xf7 => Ok(None),
            // TODO decode
            _ => return Err(CborError::UnexpectedCode.into()),
        }
    }
}

impl<T: ReadCbor> ReadCbor for Vec<T> {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        let len = match major {
            0x80..=0x97 => major as usize - 0x80,
            0x98 => read_u8(r)? as usize,
            0x99 => read_u16(r)? as usize,
            0x9a => read_u32(r)? as usize,
            0x9b => {
                let len = read_u64(r)?;
                if len > usize::max_value() as u64 {
                    return Err(CborError::LengthOutOfRange.into());
                }
                len as usize
            }
            _ => return Err(CborError::UnexpectedCode.into()),
        };
        read_list(r, len)
    }
}

impl<T: ReadCbor> ReadCbor for BTreeMap<String, T> {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        let len = match major {
            0xa0..=0xb7 => major as usize - 0xa0,
            0xb8 => read_u8(r)? as usize,
            0xb9 => read_u16(r)? as usize,
            0xba => read_u32(r)? as usize,
            0xbb => {
                let len = read_u64(r)?;
                if len > usize::max_value() as u64 {
                    return Err(CborError::LengthOutOfRange.into());
                }
                len as usize
            }
            _ => return Err(CborError::UnexpectedCode.into()),
        };
        read_map(r, len)
    }
}

impl ReadCbor for Ipld {
    #[inline]
    fn read_cbor<R: Read>(r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        let ipld = match major {
            // Major type 0: an unsigned integer
            0x00..=0x17 => Ipld::Integer(major as i128),
            0x18 => Ipld::Integer(read_u8(r)? as i128),
            0x19 => Ipld::Integer(read_u16(r)? as i128),
            0x1a => Ipld::Integer(read_u32(r)? as i128),
            0x1b => Ipld::Integer(read_u64(r)? as i128),

            // Major type 1: a negative integer
            0x20..=0x37 => Ipld::Integer(-1 - (major - 0x20) as i128),
            0x38 => Ipld::Integer(-1 - read_u8(r)? as i128),
            0x39 => Ipld::Integer(-1 - read_u16(r)? as i128),
            0x3a => Ipld::Integer(-1 - read_u32(r)? as i128),
            0x3b => Ipld::Integer(-1 - read_u64(r)? as i128),

            // Major type 2: a byte string
            0x40..=0x57 => {
                let len = major - 0x40;
                let bytes = read_bytes(r, len as usize)?;
                Ipld::Bytes(bytes)
            }
            0x58 => {
                let len = read_u8(r)?;
                let bytes = read_bytes(r, len as usize)?;
                Ipld::Bytes(bytes)
            }
            0x59 => {
                let len = read_u16(r)?;
                let bytes = read_bytes(r, len as usize)?;
                Ipld::Bytes(bytes)
            }
            0x5a => {
                let len = read_u32(r)?;
                let bytes = read_bytes(r, len as usize)?;
                Ipld::Bytes(bytes)
            }
            0x5b => {
                let len = read_u64(r)?;
                if len > usize::max_value() as u64 {
                    return Err(CborError::LengthOutOfRange.into());
                }
                let bytes = read_bytes(r, len as usize)?;
                Ipld::Bytes(bytes)
            }

            // Major type 3: a text string
            0x60..=0x77 => {
                let len = major - 0x60;
                let string = read_str(r, len as usize)?;
                Ipld::String(string)
            }
            0x78 => {
                let len = read_u8(r)?;
                let string = read_str(r, len as usize)?;
                Ipld::String(string)
            }
            0x79 => {
                let len = read_u16(r)?;
                let string = read_str(r, len as usize)?;
                Ipld::String(string)
            }
            0x7a => {
                let len = read_u32(r)?;
                let string = read_str(r, len as usize)?;
                Ipld::String(string)
            }
            0x7b => {
                let len = read_u64(r)?;
                if len > usize::max_value() as u64 {
                    return Err(CborError::LengthOutOfRange.into());
                }
                let string = read_str(r, len as usize)?;
                Ipld::String(string)
            }

            // Major type 4: an array of data items
            0x80..=0x97 => {
                let len = major - 0x80;
                let list = read_list(r, len as usize)?;
                Ipld::List(list)
            }
            0x98 => {
                let len = read_u8(r)?;
                let list = read_list(r, len as usize)?;
                Ipld::List(list)
            }
            0x99 => {
                let len = read_u16(r)?;
                let list = read_list(r, len as usize)?;
                Ipld::List(list)
            }
            0x9a => {
                let len = read_u32(r)?;
                let list = read_list(r, len as usize)?;
                Ipld::List(list)
            }
            0x9b => {
                let len = read_u64(r)?;
                if len > usize::max_value() as u64 {
                    return Err(CborError::LengthOutOfRange.into());
                }
                let list = read_list(r, len as usize)?;
                Ipld::List(list)
            }

            // Major type 5: a map of pairs of data items
            0xa0..=0xb7 => {
                let len = major - 0xa0;
                let map = read_map(r, len as usize)?;
                Ipld::Map(map)
            }
            0xb8 => {
                let len = read_u8(r)?;
                let map = read_map(r, len as usize)?;
                Ipld::Map(map)
            }
            0xb9 => {
                let len = read_u16(r)?;
                let map = read_map(r, len as usize)?;
                Ipld::Map(map)
            }
            0xba => {
                let len = read_u32(r)?;
                let map = read_map(r, len as usize)?;
                Ipld::Map(map)
            }
            0xbb => {
                let len = read_u64(r)?;
                if len > usize::max_value() as u64 {
                    return Err(CborError::LengthOutOfRange.into());
                }
                let map = read_map(r, len as usize)?;
                Ipld::Map(map)
            }

            // Major type 6: optional semantic tagging of other major types
            0xd8 => Ipld::Link(read_link(r)?),

            // Major type 7: floating-point numbers and other simple data types that need no content
            0xf4 => Ipld::Bool(false),
            0xf5 => Ipld::Bool(true),
            0xf6 => Ipld::Null,
            0xf7 => Ipld::Null,
            0xfa => Ipld::Float(read_f32(r)? as f64),
            0xfb => Ipld::Float(read_f64(r)?),
            _ => return Err(CborError::UnexpectedCode.into()),
        };
        Ok(ipld)
    }
}
