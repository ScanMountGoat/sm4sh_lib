use std::fmt::Write;

use case::CaseExt;
use indoc::formatdoc;
use log::error;
use sm4sh_model::{
    AlphaFunc,
    database::{Operation, OutputExpr, Parameter, ShaderProgram, Value},
};

const OUT_VAR: &str = "RESULT";
const VAR_PREFIX: &str = "VAR";

/// Generated WGSL model shader code for a material.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct ShaderWgsl {
    assignments: String,
    outputs: Vec<String>,
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
        let mut source = crate::shader::model::SOURCE.to_string();

        source = source.replace("let ASSIGN_VARS_GENERATED = 0.0;", &self.assignments);
        source = source.replace(
            "let ASSIGN_OUT_COLOR_GENERATED = 0.0;",
            &self.outputs.join("\n").replace(OUT_VAR, "out_color"),
        );
        source = source.replace("let ALPHA_TEST_GENERATED = 0.0;", &self.discard);

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
        let value_wgsl = expr_wgsl(expr);
        writeln!(
            &mut wgsl,
            "let {VAR_PREFIX}{i} = {};",
            value_wgsl.unwrap_or("0.0".to_string())
        )
        .unwrap();
    }
    wgsl
}

fn expr_wgsl(expr: &OutputExpr<Operation>) -> Option<String> {
    match expr {
        OutputExpr::Value(value) => value_wgsl(value),
        OutputExpr::Func { op, args } => func_wgsl(op, args),
    }
}

fn value_wgsl(value: &Value) -> Option<String> {
    match value {
        Value::Int(i) => Some(format!("{i:?}")),
        Value::Float(f) => Some(format!("{f:?}")),
        Value::Parameter(p) => parameter_wgsl(p),
        Value::Texture(t) => texture_wgsl(t),
        Value::Attribute(a) => attribute_wgsl(a),
    }
}

fn texture_wgsl(t: &sm4sh_model::database::Texture) -> Option<String> {
    match t.name.as_str() {
        "g_PCFTextureSampler"
        | "sampler0"
        | "Sampler11"
        | "samplerA"
        | "samplerB"
        | "multiplicationSampler"
        | "frameSampler" => None,
        "colorSampler2" | "colorSampler3" => None, // TODO: load all color textures
        "reflectionSampler" => sampler_2d_or_cube(
            "reflection_texture",
            "reflection_texture_cube",
            "reflection_sampler",
            &t.texcoords,
            t.channel,
        ),
        "reflectionCubeSampler" => sampler_2d_or_cube(
            "reflection_cube_texture_2d",
            "reflection_cube_texture",
            "reflection_cube_sampler",
            &t.texcoords,
            t.channel,
        ),
        "g_VSMTextureSampler" => Some(format!(
            "textureSample({}, {}, vec2({}, {})){}",
            "g_vsm_texture",
            "g_vsm_sampler",
            arg(&t.texcoords, 0)?,
            arg(&t.texcoords, 1)?,
            channel_wgsl(t.channel)
        )),
        _ => Some(format!(
            "textureSample({}, {}, vec2({}, {})){}",
            t.name.to_snake().replace("_sampler", "_texture"),
            t.name.to_snake(),
            arg(&t.texcoords, 0)?,
            arg(&t.texcoords, 1)?,
            channel_wgsl(t.channel)
        )),
    }
}

fn sampler_2d_or_cube(
    name_2d: &str,
    name_cube: &str,
    name_sampler: &str,
    texcoords: &[usize],
    channel: Option<char>,
) -> Option<String> {
    match texcoords {
        [u, v] => Some(format!(
            "textureSample({}, {}, vec2({VAR_PREFIX}{u}, {VAR_PREFIX}{v})){}",
            name_2d,
            name_sampler,
            channel_wgsl(channel)
        )),
        [u, v, w] => Some(format!(
            "textureSample({}, {}, vec3({VAR_PREFIX}{u}, {VAR_PREFIX}{v}, {VAR_PREFIX}{w})){}",
            name_cube,
            name_sampler,
            channel_wgsl(channel)
        )),
        _ => None,
    }
}

fn attribute_wgsl(a: &sm4sh_model::database::Attribute) -> Option<String> {
    if a.name.starts_with("a_") {
        Some(format!("{}{}", a.name, channel_wgsl(a.channel)))
    } else {
        error!("Unrecognized attribute {a}");
        None
    }
}

fn parameter_wgsl(p: &Parameter) -> Option<String> {
    // TODO: just convert case instead of matching names?
    match p.name.as_str() {
        "MC" => parameter_wgsl_inner(p, "uniforms"),
        "MC_EFFECT" => parameter_wgsl_inner(p, "effect_uniforms"),
        "FB0" => parameter_wgsl_inner(p, "fb0"),
        "FB1" => parameter_wgsl_inner(p, "fb1"),
        "FB3" => parameter_wgsl_inner(p, "fb3"),
        "FB4" => parameter_wgsl_inner(p, "fb4"),
        "FB5" => parameter_wgsl_inner(p, "fb5"),
        "PerDraw" => match p.field.as_str() {
            "LocalToWorldMatrix" => Some(format!(
                "local_to_world_matrix{}{}",
                index_wgsl(p.index),
                channel_wgsl(p.channel)
            )),
            "LocalToViewMatrix" => Some(format!(
                "camera.view{}{}",
                index_wgsl(p.index),
                channel_wgsl(p.channel)
            )),
            "LocalToProjectionMatrix" => Some(format!(
                "camera.view_projection{}{}",
                index_wgsl(p.index),
                channel_wgsl(p.channel)
            )),
            _ => {
                error!("Unrecognized uniform {p}");
                None
            }
        },
        "PerView" => match p.field.as_str() {
            "WorldToProjectionMatrix" => Some(format!(
                "camera.view_projection{}{}",
                index_wgsl(p.index),
                channel_wgsl(p.channel)
            )),
            "WorldToViewMatrix" => Some(format!(
                "camera.view{}{}",
                index_wgsl(p.index),
                channel_wgsl(p.channel)
            )),
            "ViewToProjectionMatrix" => Some(format!(
                "camera.projection{}{}",
                index_wgsl(p.index),
                channel_wgsl(p.channel)
            )),
            _ => {
                error!("Unrecognized uniform {p}");
                None
            }
        },
        "CB10" => None, // TODO: figure out why C10.xyzw is used
        "CB11" => None, // TODO: figure out why C11.xyzw is used
        _ => {
            error!("Unrecognized uniform {p}");
            None
        }
    }
}

fn parameter_wgsl_inner(p: &Parameter, buffer_name: &str) -> Option<String> {
    Some(format!(
        "{buffer_name}.{}{}{}",
        p.field.to_snake(),
        index_wgsl(p.index),
        channel_wgsl(p.channel)
    ))
}

fn func_wgsl(op: &Operation, args: &[usize]) -> Option<String> {
    let arg0 = arg(args, 0);
    let arg1 = arg(args, 1);
    let arg2 = arg(args, 2);
    let arg3 = arg(args, 3);
    let arg4 = arg(args, 4);
    let arg5 = arg(args, 5);
    let arg6 = arg(args, 6);
    let arg7 = arg(args, 7);

    match op {
        Operation::Add => Some(format!("{} + {}", arg0?, arg1?)),
        Operation::Sub => Some(format!("{} - {}", arg0?, arg1?)),
        Operation::Mul => Some(format!("{} * {}", arg0?, arg1?)),
        Operation::Div => Some(format!("{} / {}", arg0?, arg1?)),
        Operation::Mix => Some(format!("mix({}, {}, {})", arg0?, arg1?, arg2?)),
        Operation::Clamp => Some(format!("clamp({}, {}, {})", arg0?, arg1?, arg2?)),
        Operation::Min => Some(format!("min({}, {})", arg0?, arg1?)),
        Operation::Max => Some(format!("max({}, {})", arg0?, arg1?)),
        Operation::Abs => Some(format!("abs({})", arg0?)),
        Operation::Floor => Some(format!("floor({})", arg0?)),
        Operation::Power => Some(format!("pow({}, {})", arg0?, arg1?)),
        Operation::Sqrt => Some(format!("sqrt({})", arg0?)),
        Operation::InverseSqrt => Some(format!("inverseSqrt({})", arg0?)),
        Operation::Fma => Some(format!("{} * {} + {}", arg0?, arg1?, arg2?)),
        Operation::Dot4 => Some(format!(
            "dot(vec4({}, {}, {}, {}), vec4({}, {}, {}, {}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?, arg7?
        )),
        Operation::Sin => Some(format!("sin({})", arg0?)),
        Operation::Cos => Some(format!("cos({})", arg0?)),
        Operation::Exp2 => Some(format!("exp2({})", arg0?)),
        Operation::Log2 => Some(format!("log2({})", arg0?)),
        Operation::Select => Some(format!("mix({}, {}, f32({}))", arg2?, arg1?, arg0?)),
        Operation::Negate => Some(format!("-({})", arg0?)),
        Operation::Equal => Some(format!("{} == {}", arg0?, arg1?)),
        Operation::NotEqual => Some(format!("{} != {}", arg0?, arg1?)),
        Operation::Less => Some(format!("{} < {}", arg0?, arg1?)),
        Operation::Greater => Some(format!("{} > {}", arg0?, arg1?)),
        Operation::LessEqual => Some(format!("{} <= {}", arg0?, arg1?)),
        Operation::GreaterEqual => Some(format!("{} >= {}", arg0?, arg1?)),
        Operation::Fract => Some(format!("fract({})", arg0?)),
        Operation::IntBitsToFloat => Some(format!("bitcast<f32>({})", arg0?)),
        Operation::FloatBitsToInt => Some(format!("bitcast<i32>({})", arg0?)),
        Operation::Unk => None,
    }
}

fn arg(args: &[usize], i: usize) -> Option<String> {
    Some(format!("{VAR_PREFIX}{}", args.get(i)?))
}

fn index_wgsl(i: Option<usize>) -> String {
    i.map(|i| format!("[{i}]")).unwrap_or_default()
}

fn channel_wgsl(c: Option<char>) -> String {
    c.map(|c| format!(".{c}")).unwrap_or_default()
}

fn generate_outputs_wgsl(program: &ShaderProgram) -> Vec<String> {
    program
        .output_dependencies
        .iter()
        .map(|(name, i)| {
            let mut wgsl = String::new();
            match name.as_str() {
                "out_attr0.x" => writeln!(&mut wgsl, "{OUT_VAR}.x = {VAR_PREFIX}{i};").unwrap(),
                "out_attr0.y" => writeln!(&mut wgsl, "{OUT_VAR}.y = {VAR_PREFIX}{i};").unwrap(),
                "out_attr0.z" => writeln!(&mut wgsl, "{OUT_VAR}.z = {VAR_PREFIX}{i};").unwrap(),
                "out_attr0.w" => writeln!(&mut wgsl, "{OUT_VAR}.w = {VAR_PREFIX}{i};").unwrap(),
                _ => error!("Unrecognized output {name}"),
            }

            wgsl
        })
        .collect()
}
