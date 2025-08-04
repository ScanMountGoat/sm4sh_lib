use binrw::{args, binread, helpers::until_eof, io::TakeSeekExt, BinRead, FilePtr32};

// TODO: namco helper bones?
// TODO: NHB for big endian?
// TODO: Write support.
#[binread]
#[derive(Debug, PartialEq, Clone)]
#[br(magic(b" BHN"))]
#[br(little)]
pub struct Nhb {
    pub count: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub data_count: u32,
    pub bone_count: u32,

    pub hash_count: u32,
    // TODO: hash section size, id 8?
    #[br(parse_with = FilePtr32::parse, offset = 40)]
    #[br(args { inner: args! { count: hash_count as usize }})]
    pub hashes: Vec<u32>,

    pub unk5: u32,

    #[br(count = data_count)]
    pub unk6: Vec<Data>,
}

#[derive(Debug, BinRead, PartialEq, Clone)]
pub struct Data {
    pub size: u32,

    #[br(assert(id == 2))]
    pub id: u32,

    // Subtract size of size and id fields.
    #[br(map_stream = |r| r.take_seek(size.saturating_sub(8) as u64))]
    #[br(parse_with = until_eof)]
    items: Vec<DataItem>,
}

// TODO: constraint data?
#[derive(Debug, BinRead, PartialEq, Clone)]
pub struct DataItem {
    pub size: u32,

    // Subtract size of size field but leave room for the id field.
    #[br(map_stream = |r| r.take_seek(size.saturating_sub(4).max(4) as u64))]
    pub inner: DataItemInner,
}

#[derive(Debug, BinRead, PartialEq, Clone)]
pub enum DataItemInner {
    // TODO: Why does this reach a recursion limit and not compile?
    // #[br(magic(2u32))]
    // Unk2 {
    //     #[br(parse_with = until_eof)]
    //     items: Vec<DataItem>,
    // },
    #[br(magic(3u32))]
    Unk3 {
        #[br(parse_with = until_eof)]
        items: Vec<u32>,
    },

    #[br(magic(4u32))]
    Unk4 {
        #[br(parse_with = until_eof)]
        items: Vec<i16>,
    },

    #[br(magic(5u32))]
    Unk5 {
        #[br(parse_with = until_eof)]
        items: Vec<u16>,
    },

    #[br(magic(6u32))]
    Unk6 {
        #[br(parse_with = until_eof)]
        items: Vec<u32>,
    },

    #[br(magic(7u32))]
    Unk7 {
        #[br(parse_with = until_eof)]
        items: Vec<f32>,
    },

    #[br(magic(257u32))]
    Unk257,
}
