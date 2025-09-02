use std::fmt::Write;

use case::CaseExt;
use log::error;
use sm4sh_model::database::{Operation, OutputExpr, Parameter, ShaderProgram, Value};

const OUT_VAR: &str = "RESULT";
const VAR_PREFIX: &str = "VAR";

/// Generated WGSL model shader code for a material.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct ShaderWgsl {
    assignments: String,
    outputs: Vec<String>,
}

impl ShaderWgsl {
    pub fn new(program: Option<&ShaderProgram>) -> Self {
        let (assignments, outputs) = program
            .map(|p| (generate_assignments_wgsl(p), generate_outputs_wgsl(p)))
            .unwrap_or_default();

        Self {
            assignments,
            outputs,
        }
    }

    pub fn create_model_shader(&self) -> String {
        let mut source = crate::shader::model::SOURCE.to_string();

        source = source.replace("let ASSIGN_VARS_GENERATED = 0.0;", &self.assignments);
        source = source.replace(
            "let ASSIGN_OUT_COLOR_GENERATED = 0.0;",
            &self.outputs.join("\n").replace(OUT_VAR, "out_color"),
        );

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
    // TODO: Fill in defaults for unknown constant buffers.
    // TODO: Remap attribute names to the expected values.
    match value {
        Value::Constant(f) => Some(format!("{f:?}")),
        Value::Parameter(p) => parameter_wgsl(p),
        Value::Texture(t) => texture_wgsl(t),
        Value::Attribute(a) => attribute_wgsl(a),
    }
}

fn texture_wgsl(t: &sm4sh_model::database::Texture) -> Option<String> {
    match t.name.as_str() {
        "g_PCFTextureSampler" | "g_VSMTextureSampler" | "sampler0" | "sampler11" => None,
        "reflectionCubeSampler" => Some(format!(
            "textureSample({}, {}, vec3({}, {}, {})){}",
            t.name.to_snake().replace("_sampler", "_texture"),
            t.name.to_snake(),
            arg(&t.texcoords, 0)?,
            arg(&t.texcoords, 1)?,
            arg(&t.texcoords, 2)?,
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

fn attribute_wgsl(a: &sm4sh_model::database::Attribute) -> Option<String> {
    if a.name.starts_with("a_") {
        Some(format!("{}{}", a.name, channel_wgsl(a.channel)))
    } else {
        error!("Unrecognized attribute {a}");
        None
    }
}

fn parameter_wgsl(p: &Parameter) -> Option<String> {
    match p.name.as_str() {
        "MC" => Some(format!(
            "uniforms.{}{}{}",
            p.field,
            p.index.map(|i| format!("[{i}]")).unwrap_or_default(),
            channel_wgsl(p.channel)
        )),
        _ => {
            error!("Unrecognized uniform {p}");
            None
        }
    }
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
        Operation::Select => Some(format!("mix({}, {}, f32({}))", arg2?, arg1?, arg0?)),
        Operation::Negate => Some(format!("-({})", arg0?)),
        Operation::Unk => None,
    }
}

fn arg(args: &[usize], i: usize) -> Option<String> {
    Some(format!("{VAR_PREFIX}{}", args.get(i)?))
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
