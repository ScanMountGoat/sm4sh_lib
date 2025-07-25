use std::io::Cursor;

use binrw::BinReaderExt;
use glam::{EulerRot, Mat4, Quat, Vec3, Vec4, Vec4Swizzles};
use sm4sh_lib::omo::{Omo, OmoNode, PositionType, RotationType, ScaleType};

use crate::nud::VbnSkeleton;

#[derive(Debug, PartialEq, Clone)]
pub struct Animation {
    pub nodes: Vec<AnimationNode>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AnimationNode {
    pub hash: u32,
    pub translation: Option<Vec3>,
    pub rotation: Option<Quat>, // TODO: enum for Vec3 euler or Quat?
    pub scale: Option<Vec3>,
}

impl Animation {
    pub fn from_omo(omo: &Omo) -> Self {
        let mut nodes = Vec::new();

        for node in &omo.nodes {
            let data = omo_node_data(node, &omo.inter_data);

            // TODO: apply "frames" and interpolate

            let key = if let Some([x, y]) = omo.keys.get(0..2) {
                u16::from_be_bytes([*x, *y])
            } else {
                0
            };

            let animation_node = AnimationNode {
                hash: node.hash,
                translation: data.translation(key),
                rotation: data.rotation(key),
                scale: data.scale(key),
            };
            nodes.push(animation_node);
        }

        Self { nodes }
    }

    pub fn model_space_transforms(&self, skeleton: &VbnSkeleton) -> Vec<Mat4> {
        let mut final_transforms: Vec<_> = skeleton
            .bones
            .iter()
            .map(|b| {
                if let Some(node) = self.nodes.iter().find(|n| n.hash == b.hash) {
                    Mat4::from_translation(node.translation.unwrap_or(b.translation))
                        * node
                            .rotation
                            .map(|r| Mat4::from_quat(r))
                            .unwrap_or_else(|| {
                                Mat4::from_euler(
                                    EulerRot::XYZEx,
                                    b.rotation.x,
                                    b.rotation.y,
                                    b.rotation.z,
                                )
                            })
                        * Mat4::from_scale(node.scale.unwrap_or(b.scale))
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

    pub fn skinning_transforms(&self, skeleton: &VbnSkeleton) -> Vec<Mat4> {
        let anim_transforms = self.model_space_transforms(skeleton);
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
struct Transform {
    translation_min: Option<Vec3>,
    translation_max: Option<Vec3>,

    rotation_min: Option<Quat>,
    rotation_max: Option<Quat>,

    scale_min: Option<Vec3>,
    scale_max: Option<Vec3>,
}

impl Transform {
    fn translation(&self, key: u16) -> Option<Vec3> {
        interpolate(self.translation_min, self.translation_max, key)
    }

    fn rotation(&self, key: u16) -> Option<Quat> {
        interpolate_quat(self.rotation_min, self.rotation_max, key)
    }

    fn scale(&self, key: u16) -> Option<Vec3> {
        interpolate(self.scale_min, self.scale_max, key)
    }
}

fn interpolate(min: Option<Vec3>, max: Option<Vec3>, key: u16) -> Option<Vec3> {
    let f = (key as f32) / 65535.0;
    let min = min?;
    if let Some(max) = max {
        Some(min + f * max)
    } else {
        Some(min)
    }
}

fn interpolate_quat(min: Option<Quat>, max: Option<Quat>, key: u16) -> Option<Quat> {
    let f = (key as f32) / 65535.0;
    let min = min?;
    if let Some(max) = max {
        let xyz = min.xyz() + f * max.xyz();
        Some(quat_from_xyz(xyz))
    } else {
        Some(Quat::from_array(min.to_array()))
    }
}

fn omo_node_data(node: &OmoNode, inter_data: &[u8]) -> Transform {
    let mut data = Cursor::new(inter_data);

    let mut translation_min = None;
    let mut translation_max = None;
    if node.flags.position() {
        match node.flags.position_type() {
            PositionType::Unk4 => {}
            PositionType::Inter => {
                let v: [f32; 3] = data.read_be().unwrap();
                translation_min = Some(v.into());

                let v: [f32; 3] = data.read_be().unwrap();
                translation_max = Some(v.into());
            }
            PositionType::Const => {
                let v: [f32; 3] = data.read_be().unwrap();
                translation_min = Some(v.into());
            }
        }
    }

    let mut rotation_min = None;
    let mut rotation_max = None;
    if node.flags.rotation() {
        match node.flags.rotation_type() {
            RotationType::Inter => {
                // TODO: Store euler angles?
                let v: [f32; 3] = data.read_be().unwrap();
                rotation_min = Some(Quat::from_euler(EulerRot::XYZEx, v[0], v[1], v[2]));

                let v: [f32; 3] = data.read_be().unwrap();
                rotation_max = Some(Quat::from_euler(EulerRot::XYZEx, v[0], v[1], v[2]));
            }
            RotationType::FConst => {
                // TODO: Is this actually a quaternion?
                let v: [f32; 4] = data.read_be().unwrap();
                rotation_min = Some(Quat::from_array(v));
            }
            RotationType::Const => {
                let v: [f32; 3] = data.read_be().unwrap();
                rotation_min = Some(Quat::from_euler(EulerRot::XYZEx, v[0], v[1], v[2]));
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
            ScaleType::Const | ScaleType::Const2 => {
                let v: [f32; 3] = data.read_be().unwrap();
                scale_min = Some(v.into());
            }
            ScaleType::Inter => {
                let v: [f32; 3] = data.read_be().unwrap();
                scale_min = Some(v.into());

                let v: [f32; 3] = data.read_be().unwrap();
                scale_max = Some(v.into());
            }
        }
    }

    Transform {
        translation_min,
        translation_max,
        rotation_min,
        rotation_max,
        scale_min,
        scale_max,
    }
}

fn quat_from_xyz(xyz: Vec3) -> Quat {
    // Assume normalized quaternions to infer the missing component.
    // TODO: Are these vectors always normalized?
    let [x, y, z] = xyz.to_array();
    let w = (1.0 - x * x - y * y - z * z).abs().sqrt();
    Quat::from_xyzw(x, y, z, w)
}
