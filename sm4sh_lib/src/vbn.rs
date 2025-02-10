use binrw::{BinRead, BinWrite, NullString};

// TODO: Better naming
// TODO: Find a better way to detect endianness.
#[derive(Debug, BinRead, BinWrite)]
pub enum Vbn {
    #[brw(magic(b" NBV"))]
    Le(#[brw(little)] VbnInner),

    #[brw(magic(b"VBN "))]
    Be(#[brw(big)] VbnInner),
}

#[derive(Debug, BinRead, BinWrite)]
pub struct VbnInner {
    pub version: u32,
    pub total_bone_count: u32,
    /// Number of [Bone] in [bones](#structfield.bones) for each type in [BoneType].
    pub bone_count_per_type: [u32; 4],
    #[br(count = total_bone_count)]
    pub bones: Vec<Bone>,
    #[br(count = total_bone_count)]
    pub transforms: Vec<BoneTransform>,
}

#[derive(Debug, BinRead, BinWrite)]
pub struct Bone {
    #[br(map = |x: NullString| x.to_string())]
    #[bw(map = |x| NullString::from(x.as_str()))]
    #[brw(pad_size_to = 64)]
    pub name: String,
    pub bone_type: BoneType,
    pub parent_bone_index: i32, // TODO: is this really 4 bytes?
    pub bone_id: u32,           // TODO: hash?
}

#[derive(Debug, BinRead, BinWrite)]
pub struct BoneTransform {
    pub position: [f32; 3],
    pub rotation: [f32; 3], // TODO: xyz_euler?
    pub scale: [f32; 3],
}

#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u32))]
pub enum BoneType {
    Normal = 0,
    Follow = 1,
    /// HLP_
    Helper = 2,
    Swing = 3,
}
