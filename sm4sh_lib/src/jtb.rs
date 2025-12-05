use binrw::{BinRead, BinWrite};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct Jtb {
    pub count1: u16,
    pub count2: u16,

    // TODO: Not all files have enough data?
    #[br(count = count1)]
    pub items1: Vec<u16>,

    #[br(count = count2)]
    pub items2: Vec<u16>,
}
