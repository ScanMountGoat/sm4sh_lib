use sm4sh_lib::nud::{MeshGroup, Nud};
use vertex::Vertices;

pub mod vertex;

#[derive(Debug, PartialEq, Clone)]
pub struct NudModel {
    pub groups: Vec<NudMeshGroup>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NudMeshGroup {
    pub meshes: Vec<NudMesh>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NudMesh {
    // TODO: Assume meshes have unique vertices and put data here?
    pub vertices: Vertices,
    pub vertex_indices: Vec<u16>,
    // TODO: material?
}

impl NudModel {
    pub fn from_nud(nud: &Nud) -> Self {
        Self {
            groups: nud
                .mesh_groups
                .iter()
                .map(|g| NudMeshGroup { meshes: todo!() })
                .collect(),
        }
    }

    pub fn to_nud(&self) -> Nud {
        Nud {
            file_size: todo!(),
            version: todo!(),
            mesh_group_count: self.groups.len() as u16,
            bone_start_index: todo!(),
            bone_end_index: todo!(),
            indices_offset: todo!(),
            indices_size: todo!(),
            vertex_buffer0_size: todo!(),
            vertex_buffer1_size: todo!(),
            bounding_sphere: todo!(),
            mesh_groups: self
                .groups
                .iter()
                .map(|g| MeshGroup {
                    bounding_sphere: todo!(),
                    center: todo!(),
                    sort_bias: todo!(),
                    name: todo!(),
                    unk1: todo!(),
                    bone_flag: todo!(),
                    parent_bone_index: todo!(),
                    mesh_count: todo!(),
                    position: todo!(),
                })
                .collect(),
            meshes: todo!(),
            index_buffer: todo!(),
            vertex_buffer0: todo!(),
            vertex_buffer1: todo!(),
        }
    }
}
