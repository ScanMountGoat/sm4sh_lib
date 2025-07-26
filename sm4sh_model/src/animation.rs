use std::io::Cursor;

use binrw::BinReaderExt;
use glam::{vec3, EulerRot, Mat4, Quat, Vec3};
use sm4sh_lib::omo::{Omo, OmoNode, PositionType, RotationType, ScaleType};

use crate::nud::VbnSkeleton;

#[derive(Debug, PartialEq, Clone)]
pub struct Animation {
    pub nodes: Vec<AnimationNode>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AnimationNode {
    pub hash: u32,
    pub translations: Vec<Option<Vec3>>,
    pub rotations: Vec<Option<Quat>>,
    pub scales: Vec<Option<Vec3>>,
}

impl Animation {
    pub fn from_omo(omo: &Omo) -> Self {
        let mut nodes = Vec::new();
        for node in &omo.nodes {
            let data = omo_node_data(node, &omo.inter_data);

            // TODO: Find a nicer way to select key data for each frame.
            let mut translations = Vec::new();
            let mut rotations = Vec::new();
            let mut scales = Vec::new();

            for frame in &omo.frames {
                // Convert a byte offset to an index for u16s.
                let mut key_index = node.key_offset as usize / 2;
                translations.push(data.translation(&frame.keys, &mut key_index));
                rotations.push(data.rotation(&frame.keys, &mut key_index));
                scales.push(data.scale(&frame.keys, &mut key_index));
            }

            let animation_node = AnimationNode {
                hash: node.hash,
                translations,
                rotations,
                scales,
            };
            nodes.push(animation_node);
        }

        Self { nodes }
    }

    pub fn model_space_transforms(
        &self,
        skeleton: &VbnSkeleton,
        current_time_seconds: f32,
    ) -> Vec<Mat4> {
        // TODO: interpolation and looping.
        let frame_index = (current_time_seconds * 60.0) as usize;

        let mut final_transforms: Vec<_> = skeleton
            .bones
            .iter()
            .map(|b| {
                if let Some(node) = self.nodes.iter().find(|n| n.hash == b.hash) {
                    let translation = node
                        .translations
                        .get(frame_index)
                        .copied()
                        .flatten()
                        .unwrap_or(b.translation);

                    let rotation = node
                        .rotations
                        .get(frame_index)
                        .copied()
                        .flatten()
                        .map(Mat4::from_quat)
                        .unwrap_or_else(|| {
                            Mat4::from_euler(
                                EulerRot::XYZEx,
                                b.rotation.x,
                                b.rotation.y,
                                b.rotation.z,
                            )
                        });

                    let scale = node
                        .scales
                        .get(frame_index)
                        .copied()
                        .flatten()
                        .unwrap_or(b.scale);

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

    pub fn skinning_transforms(
        &self,
        skeleton: &VbnSkeleton,
        current_time_seconds: f32,
    ) -> Vec<Mat4> {
        let anim_transforms = self.model_space_transforms(skeleton, current_time_seconds);
        let bind_transforms = skeleton.model_space_transforms();

        let mut animated_transforms = vec![Mat4::IDENTITY; skeleton.bones.len()];
        for i in 0..skeleton.bones.len() {
            let inverse_bind = bind_transforms[i].inverse();
            animated_transforms[i] = anim_transforms[i] * inverse_bind;
        }

        animated_transforms
    }
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

fn omo_node_data(node: &OmoNode, inter_data: &[u8]) -> TransformData {
    let mut data = Cursor::new(&inter_data[node.inter_offset as usize..]);

    let mut translation_min = None;
    let mut translation_max = None;
    if node.flags.position() {
        match node.flags.position_type() {
            PositionType::Frame => {}
            PositionType::Interpolate => {
                let v: [f32; 3] = data.read_be().unwrap();
                translation_min = Some(v.into());

                let v: [f32; 3] = data.read_be().unwrap();
                translation_max = Some(v.into());
            }
            PositionType::Constant => {
                let v: [f32; 3] = data.read_be().unwrap();
                translation_min = Some(v.into());
            }
        }
    }

    let mut rotation_min = None;
    let mut rotation_max = None;
    if node.flags.rotation() {
        match node.flags.rotation_type() {
            RotationType::Interpolate => {
                let v: [f32; 3] = data.read_be().unwrap();
                rotation_min = Some(v.into());

                let v: [f32; 3] = data.read_be().unwrap();
                rotation_max = Some(v.into());
            }
            RotationType::FConst => {
                // TODO: Is this actually a full quaternion?
                let v: [f32; 4] = data.read_be().unwrap();
                rotation_min = Some([v[0], v[1], v[2]].into());
            }
            RotationType::Constant => {
                let v: [f32; 3] = data.read_be().unwrap();
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
                let v: [f32; 3] = data.read_be().unwrap();
                scale_min = Some(v.into());
            }
            ScaleType::Interpolate => {
                let v: [f32; 3] = data.read_be().unwrap();
                scale_min = Some(v.into());

                let v: [f32; 3] = data.read_be().unwrap();
                scale_max = Some(v.into());
            }
        }
    }

    TransformData {
        translation_min,
        translation_max,
        rotation_min,
        rotation_max,
        scale_min,
        scale_max,
    }
}
