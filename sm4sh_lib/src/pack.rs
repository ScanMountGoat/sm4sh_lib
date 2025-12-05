use std::io::{Read, Seek, SeekFrom};

use crate::parse_string_ptr32;
use binrw::{BinRead, BinWrite, binrw};
use xc3_write::Offset;

#[binrw]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
#[brw(magic(b"KCAP"))]
pub struct Pack {
    #[br(temp)]
    #[bw(calc = 0)]
    _unk1: u32,

    #[br(temp)]
    #[bw(calc = items.len() as u32)]
    count: u32,

    #[br(temp)]
    #[bw(calc = 0)]
    _unk2: u32,

    #[br(temp, count = count)]
    #[bw(ignore)]
    names: Vec<StringPtr>,

    #[br(temp, count = count)]
    #[bw(ignore)]
    offsets: Vec<u32>,

    #[br(temp, count = count)]
    #[bw(ignore)]
    sizes: Vec<u32>,

    #[br(parse_with = read_items, args_raw((&names, &offsets, &sizes)))]
    #[bw(write_with = write_items)]
    pub items: Vec<PackItem>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead)]
struct StringPtr(#[br(parse_with = parse_string_ptr32)] String);

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct PackItem {
    pub name: String,
    pub data: Vec<u8>,
}

fn read_items<R: Read + Seek>(
    reader: &mut R,
    _endian: binrw::Endian,
    args: (&[StringPtr], &[u32], &[u32]),
) -> binrw::BinResult<Vec<PackItem>> {
    let (names, offsets, sizes) = args;
    let mut items = Vec::new();
    for ((name, offset), size) in names.iter().zip(offsets).zip(sizes) {
        reader.seek(SeekFrom::Start(*offset as u64))?;
        let mut data = vec![0u8; *size as usize];
        reader.read_exact(&mut data)?;
        items.push(PackItem {
            name: name.0.clone(),
            data,
        });
    }
    Ok(items)
}

fn write_items<W: std::io::Write + Seek>(
    items: &Vec<PackItem>,
    writer: &mut W,
    endian: binrw::Endian,
    _args: (),
) -> binrw::BinResult<()> {
    // Names
    let mut name_offsets = Vec::new();
    for item in items {
        let offset = Offset::<u32, _>::new(writer.stream_position()?, &item.name, None, 0u8);
        0u32.write_options(writer, endian, ())?;
        name_offsets.push(offset);
    }

    // Offsets
    let mut data_offsets = Vec::new();
    for item in items {
        // Data offsets are aligned to 16.
        let offset = Offset::<u32, _>::new(writer.stream_position()?, &item.data, Some(16), 0u8);
        0u32.write_options(writer, endian, ())?;
        data_offsets.push(offset);
    }

    // Sizes
    for item in items {
        (item.data.len() as u32).write_options(writer, endian, ())?;
    }

    let endian = match endian {
        binrw::Endian::Big => xc3_write::Endian::Big,
        binrw::Endian::Little => xc3_write::Endian::Little,
    };

    let mut data_ptr = 0;

    for offset in name_offsets {
        offset.write(writer, 0, &mut data_ptr, endian)?;
    }

    for offset in data_offsets {
        offset.write(writer, 0, &mut data_ptr, endian)?;
    }

    Ok(())
}
