use case::CaseExt;
use glam::{Mat4, UVec4, Vec4, vec4};
use xc3_shader::expr::{OutputExpr, Parameter, Value};

use crate::database::ShaderProgram;

pub fn uniform_parameter_value(program: &ShaderProgram, p: &Parameter) -> Option<f32> {
    // TODO: properly set the index.
    let i = match p.index.map(|i| &program.exprs[i]) {
        Some(OutputExpr::Value(Value::Int(i))) => *i as usize,
        _ => 0,
    };
    match p.name.as_str() {
        "FB0" => Some(fb0(1920.0, 1080.0).get_field(&p.field, i, p.channel)),
        "FB1" => Some(fb1().get_field(&p.field, i, p.channel)),
        "FB3" => Some(fb3().get_field(&p.field, p.channel)),
        "FB4" => Some(fb4().get_field(&p.field, p.channel)),
        "FB5" => Some(fb5().get_field(&p.field, p.channel)),
        _ => None,
    }
}

// TODO: Avoid duplicating these types with sm4sh_wgpu.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fb0 {
    pub depth_of_field0: glam::Vec4,
    pub depth_of_field1: glam::Vec4,
    pub depth_of_field_tex_size: glam::Vec4,
    pub proj_inv_matrix: glam::Mat4,
    pub refraction_param: glam::Vec4,
    pub proj_to_view: glam::Vec4,
    pub view_to_proj: glam::Vec4,
    pub gi_buffer_size: glam::Vec4,
    pub weight0: glam::Vec4,
    pub weight1: glam::Vec4,
    pub random_vector: [glam::Vec4; 31],
    pub reflection_param: glam::Vec4,
    pub sun_shaft_light_param0: [glam::Vec4; 2],
    pub sun_shaft_light_param1: [glam::Vec4; 2],
    pub sun_shaft_blur_param: [glam::Vec4; 4],
    pub sun_shaft_composite_param: [glam::Vec4; 2],
    pub glare_abstract_param: glam::Vec4,
    pub render_target_tex_size: glam::Vec4,
    pub glare_fog_param: [glam::Vec4; 2],
    pub glare_simple_color: glam::Vec4,
    pub pad0_fb0: glam::Vec4,
    pub lens_flare_param: glam::Vec4,
    pub outline_param: glam::Vec4,
    pub post_reflection_color: glam::Vec4,
    pub multi_shadow_matrix: [glam::Mat4; 4],
    pub shadow_map_matrix: glam::Mat4,
    pub view: glam::Mat4,
    pub eye: glam::Vec4,
    pub constant_color: glam::Vec4,
    pub light_map_pos: glam::Vec4,
    pub reflection_gain: glam::Vec4,
    pub hdr_constant: glam::Vec4,
    pub _g_fresnel_color: glam::Vec4,
    pub effect_light_param0: glam::Vec4,
    pub effect_light_param1: glam::Vec4,
    pub bg_rot_inv: glam::Mat4,
    pub reflection_color1: glam::Vec4,
    pub reflection_color2: glam::Vec4,
    pub reflection_color3: glam::Vec4,
    pub effect_light_param2: glam::Vec4,
}
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fb1 {
    pub light_map_matrix: glam::Mat4,
    pub blink_color: glam::Vec4,
    pub g_constant_volume: glam::Vec4,
    pub g_constant_offset: glam::Vec4,
    pub uv_scroll_counter: glam::Vec4,
    pub spycloak_params: glam::Vec4,
    pub compress_param: glam::Vec4,
    pub g_fresnel_color: glam::Vec4,
    pub depth_offset: glam::Vec4,
    pub outline_color: glam::Vec4,
    pub pad0_fb1: [glam::Vec4; 3],
    pub light_map_color_gain: glam::Vec4,
    pub light_map_color_offset: glam::Vec4,
    pub ceiling_dir: glam::Vec4,
    pub ceiling_color: glam::Vec4,
    pub ground_color: glam::Vec4,
    pub ambient_color: glam::Vec4,
    pub light_dir_color1: glam::Vec4,
    pub light_dir_color2: glam::Vec4,
    pub light_dir_color3: glam::Vec4,
    pub light_dir1: glam::Vec4,
    pub light_dir2: glam::Vec4,
    pub light_dir3: glam::Vec4,
    pub fog_color: glam::Vec4,
    pub g_fresnel_offset: glam::Vec4,
    pub shadow_map_param: glam::Vec4,
    pub char_shadow_color: glam::Vec4,
    pub char_shadow_color2: glam::Vec4,
    pub soft_lighting_params2: glam::Vec4,
    pub bg_shadow_color: glam::Vec4,
    pub g_ibl_color_gain: glam::Vec4,
    pub g_ibl_color_offset: glam::Vec4,
    pub g_constant_min: glam::Vec4,
    pub loupe_shadow_params: glam::Vec4,
    pub soft_light_color_gain: glam::Vec4,
    pub soft_light_color_offset: glam::Vec4,
    pub character_color: glam::Vec4,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fb3 {
    pub hdr_range: glam::Vec4,
    pub colr_hdr_range: glam::Vec4,
}
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fb4 {
    pub effect_light_entry: glam::Vec4,
}
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fb5 {
    pub effect_light_area: glam::UVec4,
}

// TODO: find a way to avoid duplicating this logic with sm4sh_wgpu
fn fb0(width: f32, height: f32) -> Fb0 {
    Fb0 {
        depth_of_field0: vec4(0.0, 0.0, 0.0, 0.0),
        depth_of_field1: vec4(0.0, 0.0, 0.0, 0.0),
        depth_of_field_tex_size: vec4(0.0, 0.0, 0.0, 0.0),
        proj_inv_matrix: Mat4::IDENTITY, // TODO: Fill in this value
        refraction_param: vec4(0.0, 0.0, 0.0, 0.0),
        proj_to_view: vec4(0.47635, 0.26795, 256.00, 0.00),
        view_to_proj: vec4(1.04964, -1.86603, 0.00391, 0.00),
        gi_buffer_size: vec4(width / 4.0, height / 4.0, 4.0 / width, 4.0 / height),
        weight0: vec4(0.14374, 0.1258, 0.09635, 0.06459),
        weight1: vec4(0.03789, 0.01945, 0.00874, 0.00344),
        random_vector: [Vec4::ZERO; 31], // TODO: Fill in these values
        reflection_param: vec4(0.0075, 2.50, 0.25, 0.00),
        sun_shaft_light_param0: [vec4(0.0, 0.0, 0.0, 0.0), vec4(0.0, 0.0, 0.0, 0.0)],
        sun_shaft_light_param1: [vec4(0.0, 0.0, 0.0, 0.0), vec4(0.0, 0.0, 0.0, 0.0)],
        sun_shaft_blur_param: [
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
        ],
        sun_shaft_composite_param: [vec4(0.0, 0.0, 0.0, 0.0), vec4(0.0, 0.0, 0.0, 0.0)],
        glare_abstract_param: vec4(1.0, 4.0, 0.0, 0.0),
        render_target_tex_size: vec4(1.0 / width, 1.0 / height, 2.0 / width, 2.0 / height),
        glare_fog_param: [vec4(0.0, 0.0, 0.0, 0.0), vec4(0.0, 0.0, 0.0, 0.0)],
        glare_simple_color: vec4(0.0, 0.0, 0.0, 0.0),
        pad0_fb0: vec4(0.0, 0.0, 0.0, 0.0),
        lens_flare_param: vec4(0.0, 0.0, 0.0, 0.0),
        outline_param: vec4(0.25, 0.00, 0.00, 0.00),
        post_reflection_color: vec4(0.50, 0.50, 0.50, 0.20),
        multi_shadow_matrix: [Mat4::IDENTITY; 4], // TODO: fill in these values
        shadow_map_matrix: Mat4::from_cols_array_2d(&[
            [0.00814, 0.00, 0.00, 0.00],
            [0.00, -0.00504, -0.01631, 0.00],
            [0.00, 0.01385, -0.00594, 0.00],
            [0.49189, 0.67917, 1.09728, 1.00],
        ]), // TODO: fill in these values
        view: Mat4::ZERO,                         // TODO: fill in these values
        eye: vec4(40.0, 47.40689, 37.02085, 1.0), // TODO: fill in these values
        constant_color: vec4(1.0, 1.0, 1.0, 1.0),
        light_map_pos: vec4(0.0, 0.0, 0.0, 0.0),
        reflection_gain: vec4(1.0, 1.0, 1.0, 1.0),
        hdr_constant: vec4(0.5, 2.0, 1.0, 1.0),
        _g_fresnel_color: vec4(1.0, 1.0, 1.0, 1.0),
        effect_light_param0: vec4(0.1, 0.1, -15.0, 0.0),
        effect_light_param1: vec4(30.0, 12.0, 29.0, 11.0),
        bg_rot_inv: Mat4::IDENTITY,
        reflection_color1: vec4(0.0, 0.0, 0.0, 0.0),
        reflection_color2: vec4(0.0001, 0.0, 0.0, 0.0),
        reflection_color3: vec4(0.315, 0.31792, 0.35, 1.0),
        effect_light_param2: vec4(0.685, 0.68208, 0.65, 1.00),
    }
}

fn fb1() -> Fb1 {
    Fb1 {
        light_map_matrix: Mat4::IDENTITY,
        blink_color: vec4(1.0, 1.0, 1.0, 0.0),
        g_constant_volume: vec4(1.0, 1.0, 1.0, 1.0),
        g_constant_offset: vec4(0.0, 0.0, 0.0, 0.0),
        uv_scroll_counter: vec4(0.35, 0.0, 0.0, 0.0), // TODO: changes over time?
        spycloak_params: vec4(-100.0, 0.0, 0.0, 0.0),
        compress_param: vec4(1.0, 0.0, 0.0, 0.0),
        g_fresnel_color: vec4(1.0, 1.0, 1.0, 1.0),
        depth_offset: vec4(0.0, 0.0, 0.0, 0.0),
        outline_color: vec4(0.0, 0.0, 0.0, 1.0),
        pad0_fb1: [
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
        ],
        light_map_color_gain: vec4(0.4875, 0.4875, 0.4875, 0.0),
        light_map_color_offset: vec4(0.0, 0.0, 0.0, 0.0),
        ceiling_dir: vec4(0.0, 1.0, 0.0, 0.0),
        ceiling_color: vec4(0.15, 0.15, 0.15, 0.0),
        ground_color: vec4(1.0, 1.0, 1.0, 0.0),
        ambient_color: vec4(0.0, 0.0, 0.0, 0.0),
        light_dir_color1: vec4(0.75, 0.75, 0.75, 0.0),
        light_dir_color2: vec4(0.2, 0.2, 0.2, 1.0),
        light_dir_color3: vec4(0.0, 0.0, 0.0, 0.0),
        light_dir1: vec4(0.0, -0.84323, -0.53756, 0.0),
        light_dir2: vec4(-0.87287, 0.43644, -0.21822, 0.0),
        light_dir3: vec4(0.0, 0.0, 0.0, 0.0),
        fog_color: vec4(1.0, 1.0, 1.0, 1.0),
        g_fresnel_offset: vec4(0.0, 0.0, 0.0, 0.0),
        shadow_map_param: vec4(0.001, 0.0, 0.0, 0.0),
        char_shadow_color: vec4(0.315, 0.31792, 0.35, 1.0),
        char_shadow_color2: vec4(0.685, 0.68208, 0.65, 1.0),
        soft_lighting_params2: vec4(0.0, 0.0, 0.0, 1.0),
        bg_shadow_color: vec4(0.81, 0.8175, 0.90, 1.0),
        g_ibl_color_gain: vec4(1.0, 1.0, 1.0, 0.0),
        g_ibl_color_offset: vec4(0.15, 0.15, 0.15, 0.0),
        g_constant_min: Vec4::ZERO,
        loupe_shadow_params: Vec4::ZERO,
        soft_light_color_gain: Vec4::ZERO,
        soft_light_color_offset: Vec4::ZERO,
        character_color: Vec4::ZERO,
    }
}

fn fb3() -> Fb3 {
    Fb3 {
        hdr_range: vec4(0.5, 2.0, 0.0, 0.0),
        colr_hdr_range: Vec4::ZERO,
    }
}

fn fb4() -> Fb4 {
    Fb4 {
        effect_light_entry: Vec4::ZERO,
    }
}

fn fb5() -> Fb5 {
    Fb5 {
        effect_light_area: UVec4::ZERO,
    }
}

impl Fb0 {
    pub(crate) fn get_field(&self, field: &str, index: usize, channel: Option<char>) -> f32 {
        let c = match channel {
            Some('x') => 0,
            Some('y') => 1,
            Some('z') => 2,
            Some('w') => 3,
            _ => 0,
        };
        // TODO: field name, index, channel
        // TODO: move this to the shaderprogram?
        // TODO: properly handle matrix arrays.
        match field.to_snake().as_str() {
            "depth_of_field0" => self.depth_of_field0[c],
            "depth_of_field1" => self.depth_of_field1[c],
            "depth_of_field_tex_s" => self.depth_of_field_tex_size[c],
            "proj_inv_matrix" => self.proj_inv_matrix.col(index)[c],
            "refraction_param" => self.refraction_param[c],
            "proj_to_view" => self.proj_to_view[c],
            "view_to_proj" => self.view_to_proj[c],
            "gi_buffer_size" => self.gi_buffer_size[c],
            "weight0" => self.weight0[c],
            "weight1" => self.weight1[c],
            "random_vector" => self.random_vector[index][c],
            "reflection_param" => self.reflection_param[c],
            "sun_shaft_light_param" => self.sun_shaft_light_param0[index][c],
            "sun_shaft_blur_param" => self.sun_shaft_blur_param[index][c],
            "sun_shaft_composite_param" => self.sun_shaft_composite_param[index][c],
            "glare_abstract_param" => self.glare_abstract_param[c],
            "render_target_tex_size" => self.render_target_tex_size[c],
            "glare_fog_param" => self.glare_fog_param[index][c],
            "glare_simple_color" => self.glare_simple_color[c],
            "pad0_fb0" => self.pad0_fb0[c],
            "lens_flare_param" => self.lens_flare_param[c],
            "outline_param" => self.outline_param[c],
            "post_reflection_color" => self.post_reflection_color[c],
            "multi_shadow_matrix" => self.multi_shadow_matrix[index].col(0)[c],
            "shadow_map_matrix" => self.shadow_map_matrix.col(index)[c],
            "view" => self.view.col(index)[c],
            "eye" => self.eye[c],
            "constant_color" => self.constant_color[c],
            "light_map_pos" => self.light_map_pos[c],
            "reflection_gain" => self.reflection_gain[c],
            "hdr_constant" => self.hdr_constant[c],
            "_g_fresnel_color" => self._g_fresnel_color[c],
            "effect_light_param0" => self.effect_light_param0[c],
            "effect_light_param1" => self.effect_light_param1[c],
            "bg_rot_inv" => self.bg_rot_inv.col(index)[c],
            "reflection_color1" => self.reflection_color1[c],
            "reflection_color2" => self.reflection_color2[c],
            "reflection_color3" => self.reflection_color3[c],
            "effect_light_param2" => self.effect_light_param2[c],
            _ => todo!(),
        }
    }
}

impl Fb1 {
    pub(crate) fn get_field(&self, field: &str, index: usize, channel: Option<char>) -> f32 {
        let c = match channel {
            Some('x') => 0,
            Some('y') => 1,
            Some('z') => 2,
            Some('w') => 3,
            _ => 0,
        };
        // TODO: field name, index, channel
        // TODO: move this to the shaderprogram?
        // TODO: properly handle matrix arrays.
        match field.to_snake().as_str() {
            "light_map_matrix" => self.light_map_matrix.col(index)[c],
            "blink_color" => self.blink_color[c],
            "g_constant_volume" => self.g_constant_volume[c],
            "g_constant_offset" => self.g_constant_offset[c],
            "uv_scroll_counter" => self.uv_scroll_counter[c],
            "spycloak_params" => self.spycloak_params[c],
            "compress_param" => self.compress_param[c],
            "g_fresnel_color" => self.g_fresnel_color[c],
            "depth_offset" => self.depth_offset[c],
            "outline_color" => self.outline_color[c],
            "pad0_fb1" => self.pad0_fb1[index][c],
            "light_map_color_gain" => self.light_map_color_gain[c],
            "light_map_color_offset" => self.light_map_color_offset[c],
            "ceiling_dir" => self.ceiling_dir[c],
            "ceiling_color" => self.ceiling_color[c],
            "ground_color" => self.ground_color[c],
            "ambient_color" => self.ambient_color[c],
            "light_dir_color1" => self.light_dir_color1[c],
            "light_dir_color2" => self.light_dir_color2[c],
            "light_dir_color3" => self.light_dir_color3[c],
            "light_dir1" => self.light_dir1[c],
            "light_dir2" => self.light_dir2[c],
            "light_dir3" => self.light_dir3[c],
            "fog_color" => self.fog_color[c],
            "g_fresnel_offset" => self.g_fresnel_offset[c],
            "shadow_map_param" => self.shadow_map_param[c],
            "char_shadow_color" => self.char_shadow_color[c],
            "char_shadow_color2" => self.char_shadow_color2[c],
            "soft_lighting_params2" => self.soft_lighting_params2[c],
            "bg_shadow_color" => self.bg_shadow_color[c],
            "g_ibl_color_gain" => self.g_ibl_color_gain[c],
            "g_ibl_color_offset" => self.g_ibl_color_offset[c],
            "g_constant_min" => self.g_constant_min[c],
            "loupe_shadow_params" => self.loupe_shadow_params[c],
            "soft_light_color_gain" => self.soft_light_color_gain[c],
            "soft_light_color_offset" => self.soft_light_color_offset[c],
            "character_color" => self.character_color[c],
            _ => todo!(),
        }
    }
}

impl Fb3 {
    pub(crate) fn get_field(&self, field: &str, channel: Option<char>) -> f32 {
        let c = match channel {
            Some('x') => 0,
            Some('y') => 1,
            Some('z') => 2,
            Some('w') => 3,
            _ => 0,
        };
        match field.to_snake().as_str() {
            "hdr_range" => self.hdr_range[c],
            "colr_hdr_range" => self.colr_hdr_range[c],
            _ => todo!(),
        }
    }
}

impl Fb4 {
    pub(crate) fn get_field(&self, field: &str, channel: Option<char>) -> f32 {
        let c = match channel {
            Some('x') => 0,
            Some('y') => 1,
            Some('z') => 2,
            Some('w') => 3,
            _ => 0,
        };
        match field.to_snake().as_str() {
            "effect_light_entry" => self.effect_light_entry[c],
            _ => todo!(),
        }
    }
}

impl Fb5 {
    pub(crate) fn get_field(&self, field: &str, channel: Option<char>) -> f32 {
        let c = match channel {
            Some('x') => 0,
            Some('y') => 1,
            Some('z') => 2,
            Some('w') => 3,
            _ => 0,
        };
        match field.to_snake().as_str() {
            "effect_light_area" => self.effect_light_area[c] as f32,
            _ => todo!(),
        }
    }
}
