use glam::{Vec4, vec4};
use itertools::Itertools;

use crate::vertex::{BoneElementType, Bones, Vertices};

use super::{NudMesh, NudMeshGroup};

/// Data for a [NudMesh] and its parent [NudMeshGroup] to facilitate grouping meshes.
pub struct NudMeshGroupMesh {
    // TODO: document requirements for these fields?
    pub name: String,
    pub sort_bias: f32,
    pub parent_bone_index: Option<usize>,
    pub mesh: NudMesh,
}

// TODO: document sort bias
// TODO: test different sorting biases with the same group name in game?

/// Split `meshes` into [NudMeshGroup] based on the group name and parent bone.
///
/// The resulting [NudMeshGroup] will be sorted alphabetically by name and parent bone index.
///
/// This may modify the parent bone or skinning for a [NudMesh]
/// to ensure that all meshes in a [NudMeshGroup] use vertex skinning or all use a parent bone.
pub fn create_mesh_groups(meshes: &[NudMeshGroupMesh]) -> Vec<NudMeshGroup> {
    // TODO: apply parent bone optimization if needed
    // TODO: all groups with the same name should have the same sort bias?
    // TODO: calculate bounding spheres and overall bounding sphere

    // Sort to enable grouping later.
    // TODO: use IndexMap to preserve the ordering as much as possible?
    let mut meshes: Vec<_> = meshes.iter().collect();
    meshes.sort_by_key(|m| (&m.name, m.parent_bone_index));

    let mut groups = Vec::new();
    for (name, meshes) in &meshes.iter().chunk_by(|m| &m.name) {
        let group_meshes: Vec<_> = meshes.collect();

        // TODO: error if sort bias is not the same?
        // TODO: grouping by parent bone index assumes parent bone optimization is already applied?
        // TODO: if any mesh does not use a parent bone, convert parent bones to skin weights

        // Groups with the same name must have the same bone flags.
        // This means the parent bone can't be used if any meshes need skinning.
        // TODO: How to handle the case where parent bone is some but meshes have skinning?
        if group_meshes.iter().all(|m| m.parent_bone_index.is_some()) {
            // Split groups with the same name but different parent bones.
            for (parent_bone_index, split_meshes) in
                &group_meshes.iter().chunk_by(|m| m.parent_bone_index)
            {
                let split_meshes: Vec<_> = split_meshes.collect();
                groups.push(NudMeshGroup {
                    name: name.clone(),
                    meshes: split_meshes.iter().map(|m| m.mesh.clone()).collect(),
                    sort_bias: split_meshes
                        .first()
                        .map(|m| m.sort_bias)
                        .unwrap_or_default(),
                    bounding_sphere: Vec4::ZERO,
                    parent_bone_index,
                });
            }
        } else {
            // Force skinning for all meshes.
            groups.push(NudMeshGroup {
                name: name.clone(),
                meshes: group_meshes
                    .iter()
                    .map(|m| match m.parent_bone_index {
                        Some(p) => mesh_with_parent_bone_weights(m, p),
                        None => m.mesh.clone(),
                    })
                    .collect(),
                sort_bias: group_meshes
                    .first()
                    .map(|m| m.sort_bias)
                    .unwrap_or_default(),
                bounding_sphere: Vec4::ZERO,
                parent_bone_index: None,
            });
        }
    }
    groups
}

fn mesh_with_parent_bone_weights(m: &NudMeshGroupMesh, parent_bone_index: usize) -> NudMesh {
    // TODO: What element type to use?
    NudMesh {
        vertices: Vertices {
            bones: Some(Bones {
                bone_indices: vec![
                    [parent_bone_index as u32, 0, 0, 0];
                    m.mesh.vertices.positions.len()
                ],
                weights: vec![vec4(1.0, 0.0, 0.0, 0.0); m.mesh.vertices.positions.len()],
                element_type: BoneElementType::Byte,
            }),
            ..m.mesh.vertices.clone()
        },
        ..m.mesh.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        PrimitiveType,
        vertex::{BoneElementType, Bones, Normals, Uvs, Vertices},
    };

    use glam::{Vec3, vec4};

    fn nud_mesh(bone_indices: Option<[u32; 4]>, weights: Option<Vec4>) -> NudMesh {
        NudMesh {
            vertices: Vertices {
                positions: vec![Vec3::ZERO],
                normals: Normals::None(Vec::new()),
                bones: Some(Bones {
                    bone_indices: bone_indices
                        .map(|indices| vec![indices])
                        .unwrap_or_default(),
                    weights: weights.map(|weights| vec![weights]).unwrap_or_default(),
                    element_type: BoneElementType::Byte,
                }),
                colors: None,
                uvs: Uvs::Float16(Vec::new()),
            },
            vertex_indices: vec![0, 0, 0],
            primitive_type: PrimitiveType::TriangleList,
            material1: None,
            material2: None,
            material3: None,
            material4: None,
        }
    }

    #[test]
    fn create_mesh_groups_empty() {
        assert!(create_mesh_groups(&[]).is_empty());
    }

    #[test]
    fn create_mesh_groups_single() {
        // TODO: Test bounding spheres as well.
        assert_eq!(
            vec![NudMeshGroup {
                name: "a".to_string(),
                meshes: vec![nud_mesh(None, None)],
                sort_bias: 1.5,
                bounding_sphere: Vec4::ZERO,
                parent_bone_index: Some(2)
            }],
            create_mesh_groups(&[NudMeshGroupMesh {
                name: "a".to_string(),
                sort_bias: 1.5,
                parent_bone_index: Some(2),
                mesh: nud_mesh(None, None)
            }])
        );
    }

    #[test]
    fn create_mesh_groups_split_parent_bone() {
        // Test splitting groups with the same name but different parent bones.
        // TODO: Test bounding spheres as well.
        assert_eq!(
            vec![
                NudMeshGroup {
                    name: "a".to_string(),
                    meshes: vec![nud_mesh(None, None), nud_mesh(None, None)],
                    sort_bias: 1.5,
                    bounding_sphere: Vec4::ZERO,
                    parent_bone_index: Some(0)
                },
                NudMeshGroup {
                    name: "a".to_string(),
                    meshes: vec![nud_mesh(None, None)],
                    sort_bias: 1.5,
                    bounding_sphere: Vec4::ZERO,
                    parent_bone_index: Some(1)
                },
                NudMeshGroup {
                    name: "b".to_string(),
                    meshes: vec![nud_mesh(None, None)],
                    sort_bias: 1.5,
                    bounding_sphere: Vec4::ZERO,
                    parent_bone_index: Some(2)
                }
            ],
            create_mesh_groups(&[
                NudMeshGroupMesh {
                    name: "a".to_string(),
                    sort_bias: 1.5,
                    parent_bone_index: Some(0),
                    mesh: nud_mesh(None, None)
                },
                NudMeshGroupMesh {
                    name: "a".to_string(),
                    sort_bias: 1.5,
                    parent_bone_index: Some(0),
                    mesh: nud_mesh(None, None)
                },
                NudMeshGroupMesh {
                    name: "a".to_string(),
                    sort_bias: 1.5,
                    parent_bone_index: Some(1),
                    mesh: nud_mesh(None, None)
                },
                NudMeshGroupMesh {
                    name: "b".to_string(),
                    sort_bias: 1.5,
                    parent_bone_index: Some(2),
                    mesh: nud_mesh(None, None)
                }
            ])
        );
    }

    #[test]
    fn create_mesh_groups_mixed_skinning_parent_bone() {
        // Test converting parent bones to skin weights for consistent bone flags.
        // TODO: Test bounding spheres as well.
        assert_eq!(
            vec![NudMeshGroup {
                name: "a".to_string(),
                meshes: vec![
                    nud_mesh(Some([1, 2, 3, 4]), Some(vec4(0.5, 0.25, 0.125, 0.125))),
                    nud_mesh(Some([0, 0, 0, 0]), Some(vec4(1.0, 0.0, 0.0, 0.0))),
                    nud_mesh(Some([2, 0, 0, 0]), Some(vec4(1.0, 0.0, 0.0, 0.0)))
                ],
                sort_bias: 1.5,
                bounding_sphere: Vec4::ZERO,
                parent_bone_index: None
            },],
            create_mesh_groups(&[
                NudMeshGroupMesh {
                    name: "a".to_string(),
                    sort_bias: 1.5,
                    parent_bone_index: Some(0),
                    mesh: nud_mesh(None, None)
                },
                NudMeshGroupMesh {
                    name: "a".to_string(),
                    sort_bias: 1.5,
                    parent_bone_index: None,
                    mesh: nud_mesh(Some([1, 2, 3, 4]), Some(vec4(0.5, 0.25, 0.125, 0.125)))
                },
                NudMeshGroupMesh {
                    name: "a".to_string(),
                    sort_bias: 1.5,
                    parent_bone_index: Some(2),
                    mesh: nud_mesh(None, None)
                },
            ])
        );
    }

    // TODO: test case for applying parent bone optimization

    // TODO: test group with both parent bone and weights on an input mesh?
}
