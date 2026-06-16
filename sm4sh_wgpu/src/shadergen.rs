use std::{collections::BTreeSet, fmt::Write, sync::LazyLock};

use aho_corasick::AhoCorasick;
use case::CaseExt;
use indoc::formatdoc;
use log::error;
use sm4sh_model::{
    AlphaFunc,
    database::{
        ChannelXyz, Operation, OperationXyz, OutputExpr, OutputExprXyz, Parameter, ShaderProgram,
        Value, ValueXyz,
    },
};
use smol_str::{SmolStr, format_smolstr};

const OUT_VAR: &str = "out_color";
const VAR_PREFIX: &str = "VAR_";
const VAR_PREFIX_XYZ: &str = "VAR_XYZ_";

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

    // TODO: Reindex the used exprs in a separate preprocessing step?
    let mut used_exprs = BTreeSet::new();
    let mut used_exprs_xyz = BTreeSet::new();
    if let Some(xyz) = program
        .output_dependencies_xyz
        .get(&SmolStr::from("out_attr0.xyz"))
    {
        visit_exprs_xyz(
            *xyz,
            &program.exprs,
            &program.exprs_xyz,
            &mut used_exprs,
            &mut used_exprs_xyz,
        );

        if let Some(xyz) = program
            .output_dependencies
            .get(&SmolStr::from("out_attr0.w"))
        {
            visit_exprs(*xyz, &program.exprs, &mut used_exprs);
        }
    } else {
        // If XYZ merging isn't possible, there shouldn't be any used exprs.
        used_exprs = (0..program.exprs.len()).collect();
    }

    for (i, expr) in program.exprs.iter().enumerate() {
        if used_exprs.contains(&i) {
            write!(&mut wgsl, "let {VAR_PREFIX}{i} = ",).unwrap();
            if write_expr(&mut wgsl, expr).is_none() {
                write!(wgsl, "0.0").unwrap();
            }
            writeln!(&mut wgsl, ";",).unwrap();
        }
    }

    for (i, expr) in program.exprs_xyz.iter().enumerate() {
        write!(&mut wgsl, "let {VAR_PREFIX_XYZ}{i} = ",).unwrap();
        if write_expr_xyz(&mut wgsl, expr).is_none() {
            write!(&mut wgsl, "vec3(0.0)").unwrap();
        }
        writeln!(&mut wgsl, ";",).unwrap();
    }
    wgsl
}

fn visit_exprs(i: usize, exprs: &[OutputExpr<Operation>], visited: &mut BTreeSet<usize>) {
    if visited.insert(i) {
        match &exprs[i] {
            OutputExpr::Value(Value::Texture(t)) => {
                for arg in &t.texcoords {
                    visit_exprs(*arg, exprs, visited);
                }
            }
            OutputExpr::Func { args, .. } => {
                for arg in args {
                    visit_exprs(*arg, exprs, visited);
                }
            }
            OutputExpr::Value(_) => (),
        }
    }
}

fn visit_exprs_xyz(
    xyz: usize,
    exprs: &[OutputExpr<Operation>],
    exprs_xyz: &[OutputExprXyz<OperationXyz>],
    visited: &mut BTreeSet<usize>,
    visited_xyz: &mut BTreeSet<usize>,
) {
    if visited_xyz.insert(xyz) {
        match &exprs_xyz[xyz] {
            OutputExprXyz::Value(ValueXyz::Texture(t)) => {
                for arg in &t.texcoords {
                    visit_exprs(*arg, exprs, visited);
                }
            }
            OutputExprXyz::Func { args, .. } => {
                for arg in args {
                    visit_exprs_xyz(*arg, exprs, exprs_xyz, visited, visited_xyz);
                }
            }
            OutputExprXyz::Value(_) => (),
        }
    }
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
    write_texture_inner(wgsl, &t.name, &t.texcoords)?;
    write_channel(wgsl, t.channel);
    Some(())
}

fn write_texture_inner(wgsl: &mut String, name: &str, texcoords: &[usize]) -> Option<()> {
    let a = VAR_PREFIX;
    match name {
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
            texcoords,
        ),
        "reflectionCubeSampler" => write_sampler_2d_or_cube(
            wgsl,
            "reflection_cube_texture_2d",
            "reflection_cube_texture",
            "reflection_cube_sampler",
            texcoords,
        ),
        "g_VSMTextureSampler" => {
            write!(
                wgsl,
                "textureSample(g_vsm_texture, g_vsm_sampler, vec2({a}{}, {a}{}))",
                texcoords.first()?,
                texcoords.get(1)?,
            )
            .unwrap();
            Some(())
        }
        _ => {
            write!(
                wgsl,
                "textureSample({}, {}, vec2({a}{}, {a}{}))",
                name.to_snake().replace("_sampler", "_texture"),
                name.to_snake(),
                texcoords.first()?,
                texcoords.get(1)?,
            )
            .unwrap();
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
) -> Option<()> {
    match texcoords {
        [u, v] => {
            write!(
                wgsl,
                "textureSample({}, {}, vec2({VAR_PREFIX}{u}, {VAR_PREFIX}{v}))",
                name_2d, name_sampler,
            )
            .unwrap();
            Some(())
        }
        [u, v, w] => {
            // Assume 3D textures aren't used, so UVW coordinates should always be a cube map.
            write!(
                wgsl,
                "textureSample({}, {}, vec3({VAR_PREFIX}{u}, {VAR_PREFIX}{v}, {VAR_PREFIX}{w}))",
                name_cube, name_sampler,
            )
            .unwrap();
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
    if write_parameter_inner(wgsl, &p.name, &p.field, p.index).is_none() {
        error!("Unrecognized uniform {p}");
        None
    } else {
        write_channel(wgsl, p.channel);
        Some(())
    }
}

fn write_parameter_inner(
    wgsl: &mut String,
    name: &str,
    field: &str,
    index: Option<usize>,
) -> Option<()> {
    // TODO: just convert case instead of matching names?
    match name {
        "MC" => write_parameter_inner2(wgsl, "uniforms", field, index),
        "MC_EFFECT" => write_parameter_inner2(wgsl, "effect_uniforms", field, index),
        "FB0" => write_parameter_inner2(wgsl, "fb0", field, index),
        "FB1" => write_parameter_inner2(wgsl, "fb1", field, index),
        "FB3" => write_parameter_inner2(wgsl, "fb3", field, index),
        // TODO: fix shader handling for effect_light_entry
        // "FB4" => write_parameter_inner(wgsl, "fb4", field, index),
        "FB5" => write_parameter_inner2(wgsl, "fb5", field, index),
        "PerDraw" => match field {
            "LocalToWorldMatrix" => {
                write!(wgsl, "local_to_world_matrix").unwrap();
                write_index(wgsl, index);
            }
            "LocalToViewMatrix" => write_parameter_inner2(wgsl, "camera", "view", index),
            "LocalToProjectionMatrix" => {
                write_parameter_inner2(wgsl, "camera", "view_projection", index)
            }
            _ => {
                return None;
            }
        },
        "PerView" => match field {
            "WorldToProjectionMatrix" => {
                write_parameter_inner2(wgsl, "camera", "view_projection", index)
            }
            "WorldToViewMatrix" => write_parameter_inner2(wgsl, "camera", "view", index),
            "ViewToProjectionMatrix" => write_parameter_inner2(wgsl, "camera", "projection", index),
            _ => {
                return None;
            }
        },
        _ => {
            return None;
        }
    }
    Some(())
}

fn write_parameter_inner2(wgsl: &mut String, buffer_name: &str, field: &str, index: Option<usize>) {
    write!(wgsl, "{buffer_name}.{}", field.to_snake()).unwrap();
    write_index(wgsl, index);
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
    let arg8 = args.get(8);
    let arg9 = args.get(9);
    let arg10 = args.get(10);

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
        Operation::Dot3 => write!(wgsl,
            "dot(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
        Operation::Dot4 => write!(wgsl,
            "dot(vec4({a}{}, {a}{}, {a}{}, {a}{}), vec4({a}{}, {a}{}, {a}{}, {a}{}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?, arg7?
        ).unwrap(),
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
            "normalize(vec3({a}{}, {a}{}, {a}{})).x", 
            arg0?, arg1?, arg2?
        ).unwrap(),
        Operation::NormalizeY => write!(wgsl,
            "normalize(vec3({a}{}, {a}{}, {a}{})).y", 
            arg0?, arg1?, arg2?
        ).unwrap(),
        Operation::NormalizeZ => write!(wgsl,
            "normalize(vec3({a}{}, {a}{}, {a}{})).z", 
            arg0?, arg1?, arg2?
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
        Operation::VarianceShadow => write!(wgsl,
            "variance_shadow({a}{}, {a}{}, {a}{}, {a}{})",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap(),
        Operation::BlinnPhongSpecular => write!(wgsl,
            "blinn_phong_spec(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{}), {a}{})",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?, arg7?, arg8?, arg9?
        ).unwrap(),
        Operation::AnisotropicSpecular => write!(wgsl,
            "anisotropic_spec(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{}), vec2({a}{}, {a}{}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?, arg7?, arg8?, arg9?, arg10?
        ).unwrap(),
        Operation::Fresnel => write!(wgsl,
            "fresnel(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{}), {a}{})",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?,
        ).unwrap(),
        Operation::TintColorX => write!(wgsl,
            "tint_color(vec3({a}{}, {a}{}, {a}{}), {a}{}).x",
            arg0?, arg1?, arg2?, arg3?,
        ).unwrap(),
        Operation::TintColorY => write!(wgsl,
            "tint_color(vec3({a}{}, {a}{}, {a}{}), {a}{}).y",
            arg0?, arg1?, arg2?, arg3?,
        ).unwrap(),
        Operation::TintColorZ => write!(wgsl,
            "tint_color(vec3({a}{}, {a}{}, {a}{}), {a}{}).z",
            arg0?, arg1?, arg2?, arg3?,
        ).unwrap(),
        Operation::NegReflectX => write!(wgsl,
            "-reflect(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{})).x",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
        Operation::NegReflectY => write!(wgsl,
            "-reflect(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{})).y",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
        Operation::NegReflectZ => write!(wgsl,
            "-reflect(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{})).z",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
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
    // TODO: Discard the scalar expressions that aren't actually used?

    // Assume there is only one fragment output.
    // Most shaders can merge XYZ channels for easier to read code.
    if let Some(xyz) = program
        .output_dependencies_xyz
        .get(&SmolStr::from("out_attr0.xyz"))
    {
        for c in "xyz".chars() {
            writeln!(&mut wgsl, "{OUT_VAR}.{c} = {VAR_PREFIX_XYZ}{xyz}.{c};").unwrap()
        }
    } else {
        for c in "xyz".chars() {
            if let Some(i) = program
                .output_dependencies
                .get(&format_smolstr!("out_attr0.{c}"))
            {
                writeln!(&mut wgsl, "{OUT_VAR}.{c} = {VAR_PREFIX}{i}.{c};").unwrap()
            }
        }
    }
    // Alpha code is always handled separately as scalar expressions.
    if let Some(i) = program
        .output_dependencies
        .get(&SmolStr::from("out_attr0.w"))
    {
        writeln!(&mut wgsl, "{OUT_VAR}.w = {VAR_PREFIX}{i};").unwrap()
    }

    wgsl
}

fn write_expr_xyz(wgsl: &mut String, value: &OutputExprXyz<OperationXyz>) -> Option<()> {
    match value {
        OutputExprXyz::Func { op, args, channel } => write_func_xyz(wgsl, op, args, *channel),
        OutputExprXyz::Value(v) => write_value_xyz(wgsl, v),
    }
}

fn write_func_xyz(
    wgsl: &mut String,
    op: &OperationXyz,
    args: &[usize],
    channel: Option<ChannelXyz>,
) -> Option<()> {
    let arg0 = args.first();
    let arg1 = args.get(1);
    let arg2 = args.get(2);
    let arg3 = args.get(3);
    let arg4 = args.get(4);
    let arg5 = args.get(5);
    let arg6 = args.get(6);
    let arg7 = args.get(7);

    // TODO: Will these operations all work with xyz inputs?
    let a = VAR_PREFIX_XYZ;
    match op {
        OperationXyz::Unk => return None,
        OperationXyz::Add => write!(wgsl, "({a}{} + {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::Sub => write!(wgsl, "({a}{} - {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::Mul => write!(wgsl, "({a}{} * {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::Div => write!(wgsl, "({a}{} / {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::Mix => write!(wgsl, "mix({a}{}, {a}{}, {a}{})", arg0?, arg1?, arg2?).unwrap(),
        OperationXyz::Clamp => {
            write!(wgsl, "clamp({a}{}, {a}{}, {a}{})", arg0?, arg1?, arg2?).unwrap()
        }
        OperationXyz::Min => write!(wgsl, "min({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::Max => write!(wgsl, "max({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::Abs => write!(wgsl, "abs({a}{})", arg0?).unwrap(),
        OperationXyz::Floor => write!(wgsl, "floor({a}{})", arg0?).unwrap(),
        OperationXyz::Power => write!(wgsl, "pow({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::Sqrt => write!(wgsl, "sqrt({a}{})", arg0?).unwrap(),
        OperationXyz::InverseSqrt => write!(wgsl, "inverseSqrt({a}{})", arg0?).unwrap(),
        OperationXyz::Fma => write!(wgsl, "({a}{} * {a}{} + {a}{})", arg0?, arg1?, arg2?).unwrap(),
        OperationXyz::Dot => write!(
            wgsl,
            "vec3(dot(vec4({a}{}.x, {a}{}.x, {a}{}.x, {a}{}.x), vec4({a}{}.x, {a}{}.x, {a}{}.x, {a}{}.x)))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?, arg7?
        )
        .unwrap(),
        OperationXyz::Sin => write!(wgsl, "sin({a}{})", arg0?).unwrap(),
        OperationXyz::Cos => write!(wgsl, "cos({a}{})", arg0?).unwrap(),
        OperationXyz::Exp2 => write!(wgsl, "exp2({a}{})", arg0?).unwrap(),
        OperationXyz::Log2 => write!(wgsl, "log2({a}{})", arg0?).unwrap(),
        OperationXyz::Select => {
            write!(wgsl, "mix({a}{}, {a}{}, vec3<f32>({a}{}))", arg2?, arg1?, arg0?).unwrap()
        }
        OperationXyz::Negate => write!(wgsl, "-({a}{})", arg0?).unwrap(),
        OperationXyz::Equal => write!(wgsl, "({a}{} == {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::NotEqual => write!(wgsl, "({a}{} != {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::Less => write!(wgsl, "({a}{} < {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::Greater => write!(wgsl, "({a}{} > {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::LessEqual => write!(wgsl, "({a}{} <= {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::GreaterEqual => write!(wgsl, "({a}{} >= {a}{})", arg0?, arg1?).unwrap(),
        OperationXyz::Fract => write!(wgsl, "fract({a}{})", arg0?).unwrap(),
        OperationXyz::IntBitsToFloat => write!(wgsl, "bitcast<f32>({a}{})", arg0?).unwrap(),
        OperationXyz::FloatBitsToInt => write!(wgsl, "bitcast<i32>({a}{})", arg0?).unwrap(),
        OperationXyz::NormalMap => write!(
            wgsl,
            "apply_normal_map({a}{}, a_Tangent.xyz, a_Binormal.xyz, a_Normal.xyz)",
            arg0?
        )
        .unwrap(),
        OperationXyz::Normalize => write!(wgsl, "normalize({a}{})", arg0?).unwrap(),
        // TODO: Don't assume attributes are already in world space.
        OperationXyz::LocalToWorldPoint => write!(wgsl, "{a}{}", arg0?).unwrap(),
        OperationXyz::LocalToWorldVector => write!(wgsl, "{a}{}", arg0?).unwrap(),
        OperationXyz::VarianceShadow => write!(
            wgsl,
            "vec3(variance_shadow({a}{}.x, {a}{}.x, {a}{}.x, {a}{}.x))",
            arg0?, arg1?, arg2?, arg3?
        )
        .unwrap(),
        OperationXyz::BlinnPhongSpecular => write!(
            wgsl,
            "vec3(blinn_phong_spec({a}{}, {a}{}, {a}{}, {a}{}.x))",
            arg0?, arg1?, arg2?, arg3?,
        )
        .unwrap(),
        OperationXyz::AnisotropicSpecular => write!(
            wgsl,
            "vec3(anisotropic_spec({a}{}, {a}{}, {a}{}, vec2({a}{}.x, {a}{}.x)))",
            arg0?, arg1?, arg2?, arg3?, arg4?
        )
        .unwrap(),
        OperationXyz::Fresnel => {
            write!(wgsl, "fresnel({a}{}, {a}{}, {a}{}.x)", arg0?, arg1?, arg2?).unwrap()
        }
        OperationXyz::TintColor => {
            write!(wgsl, "tint_color({a}{}, {a}{}.x)", arg0?, arg1?).unwrap()
        }
        OperationXyz::NegReflect => {
            write!(wgsl, "-reflect({a}{}, {a}{})", arg0?, arg1?).unwrap()
        }
    }
    write_channel_xyz(wgsl, channel);
    Some(())
}

// TODO: share code with scalar?
fn write_value_xyz(wgsl: &mut String, value: &ValueXyz) -> Option<()> {
    match value {
        ValueXyz::Texture(t) => {
            write_texture_inner(wgsl, &t.name, &t.texcoords)?;
            write_channel_xyz(wgsl, t.channel);
            Some(())
        }
        ValueXyz::Attribute(a) => {
            // Some "attributes" are the simplified result of queries like the eye vector.
            if a.name.starts_with("a_")
                || matches!(
                    a.name.as_str(),
                    "eye" | "light_position" | "light_map_position" | "bitangent_sign"
                )
            {
                write!(wgsl, "{}", a.name).unwrap();
                write_channel_xyz(wgsl, a.channel);
                Some(())
            } else {
                error!("Unrecognized attribute {a}");
                None
            }
        }
        ValueXyz::Parameter(p) => {
            if write_parameter_inner(wgsl, &p.name, &p.field, p.index).is_none() {
                error!("Unrecognized parameter {p}");
                None
            } else {
                write_channel_xyz(wgsl, p.channel);
                Some(())
            }
        }
        ValueXyz::Float(f) => {
            if f.iter().all(|f| f.is_finite()) {
                write!(wgsl, "vec3({:?}, {:?}, {:?})", f[0], f[1], f[2]).unwrap();
                Some(())
            } else {
                error!("Unsupported float literals {f:?}");
                None
            }
        }
    }
}

fn write_channel_xyz(wgsl: &mut String, c: Option<ChannelXyz>) {
    if let Some(c) = c {
        match c {
            ChannelXyz::Xyz => write!(wgsl, ".xyz").unwrap(),
            ChannelXyz::X => write!(wgsl, ".xxx").unwrap(),
            ChannelXyz::Y => write!(wgsl, ".yyy").unwrap(),
            ChannelXyz::Z => write!(wgsl, ".zzz").unwrap(),
            ChannelXyz::W => write!(wgsl, ".www").unwrap(),
        }
    }
}
