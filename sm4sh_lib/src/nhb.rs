use std::io::SeekFrom;

use binrw::{BinRead, BinWrite, binread, helpers::until_eof, io::TakeSeekExt};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{parse_ptr32_count, xc3_write_binwrite_impl};

// TODO: namco helper bones?
// TODO: NHB for big endian?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(magic(b" BHN"))]
#[br(little)]
#[xc3(magic(b" BHN"))]
pub struct Nhb {
    pub count: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub data_count: u32,
    pub helper_bone_count: u32,

    pub hash_count: u32,
    #[br(parse_with = parse_ptr32_count(hash_count as usize), offset = 40)]
    #[xc3(offset(u32))]
    pub hashes: Vec<u32>,

    pub unk5: u32,

    #[br(count = data_count)]
    pub items: Vec<Data>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct Data {
    pub size: u32,

    #[br(assert(id == 2))]
    pub id: u32,

    // Subtract size of size and id fields.
    #[br(map_stream = |r| r.take_seek(size.saturating_sub(8) as u64))]
    #[br(parse_with = until_eof)]
    pub items: Vec<DataItem>,
}

// TODO: constraint data?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct DataItem {
    pub size: u32,

    // Subtract size of size field but leave room for the id field.
    #[br(map_stream = |r| r.take_seek(size.saturating_sub(4).max(4) as u64))]
    pub inner: DataItemInner,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub enum DataItemInner {
    // TODO: Why does this reach a recursion limit and not compile?
    // #[brw(magic(2u32))]
    // Unk2 {
    //     #[br(parse_with = until_eof)]
    //     items: Vec<DataItem>,
    // },
    #[brw(magic(3u32))]
    Unk3 {
        #[br(parse_with = until_eof)]
        items: Vec<u32>,
    },

    // TODO: Indices into hashes?
    #[brw(magic(4u32))]
    Unk4 {
        #[br(parse_with = until_eof)]
        items: Vec<(i16, i16)>,
    },

    // TODO: Indices into hashes?
    #[brw(magic(5u32))]
    Unk5 {
        #[br(parse_with = until_eof)]
        items: Vec<(i16, i16)>,
    },

    #[brw(magic(6u32))]
    Unk6 {
        #[br(parse_with = until_eof)]
        items: Vec<u32>,
    },

    // TODO: size 24 is quaternion?
    // TODO: size 40 is ???
    #[brw(magic(7u32))]
    Unk7 {
        #[br(parse_with = until_eof)]
        items: Vec<u32>,
    },

    // TODO: always empty?
    #[brw(magic(257u32))]
    Unk257,
}

xc3_write_binwrite_impl!(Data, DataItem);

impl Xc3WriteOffsets for NhbOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // The end of the file is always empty, hash section, empty.
        writer.seek(SeekFrom::Start(*data_ptr))?;
        let empty = DataItem {
            size: 0,
            inner: DataItemInner::Unk257,
        };
        empty.xc3_write(writer, endian)?;
        (self.hashes.data.len() as u32 * 4 + 8).xc3_write(writer, endian)?;
        8u32.xc3_write(writer, endian)?;
        *data_ptr = writer.stream_position()?;

        self.hashes
            .write_full(writer, base_offset + 40, data_ptr, endian, ())?;

        writer.seek(SeekFrom::Start(*data_ptr))?;
        let empty = DataItem {
            size: 0,
            inner: DataItemInner::Unk257,
        };
        empty.xc3_write(writer, endian)?;

        Ok(())
    }
}
