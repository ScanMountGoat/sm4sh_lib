use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
    path::Path,
};

use binrw::{BinReaderExt, BinResult};
use glam::{vec3, EulerRot, Mat4, Quat, Vec3};
use sm4sh_lib::{
    omo::{Omo, OmoNode, PositionType, RotationType, ScaleType},
    pack::Pack,
};

use crate::VbnSkeleton;

/// Load animations from a `path` like `"main.pac"`.
pub fn load_animations<P: AsRef<Path>>(path: P) -> BinResult<Vec<(String, Animation)>> {
    let mut animations = Vec::new();
    let pac = Pack::from_file(path)?;
    for item in pac.items {
        if item.name.ends_with(".omo") {
            let omo = Omo::from_bytes(&item.data)?;
            let animation = Animation::from_omo(&omo)?;
            animations.push((item.name, animation));
        }
    }
    Ok(animations)
}

#[derive(Debug, PartialEq, Clone)]
pub struct Animation {
    pub nodes: Vec<AnimationNode>,
    pub frame_count: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AnimationNode {
    pub hash: u32,
    pub translation_keyframes: Vec<Option<Vec3>>,
    pub rotation_keyframes: Vec<Option<Quat>>,
    pub scale_keyframes: Vec<Option<Vec3>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FCurves {
    // TODO: also store keyframes?
    // TODO: methods to return values per channel to work efficiently in Blender?
    /// Translation keyframes for each bone hash.
    pub translation: BTreeMap<u32, Vec<Vec3>>,
    /// Rotation keyframes for each bone hash.
    pub rotation: BTreeMap<u32, Vec<Quat>>,
    /// Scale keyframes for each bone hash.
    pub scale: BTreeMap<u32, Vec<Vec3>>,
}

impl Animation {
    pub fn from_omo(omo: &Omo) -> BinResult<Self> {
        let mut nodes = Vec::new();
        for node in &omo.nodes {
            let data = omo_node_data(node, &omo.inter_data)?;

            // TODO: Find a nicer way to select key data for each frame.
            let mut translation_keyframes = Vec::new();
            let mut rotation_keyframes = Vec::new();
            let mut scale_keyframes = Vec::new();

            for frame in &omo.frames {
                // Convert a byte offset to an index for u16s.
                let mut key_index = node.key_offset as usize / 2;
                translation_keyframes.push(data.translation(&frame.keys, &mut key_index));
                rotation_keyframes.push(data.rotation(&frame.keys, &mut key_index));
                scale_keyframes.push(data.scale(&frame.keys, &mut key_index));
            }

            let animation_node = AnimationNode {
                hash: node.hash,
                translation_keyframes,
                rotation_keyframes,
                scale_keyframes,
            };
            nodes.push(animation_node);
        }

        Ok(Self {
            nodes,
            frame_count: omo.frame_count as usize,
        })
    }

    /// Compute the the animated transform in model space for each bone in `skeleton`.
    ///
    /// See [VbnSkeleton::model_space_transforms] for the transforms without animations applied.
    pub fn model_space_transforms(&self, skeleton: &VbnSkeleton, frame: f32) -> Vec<Mat4> {
        let mut final_transforms: Vec<_> = skeleton
            .bones
            .iter()
            .map(|b| {
                if let Some(node) = self.nodes.iter().find(|n| n.hash == b.hash) {
                    let translation = node.sample_translation(frame).unwrap_or(b.translation);

                    let rotation = node
                        .sample_rotation(frame)
                        .map(Mat4::from_quat)
                        .unwrap_or_else(|| {
                            Mat4::from_euler(
                                EulerRot::XYZEx,
                                b.rotation.x,
                                b.rotation.y,
                                b.rotation.z,
                            )
                        });

                    let scale = node.sample_scale(frame).unwrap_or(b.scale);

                    Mat4::from_translation(translation) * rotation * Mat4::from_scale(scale)
                } else {
                    b.matrix()
                }
            })
            .collect();

        // TODO: Don't assume bones appear after their parents.
        for i in 0..final_transforms.len() {
            if let Some(parent) = skeleton.bones[i].parent_bone_index {
                final_transforms[i] = final_transforms[parent] * final_transforms[i];
            }
        }

        final_transforms
    }

    /// Identical to [Self::model_space_transforms] but each transform is relative to the parent bone's transform.
    pub fn local_space_transforms(&self, skeleton: &VbnSkeleton, frame: f32) -> Vec<Mat4> {
        let transforms = self.model_space_transforms(skeleton, frame);
        transforms
            .iter()
            .zip(skeleton.bones.iter())
            .map(|(transform, bone)| match bone.parent_bone_index {
                Some(p) => transforms[p].inverse() * transform,
                None => *transform,
            })
            .collect()
    }

    /// Compute the matrix for each bone in `skeleton`
    /// that transforms a vertex in model space to its animated position in model space.
    ///
    /// This can be used in a vertex shader to apply linear blend skinning
    /// by transforming the vertex by up to 4 skinning matrices
    /// and blending with vertex skin weights.
    pub fn skinning_transforms(&self, skeleton: &VbnSkeleton, frame: f32) -> Vec<Mat4> {
        let anim_transforms = self.model_space_transforms(skeleton, frame);
        let bind_transforms = skeleton.model_space_transforms();

        let mut animated_transforms = vec![Mat4::IDENTITY; skeleton.bones.len()];
        for i in 0..skeleton.bones.len() {
            let inverse_bind = bind_transforms[i].inverse();
            animated_transforms[i] = anim_transforms[i] * inverse_bind;
        }

        animated_transforms
    }

    /// Calculate animation values relative to the bone's parent and "rest pose" or "bind pose".
    ///
    /// If `use_blender_coordinates` is `true`, the resulting values will match Blender's conventions.
    /// Bones will point along the y-axis instead of the x-axis and with z-axis for up instead of the y-axis.
    pub fn fcurves(&self, skeleton: &VbnSkeleton, use_blender_coordinates: bool) -> FCurves {
        let bind_transforms: Vec<_> = skeleton
            .model_space_transforms()
            .into_iter()
            .map(|t| {
                if use_blender_coordinates {
                    sm4sh_to_blender(t)
                } else {
                    t
                }
            })
            .collect();

        let animated_bone_hashes: BTreeSet<_> = self.nodes.iter().map(|n| n.hash).collect();

        let mut translation_points = BTreeMap::new();
        let mut rotation_points = BTreeMap::new();
        let mut scale_points = BTreeMap::new();

        for frame in 0..self.frame_count {
            let transforms = self.local_space_transforms(skeleton, frame as f32);

            let mut animated_transforms = bind_transforms.clone();

            for i in 0..animated_transforms.len() {
                let bone = &skeleton.bones[i];
                if animated_bone_hashes.contains(&bone.hash) {
                    let matrix = transforms[i];
                    if let Some(parent_index) = bone.parent_bone_index {
                        let transform = if use_blender_coordinates {
                            blender_transform(matrix)
                        } else {
                            matrix
                        };
                        animated_transforms[i] = animated_transforms[parent_index] * transform;
                    } else {
                        animated_transforms[i] = if use_blender_coordinates {
                            sm4sh_to_blender(matrix)
                        } else {
                            matrix
                        };
                    }

                    // Find the transform relative to the parent and "rest pose" or "bind pose".
                    // This matches the UI values used in Blender for posing bones.
                    // TODO: Add tests for calculating this.
                    let basis_transform = if let Some(parent_index) = bone.parent_bone_index {
                        let rest_local =
                            bind_transforms[parent_index].inverse() * bind_transforms[i];
                        let local =
                            animated_transforms[parent_index].inverse() * animated_transforms[i];
                        rest_local.inverse() * local
                    } else {
                        // Equivalent to above with parent transform set to identity.
                        bind_transforms[i].inverse() * animated_transforms[i]
                    };

                    let (s, r, t) = basis_transform.to_scale_rotation_translation();
                    insert_fcurve_point(&mut translation_points, bone.hash, t);
                    insert_fcurve_point(&mut rotation_points, bone.hash, r);
                    insert_fcurve_point(&mut scale_points, bone.hash, s);
                }
            }
        }

        FCurves {
            translation: translation_points,
            rotation: rotation_points,
            scale: scale_points,
        }
    }
}

impl AnimationNode {
    /// Sample the translation at `frame` using the appropriate interpolation between frames.
    /// Returns `None` if the animation is empty.
    pub fn sample_translation(&self, frame: f32) -> Option<Vec3> {
        sample_vec3(&self.translation_keyframes, frame)
    }

    /// Sample the rotation at `frame` using the appropriate interpolation between frames.
    /// Returns `None` if the animation is empty.
    pub fn sample_rotation(&self, frame: f32) -> Option<Quat> {
        sample_quat(&self.rotation_keyframes, frame)
    }

    /// Sample the scale at `frame` using the appropriate interpolation between frames.
    /// Returns `None` if the animation is empty.
    pub fn sample_scale(&self, frame: f32) -> Option<Vec3> {
        sample_vec3(&self.scale_keyframes, frame)
    }
}

fn sample_vec3(keyframes: &[Option<Vec3>], frame: f32) -> Option<Vec3> {
    let (index, x) = frame_index_pos(frame, keyframes.len());
    let current = keyframes.get(index).copied().flatten()?;
    if let Some(next) = keyframes.get(index + 1).copied().flatten() {
        Some(current.lerp(next, x))
    } else {
        Some(current)
    }
}

fn sample_quat(keyframes: &[Option<Quat>], frame: f32) -> Option<Quat> {
    let (index, x) = frame_index_pos(frame, keyframes.len());
    let current = keyframes.get(index).copied().flatten()?;
    if let Some(next) = keyframes.get(index + 1).copied().flatten() {
        Some(current.lerp(next, x))
    } else {
        Some(current)
    }
}

fn frame_index_pos(frame: f32, frame_count: usize) -> (usize, f32) {
    // Animations are baked, so each "keyframe" lasts for exactly 1 frame at 60 fps.
    // The final keyframe should persist for the rest of the animation.
    let index = (frame as usize).min(frame_count.saturating_sub(1));
    let x = frame.fract();
    (index, x)
}

#[derive(Debug)]
struct TransformData {
    translation_min: Option<Vec3>,
    translation_max: Option<Vec3>,

    rotation_min: Option<Vec3>,
    rotation_max: Option<Vec3>,

    scale_min: Option<Vec3>,
    scale_max: Option<Vec3>,
}

impl TransformData {
    fn translation(&self, keys: &[u16], key_index: &mut usize) -> Option<Vec3> {
        interpolate_vec3(self.translation_min, self.translation_max, keys, key_index)
    }

    fn rotation(&self, keys: &[u16], key_index: &mut usize) -> Option<Quat> {
        let xyz = interpolate_vec3(self.rotation_min, self.rotation_max, keys, key_index)?;
        let [x, y, z] = xyz.to_array();
        // Assume unit quaternion.
        let w = (1.0 - x * x - y * y - z * z).abs().sqrt();
        Some(Quat::from_xyzw(x, y, z, w))
    }

    fn scale(&self, keys: &[u16], key_index: &mut usize) -> Option<Vec3> {
        interpolate_vec3(self.scale_min, self.scale_max, keys, key_index)
    }
}

fn interpolate_vec3(
    min: Option<Vec3>,
    max: Option<Vec3>,
    keys: &[u16],
    key_index: &mut usize,
) -> Option<Vec3> {
    let min = min?;
    if let Some(max) = max {
        let f = vec3(
            keys[*key_index] as f32,
            keys[*key_index + 1] as f32,
            keys[*key_index + 2] as f32,
        ) / 65535.0;
        *key_index += 3;
        Some(min + f * max)
    } else {
        Some(min)
    }
}

fn omo_node_data(node: &OmoNode, inter_data: &[u8]) -> BinResult<TransformData> {
    let mut data = Cursor::new(&inter_data[node.inter_offset as usize..]);

    let mut translation_min = None;
    let mut translation_max = None;
    if node.flags.position() {
        match node.flags.position_type() {
            PositionType::Frame => {}
            PositionType::Interpolate => {
                let v: [f32; 3] = data.read_be()?;
                translation_min = Some(v.into());

                let v: [f32; 3] = data.read_be()?;
                translation_max = Some(v.into());
            }
            PositionType::Constant => {
                let v: [f32; 3] = data.read_be()?;
                translation_min = Some(v.into());
            }
        }
    }

    let mut rotation_min = None;
    let mut rotation_max = None;
    if node.flags.rotation() {
        match node.flags.rotation_type() {
            RotationType::Interpolate => {
                let v: [f32; 3] = data.read_be()?;
                rotation_min = Some(v.into());

                let v: [f32; 3] = data.read_be()?;
                rotation_max = Some(v.into());
            }
            RotationType::FConst => {
                // TODO: Is this actually a full quaternion?
                let v: [f32; 4] = data.read_be()?;
                rotation_min = Some([v[0], v[1], v[2]].into());
            }
            RotationType::Constant => {
                let v: [f32; 3] = data.read_be()?;
                rotation_min = Some(v.into());
            }
            RotationType::Frame => {
                // TODO: what does "frame" mean?
            }
        }
    }

    let mut scale_min = None;
    let mut scale_max = None;
    if node.flags.scale() {
        match node.flags.scale_type() {
            ScaleType::Constant | ScaleType::Constant2 => {
                let v: [f32; 3] = data.read_be()?;
                scale_min = Some(v.into());
            }
            ScaleType::Interpolate => {
                let v: [f32; 3] = data.read_be()?;
                scale_min = Some(v.into());

                let v: [f32; 3] = data.read_be()?;
                scale_max = Some(v.into());
            }
        }
    }

    Ok(TransformData {
        translation_min,
        translation_max,
        rotation_min,
        rotation_max,
        scale_min,
        scale_max,
    })
}

fn sm4sh_to_blender(m: Mat4) -> Mat4 {
    // Hard code these matrices for better precision.
    // rotate x -90 degrees
    let y_up_to_z_up = Mat4::from_cols_array_2d(&[
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    // rotate z -90 degrees.
    let x_major_to_y_major = Mat4::from_cols_array_2d(&[
        [0.0, -1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    y_up_to_z_up * m * x_major_to_y_major
}

fn blender_transform(m: Mat4) -> Mat4 {
    // In game, the bone's x-axis points from parent to child.
    // In Blender, the bone's y-axis points from parent to child.
    // https://en.wikipedia.org/wiki/Matrix_similarity
    // Perform the transformation m in Sm4sh's basis and convert back to Blender.
    let p = Mat4::from_cols_array_2d(&[
        [0.0, -1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ])
    .transpose();
    p * m * p.inverse()
}

fn insert_fcurve_point<T: Copy>(points: &mut BTreeMap<u32, Vec<T>>, hash: u32, t: T) {
    points
        .entry(hash)
        .and_modify(|f| {
            f.push(t);
        })
        .or_insert(vec![t]);
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::VbnBone;
    use glam::quat;
    use sm4sh_lib::vbn::BoneType;

    macro_rules! assert_matrix_relative_eq {
        ($a:expr, $b:expr) => {
            assert!(
                $a.to_cols_array()
                    .iter()
                    .zip($b.to_cols_array().iter())
                    .all(|(a, b)| approx::relative_eq!(a, b, epsilon = 0.0001f32)),
                "Matrices not equal to within 0.0001.\nleft = {:?}\nright = {:?}",
                $a,
                $b
            )
        };
    }

    #[test]
    fn model_space_transforms_empty() {
        let animation = Animation {
            frame_count: 1,
            nodes: Vec::new(),
        };

        assert!(animation
            .model_space_transforms(&VbnSkeleton { bones: Vec::new() }, 0.0)
            .is_empty());
    }

    #[test]
    fn model_space_transforms() {
        let animation = Animation {
            frame_count: 1,
            nodes: vec![
                AnimationNode {
                    translation_keyframes: vec![Some(vec3(1.0, 2.0, 3.0))],
                    rotation_keyframes: vec![Some(quat(0.0, 0.0, 0.0, 1.0))],
                    scale_keyframes: vec![Some(vec3(1.0, 1.0, 1.0))],
                    hash: 1,
                },
                AnimationNode {
                    translation_keyframes: vec![Some(vec3(1.0, 2.0, 3.0))],
                    rotation_keyframes: vec![Some(quat(0.0, 0.0, 0.0, 1.0))],
                    scale_keyframes: vec![Some(vec3(1.0, 1.0, 1.0))],
                    hash: 2,
                },
            ],
        };

        let skeleton = VbnSkeleton {
            bones: vec![
                VbnBone {
                    name: "a".to_string(),
                    hash: 1,
                    parent_bone_index: None,
                    bone_type: BoneType::Normal,
                    translation: Vec3::ZERO,
                    rotation: Vec3::ZERO,
                    scale: Vec3::ONE,
                },
                VbnBone {
                    name: "b".to_string(),
                    hash: 2,
                    parent_bone_index: Some(0),
                    bone_type: BoneType::Normal,
                    translation: Vec3::ZERO,
                    rotation: Vec3::ZERO,
                    scale: Vec3::ONE,
                },
            ],
        };

        let transforms = animation.model_space_transforms(&skeleton, 0.0);
        assert_eq!(2, transforms.len());
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [1.0, 2.0, 3.0, 1.0],
            ]),
            transforms[0]
        );
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [2.0, 4.0, 6.0, 1.0],
            ]),
            transforms[1]
        );
    }

    #[test]
    fn local_space_transforms() {
        let animation = Animation {
            frame_count: 1,
            nodes: vec![
                AnimationNode {
                    translation_keyframes: vec![Some(vec3(1.0, 2.0, 3.0))],
                    rotation_keyframes: vec![Some(quat(0.0, 0.0, 0.0, 1.0))],
                    scale_keyframes: vec![Some(vec3(1.0, 1.0, 1.0))],
                    hash: 1,
                },
                AnimationNode {
                    translation_keyframes: vec![Some(vec3(10.0, 20.0, 30.0))],
                    rotation_keyframes: vec![Some(quat(0.0, 0.0, 0.0, 1.0))],
                    scale_keyframes: vec![Some(vec3(1.0, 1.0, 1.0))],
                    hash: 2,
                },
            ],
        };

        let skeleton = VbnSkeleton {
            bones: vec![
                VbnBone {
                    name: "a".to_string(),
                    hash: 1,
                    parent_bone_index: None,
                    bone_type: BoneType::Normal,
                    translation: Vec3::ZERO,
                    rotation: Vec3::ZERO,
                    scale: Vec3::ONE,
                },
                VbnBone {
                    name: "b".to_string(),
                    hash: 2,
                    parent_bone_index: Some(0),
                    bone_type: BoneType::Normal,
                    translation: Vec3::ZERO,
                    rotation: Vec3::ZERO,
                    scale: Vec3::ONE,
                },
            ],
        };

        let transforms = animation.local_space_transforms(&skeleton, 0.0);
        assert_eq!(2, transforms.len());
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [1.0, 2.0, 3.0, 1.0],
            ]),
            transforms[0]
        );
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [10.0, 20.0, 30.0, 1.0],
            ]),
            transforms[1]
        );
    }

    #[test]
    fn fcurves_sm4sh() {
        let animation = Animation {
            frame_count: 1,
            nodes: vec![
                AnimationNode {
                    translation_keyframes: vec![Some(vec3(1.0, 2.0, 3.0))],
                    rotation_keyframes: vec![Some(quat(1.0, 0.0, 0.0, 0.0))],
                    scale_keyframes: vec![Some(vec3(1.0, 1.0, 1.0))],
                    hash: 1,
                },
                AnimationNode {
                    translation_keyframes: vec![Some(vec3(10.0, 20.0, 30.0))],
                    rotation_keyframes: vec![Some(quat(0.0, 1.0, 0.0, 0.0))],
                    scale_keyframes: vec![Some(vec3(1.0, 1.0, 1.0))],
                    hash: 2,
                },
            ],
        };

        let skeleton = VbnSkeleton {
            bones: vec![
                VbnBone {
                    name: "a".to_string(),
                    hash: 1,
                    parent_bone_index: None,
                    bone_type: BoneType::Normal,
                    translation: Vec3::ZERO,
                    rotation: Vec3::ZERO,
                    scale: Vec3::ONE,
                },
                VbnBone {
                    name: "b".to_string(),
                    hash: 2,
                    parent_bone_index: Some(0),
                    bone_type: BoneType::Normal,
                    translation: Vec3::ZERO,
                    rotation: Vec3::ZERO,
                    scale: Vec3::ONE,
                },
            ],
        };

        let fcurves = animation.fcurves(&skeleton, false);
        assert_eq!(
            FCurves {
                translation: [
                    (1, vec![vec3(1.0, 2.0, 3.0)]),
                    (2, vec![vec3(10.0, 20.0, 30.0)])
                ]
                .into(),
                rotation: [
                    (1, vec![quat(1.0, 0.0, 0.0, 0.0)]),
                    (2, vec![quat(0.0, 1.0, 0.0, 0.0)])
                ]
                .into(),
                scale: [
                    (1, vec![vec3(1.0, 1.0, 1.0)]),
                    (2, vec![vec3(1.0, 1.0, 1.0)])
                ]
                .into()
            },
            fcurves
        );
    }

    #[test]
    fn fcurves_blender() {
        let animation = Animation {
            frame_count: 1,
            nodes: vec![
                AnimationNode {
                    translation_keyframes: vec![Some(vec3(1.0, 2.0, 3.0))].into(),
                    rotation_keyframes: vec![Some(quat(1.0, 0.0, 0.0, 0.0))].into(),
                    scale_keyframes: vec![Some(vec3(1.0, 1.0, 1.0))].into(),
                    hash: 1,
                },
                AnimationNode {
                    translation_keyframes: vec![Some(vec3(10.0, 20.0, 30.0))].into(),
                    rotation_keyframes: vec![Some(quat(0.0, 1.0, 0.0, 0.0))].into(),
                    scale_keyframes: vec![Some(vec3(1.0, 1.0, 1.0))].into(),
                    hash: 2,
                },
            ],
        };

        let skeleton = VbnSkeleton {
            bones: vec![
                VbnBone {
                    name: "a".to_string(),
                    hash: 1,
                    parent_bone_index: None,
                    bone_type: BoneType::Normal,
                    translation: Vec3::ZERO,
                    rotation: Vec3::ZERO,
                    scale: Vec3::ONE,
                },
                VbnBone {
                    name: "b".to_string(),
                    hash: 2,
                    parent_bone_index: Some(0),
                    bone_type: BoneType::Normal,
                    translation: Vec3::ZERO,
                    rotation: Vec3::ZERO,
                    scale: Vec3::ONE,
                },
            ],
        };

        let fcurves = animation.fcurves(&skeleton, true);
        assert_eq!(
            FCurves {
                translation: [
                    (1, vec![vec3(-2.0, 1.0, 3.0)]),
                    (2, vec![vec3(-20.0, 10.0, 30.0)])
                ]
                .into(),
                rotation: [
                    (1, vec![quat(0.0, 1.0, 0.0, 0.0)]),
                    (2, vec![quat(1.0, 0.0, 0.0, 0.0)])
                ]
                .into(),
                scale: [
                    (1, vec![vec3(1.0, 1.0, 1.0)]),
                    (2, vec![vec3(1.0, 1.0, 1.0)])
                ]
                .into()
            },
            fcurves
        );
    }
}
