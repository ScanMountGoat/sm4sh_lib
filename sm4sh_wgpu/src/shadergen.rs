use std::{fmt::Write, sync::LazyLock};

use aho_corasick::AhoCorasick;
use case::CaseExt;
use indoc::formatdoc;
use log::error;
use sm4sh_model::{
    AlphaFunc,
    database::{Operation, OutputExpr, Parameter, ShaderProgram, Value},
};

const OUT_VAR: &str = "out_color";
const VAR_PREFIX: &str = "VAR";

static WGSL_REPLACEMENTS: LazyLock<AhoCorasick> = LazyLock::new(|| {
    AhoCorasick::new([
        "let ASSIGN_VARS_GENERATED = 0.0;",
        "let ASSIGN_OUT_COLOR_GENERATED = 0.0;",
        "let ALPHA_TEST_GENERATED = 0.0;",
    ])
    .unwrap()
});

/// Generated WGSL model shader code for a material.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct ShaderWgsl {
    assignments: String,
    outputs: String,
    discard: String,
}

impl ShaderWgsl {
    pub fn new(
        program: Option<&ShaderProgram>,
        alpha_test_ref_func: Option<(u16, AlphaFunc)>,
    ) -> Self {
        let (assignments, outputs) = program
            .map(|p| (generate_assignments_wgsl(p), generate_outputs_wgsl(p)))
            .unwrap_or_default();

        let discard = alpha_test_ref_func
            .map(|(ref_value, func)| alpha_test(ref_value, func))
            .unwrap_or_default();

        Self {
            assignments,
            outputs,
            discard,
        }
    }

    pub fn create_model_shader(&self) -> String {
        let replace_with = &[&self.assignments, &self.outputs, &self.discard];

        let mut source = WGSL_REPLACEMENTS.replace_all(crate::shader::model::SOURCE, replace_with);

        // This section is only used for wgsl_to_wgpu reachability analysis and can be removed.
        if let (Some(start), Some(end)) = (
            source.find("let REMOVE_BEGIN = 0.0;"),
            source.find("let REMOVE_END = 0.0;"),
        ) {
            source.replace_range(start..end, "");
        }

        source
    }
}

fn alpha_test(ref_value: u16, func: AlphaFunc) -> String {
    // The function determines what alpha values pass the alpha test.
    let ref_value = ref_value as f32 / 255.0;
    match func {
        AlphaFunc::Disabled => String::new(),
        AlphaFunc::Never => "discard;".to_string(),
        AlphaFunc::Less => alpha_test_inner(ref_value, "<"),
        AlphaFunc::Equal => alpha_test_inner(ref_value, "=="),
        AlphaFunc::Greater => alpha_test_inner(ref_value, ">"),
        AlphaFunc::NotEqual => alpha_test_inner(ref_value, "!="),
        AlphaFunc::GreaterEqual => alpha_test_inner(ref_value, ">="),
        AlphaFunc::Always => String::new(),
    }
}

fn alpha_test_inner(ref_value: f32, func: &str) -> String {
    formatdoc! {"
        if !(out_color.a {func} {ref_value:?}) {{
            discard;
        }}
    "}
}

fn generate_assignments_wgsl(program: &ShaderProgram) -> String {
    let mut wgsl = String::new();
    for (i, expr) in program.exprs.iter().enumerate() {
        write!(&mut wgsl, "let {VAR_PREFIX}{i} = ",).unwrap();
        if write_expr(&mut wgsl, expr).is_none() {
            write!(wgsl, "0.0").unwrap();
        }
        writeln!(&mut wgsl, ";",).unwrap();
    }
    wgsl
}

fn write_expr(wgsl: &mut String, expr: &OutputExpr<Operation>) -> Option<()> {
    match expr {
        OutputExpr::Value(value) => write_value(wgsl, value),
        OutputExpr::Func { op, args } => write_func(wgsl, op, args),
    }
}

fn write_value(wgsl: &mut String, value: &Value) -> Option<()> {
    match value {
        Value::Int(i) => {
            write!(wgsl, "{i:?}").unwrap();
            Some(())
        }
        Value::Float(f) => {
            write!(wgsl, "{f:?}").unwrap();
            Some(())
        }
        Value::Parameter(p) => write_parameter(wgsl, p),
        Value::Texture(t) => write_texture(wgsl, t),
        Value::Attribute(a) => write_attribute(wgsl, a),
    }
}

fn write_texture(wgsl: &mut String, t: &sm4sh_model::database::Texture) -> Option<()> {
    let a = VAR_PREFIX;
    match t.name.as_str() {
        "g_PCFTextureSampler"
        | "sampler0"
        | "Sampler11"
        | "samplerA"
        | "samplerB"
        | "multiplicationSampler"
        | "frameSampler" => None,
        "colorSampler2" | "colorSampler3" => None, // TODO: load all color textures
        "reflectionSampler" => write_sampler_2d_or_cube(
            wgsl,
            "reflection_texture",
            "reflection_texture_cube",
            "reflection_sampler",
            &t.texcoords,
            t.channel,
        ),
        "reflectionCubeSampler" => write_sampler_2d_or_cube(
            wgsl,
            "reflection_cube_texture_2d",
            "reflection_cube_texture",
            "reflection_cube_sampler",
            &t.texcoords,
            t.channel,
        ),
        "g_VSMTextureSampler" => {
            write!(
                wgsl,
                "textureSample(g_vsm_texture, g_vsm_sampler, vec2({a}{}, {a}{}))",
                t.texcoords.first()?,
                t.texcoords.get(1)?,
            )
            .unwrap();
            write_channel(wgsl, t.channel);
            Some(())
        }
        _ => {
            write!(
                wgsl,
                "textureSample({}, {}, vec2({a}{}, {a}{}))",
                t.name.to_snake().replace("_sampler", "_texture"),
                t.name.to_snake(),
                t.texcoords.first()?,
                t.texcoords.get(1)?,
            )
            .unwrap();
            write_channel(wgsl, t.channel);

            Some(())
        }
    }
}

fn write_sampler_2d_or_cube(
    wgsl: &mut String,
    name_2d: &str,
    name_cube: &str,
    name_sampler: &str,
    texcoords: &[usize],
    channel: Option<char>,
) -> Option<()> {
    match texcoords {
        [u, v] => {
            write!(
                wgsl,
                "textureSample({}, {}, vec2({VAR_PREFIX}{u}, {VAR_PREFIX}{v}))",
                name_2d, name_sampler,
            )
            .unwrap();
            write_channel(wgsl, channel);
            Some(())
        }
        [u, v, w] => {
            write!(
                wgsl,
                "textureSample({}, {}, vec3({VAR_PREFIX}{u}, {VAR_PREFIX}{v}, {VAR_PREFIX}{w}))",
                name_cube, name_sampler,
            )
            .unwrap();
            write_channel(wgsl, channel);
            Some(())
        }
        _ => None,
    }
}

fn write_attribute(wgsl: &mut String, a: &sm4sh_model::database::Attribute) -> Option<()> {
    // Some "attributes" are the simplified result of queries like the eye vector.
    if a.name.starts_with("a_")
        || matches!(
            a.name.as_str(),
            "eye" | "light_position" | "light_map_position" | "bitangent_sign"
        )
    {
        write!(wgsl, "{}", a.name).unwrap();
        write_channel(wgsl, a.channel);
        Some(())
    } else {
        error!("Unrecognized attribute {a}");
        None
    }
}

fn write_parameter(wgsl: &mut String, p: &Parameter) -> Option<()> {
    // TODO: just convert case instead of matching names?
    match p.name.as_str() {
        "MC" => write_parameter_inner(wgsl, "uniforms", &p.field, p.index, p.channel),
        "MC_EFFECT" => write_parameter_inner(wgsl, "effect_uniforms", &p.field, p.index, p.channel),
        "FB0" => write_parameter_inner(wgsl, "fb0", &p.field, p.index, p.channel),
        "FB1" => write_parameter_inner(wgsl, "fb1", &p.field, p.index, p.channel),
        "FB3" => write_parameter_inner(wgsl, "fb3", &p.field, p.index, p.channel),
        "FB4" => write_parameter_inner(wgsl, "fb4", &p.field, p.index, p.channel),
        "FB5" => write_parameter_inner(wgsl, "fb5", &p.field, p.index, p.channel),
        "PerDraw" => match p.field.as_str() {
            "LocalToWorldMatrix" => {
                write!(wgsl, "local_to_world_matrix").unwrap();
                write_index(wgsl, p.index);
                write_channel(wgsl, p.channel);
            }
            "LocalToViewMatrix" => {
                write_parameter_inner(wgsl, "camera", "view", p.index, p.channel)
            }
            "LocalToProjectionMatrix" => {
                write_parameter_inner(wgsl, "camera", "view_projection", p.index, p.channel)
            }
            _ => {
                error!("Unrecognized uniform {p}");
                return None;
            }
        },
        "PerView" => match p.field.as_str() {
            "WorldToProjectionMatrix" => {
                write_parameter_inner(wgsl, "camera", "view_projection", p.index, p.channel)
            }
            "WorldToViewMatrix" => {
                write_parameter_inner(wgsl, "camera", "view", p.index, p.channel)
            }
            "ViewToProjectionMatrix" => {
                write_parameter_inner(wgsl, "camera", "projection", p.index, p.channel)
            }
            _ => {
                error!("Unrecognized uniform {p}");
                return None;
            }
        },
        "CB10" => return None, // TODO: figure out why C10.xyzw is used
        "CB11" => return None, // TODO: figure out why C11.xyzw is used
        _ => {
            error!("Unrecognized uniform {p}");
            return None;
        }
    }
    Some(())
}

fn write_parameter_inner(
    wgsl: &mut String,
    buffer_name: &str,
    field: &str,
    index: Option<usize>,
    channel: Option<char>,
) {
    write!(wgsl, "{buffer_name}.{}", field.to_snake(),).unwrap();
    write_index(wgsl, index);
    write_channel(wgsl, channel);
}

fn write_func(wgsl: &mut String, op: &Operation, args: &[usize]) -> Option<()> {
    let arg0 = args.first();
    let arg1 = args.get(1);
    let arg2 = args.get(2);
    let arg3 = args.get(3);
    let arg4 = args.get(4);
    let arg5 = args.get(5);
    let arg6 = args.get(6);
    let arg7 = args.get(7);

    let a = VAR_PREFIX;
    match op {
        Operation::Unk => return None,
        Operation::Add => write!(wgsl, "{a}{} + {a}{}", arg0?, arg1?).unwrap(),
        Operation::Sub => write!(wgsl, "{a}{} - {a}{}", arg0?, arg1?).unwrap(),
        Operation::Mul => write!(wgsl, "{a}{} * {a}{}", arg0?, arg1?).unwrap(),
        Operation::Div => write!(wgsl, "{a}{} / {a}{}", arg0?, arg1?).unwrap(),
        Operation::Mix => write!(wgsl, "mix({a}{}, {a}{}, {a}{})", arg0?, arg1?, arg2?).unwrap(),
        Operation::Clamp => write!(wgsl, "clamp({a}{}, {a}{}, {a}{})", arg0?, arg1?, arg2?).unwrap(),
        Operation::Min => write!(wgsl, "min({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Max => write!(wgsl, "max({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Abs => write!(wgsl, "abs({a}{})", arg0?).unwrap(),
        Operation::Floor => write!(wgsl, "floor({a}{})", arg0?).unwrap(),
        Operation::Power => write!(wgsl, "pow({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Sqrt => write!(wgsl, "sqrt({a}{})", arg0?).unwrap(),
        Operation::InverseSqrt => write!(wgsl, "inverseSqrt({a}{})", arg0?).unwrap(),
        Operation::Fma => write!(wgsl, "{a}{} * {a}{} + {a}{}", arg0?, arg1?, arg2?).unwrap(),
        Operation::Dot => {
            if args.len() == 6 {
                write!(wgsl,
                    "dot(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{}))",
                    arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
                ).unwrap()
            } else {
                write!(wgsl,
                    "dot(vec4({a}{}, {a}{}, {a}{}, {a}{}), vec4({a}{}, {a}{}, {a}{}, {a}{}))",
                    arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?, arg7?
                ).unwrap()
            }
        }
        Operation::Sin => write!(wgsl, "sin({a}{})", arg0?).unwrap(),
        Operation::Cos => write!(wgsl, "cos({a}{})", arg0?).unwrap(),
        Operation::Exp2 => write!(wgsl, "exp2({a}{})", arg0?).unwrap(),
        Operation::Log2 => write!(wgsl, "log2({a}{})", arg0?).unwrap(),
        Operation::Select => write!(wgsl,
            "mix({a}{}, {a}{}, f32({a}{}))",
            arg2?, arg1?, arg0?
        ).unwrap(),
        Operation::Negate => write!(wgsl, "-({a}{})", arg0?).unwrap(),
        Operation::Equal => write!(wgsl, "{a}{} == {a}{}", arg0?, arg1?).unwrap(),
        Operation::NotEqual => write!(wgsl, "{a}{} != {a}{}", arg0?, arg1?).unwrap(),
        Operation::Less => write!(wgsl, "{a}{} < {a}{}", arg0?, arg1?).unwrap(),
        Operation::Greater => write!(wgsl, "{a}{} > {a}{}", arg0?, arg1?).unwrap(),
        Operation::LessEqual => write!(wgsl, "{a}{} <= {a}{}", arg0?, arg1?).unwrap(),
        Operation::GreaterEqual => write!(wgsl, "{a}{} >= {a}{}", arg0?, arg1?).unwrap(),
        Operation::Fract => write!(wgsl, "fract({a}{})", arg0?).unwrap(),
        Operation::IntBitsToFloat => write!(wgsl, "bitcast<f32>({a}{})", arg0?).unwrap(),
        Operation::FloatBitsToInt => write!(wgsl, "bitcast<i32>({a}{})", arg0?).unwrap(),
        Operation::NormalMapX => write!(wgsl,
            "apply_normal_map(vec3({a}{}, {a}{}, {a}{}), a_Tangent.xyz, a_Binormal.xyz, a_Normal.xyz).x",
            arg0?, arg1?, arg2?
        ).unwrap(),
        Operation::NormalMapY => write!(wgsl,
            "apply_normal_map(vec3({a}{}, {a}{}, {a}{}), a_Tangent.xyz, a_Binormal.xyz, a_Normal.xyz).y",
            arg0?, arg1?, arg2?
        ).unwrap(),
        Operation::NormalMapZ => write!(wgsl,
            "apply_normal_map(vec3({a}{}, {a}{}, {a}{}), a_Tangent.xyz, a_Binormal.xyz, a_Normal.xyz).z",
            arg0?, arg1?, arg2?
        ).unwrap(),
        Operation::NormalizeX => write!(wgsl,
            "normalize(vec4({a}{}, {a}{}, {a}{}, {a}{})).x",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap(),
        Operation::NormalizeY => write!(wgsl,
            "normalize(vec4({a}{}, {a}{}, {a}{}, {a}{})).y",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap(),
        Operation::NormalizeZ => write!(wgsl,
            "normalize(vec4({a}{}, {a}{}, {a}{}, {a}{})).z",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap(),
        Operation::SphereMapCoordX => write!(wgsl,
            "sphere_map_coords(a_Position.xyz, a_Normal.xyz, {a}{}).x",
            arg0?,
        ).unwrap(),
        Operation::SphereMapCoordY => write!(wgsl,
            "sphere_map_coords(a_Position.xyz, a_Normal.xyz, {a}{}).y",
            arg0?,
        ).unwrap(),
        // TODO: Don't assume attributes are already in world space.
        Operation::LocalToWorldPointX => write!(wgsl, "{a}{}", arg0?).unwrap(),
        Operation::LocalToWorldPointY => write!(wgsl, "{a}{}", arg1?).unwrap(),
        Operation::LocalToWorldPointZ => write!(wgsl, "{a}{}", arg2?).unwrap(),
        Operation::LocalToWorldVectorX => write!(wgsl, "{a}{}", arg0?).unwrap(),
        Operation::LocalToWorldVectorY => write!(wgsl, "{a}{}", arg1?).unwrap(),
        Operation::LocalToWorldVectorZ => write!(wgsl, "{a}{}", arg2?).unwrap(),
    }
    Some(())
}

fn write_index(wgsl: &mut String, i: Option<usize>) {
    if let Some(i) = i {
        write!(wgsl, "[{i}]").unwrap();
    }
}

fn write_channel(wgsl: &mut String, c: Option<char>) {
    if let Some(c) = c {
        write!(wgsl, ".{c}").unwrap();
    }
}

fn generate_outputs_wgsl(program: &ShaderProgram) -> String {
    let mut wgsl = String::new();
    for (name, i) in program.output_dependencies.iter() {
        match name.as_str() {
            "out_attr0.x" => writeln!(&mut wgsl, "{OUT_VAR}.x = {VAR_PREFIX}{i};").unwrap(),
            "out_attr0.y" => writeln!(&mut wgsl, "{OUT_VAR}.y = {VAR_PREFIX}{i};").unwrap(),
            "out_attr0.z" => writeln!(&mut wgsl, "{OUT_VAR}.z = {VAR_PREFIX}{i};").unwrap(),
            "out_attr0.w" => writeln!(&mut wgsl, "{OUT_VAR}.w = {VAR_PREFIX}{i};").unwrap(),
            _ => error!("Unrecognized output {name}"),
        }
    }

    wgsl
}
