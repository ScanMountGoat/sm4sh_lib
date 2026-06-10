use std::borrow::Cow;

use log::error;
use smol_str::SmolStr;
use xc3_shader::{
    expr::{
        ExprCache, OutputExpr, output_expr,
        xyz::{ExprCacheXyz, MergeXyzArgs, OperationXyzChannel, OutputExprXyz, merge_xyz_exprs},
    },
    graph::{
        BinaryOp, Expr, Graph, UnaryOp,
        glsl::{GlslGraph, merge_vertex_fragment},
    },
};

mod query;
use query::*;

// Faster than the default hash implementation.
type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

#[derive(Debug, PartialEq, Clone)]
pub struct ShaderProgram {
    /// Indices into [exprs](#structfield.exprs) for values assigned to a fragment output.
    pub output_dependencies: IndexMap<SmolStr, usize>,

    /// Unique exprs used for this program.
    pub exprs: Vec<OutputExpr<Operation>>,

    /// Indices into [exprs_xyz](#structfield.exprs_xyz) for values assigned to the XYZ channels of a fragment output.
    ///
    /// This only contains values if the XYZ channels can be successfully merged.
    pub output_dependencies_xyz: IndexMap<SmolStr, usize>,

    /// Unique merged XYZ exprs used for this program.
    pub exprs_xyz: Vec<OutputExprXyz<OperationXyz>>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
pub enum Operation {
    #[default]
    Unk,
    Add,
    Sub,
    Mul,
    Div,
    Mix,
    Clamp,
    Min,
    Max,
    Abs,
    Floor,
    Power,
    Sqrt,
    InverseSqrt,
    Fma,
    Dot,
    Sin,
    Cos,
    Exp2,
    Log2,
    Fract,
    IntBitsToFloat,
    FloatBitsToInt,
    Select,
    Negate,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    NormalMapX,
    NormalMapY,
    NormalMapZ,
    NormalizeX,
    NormalizeY,
    NormalizeZ,
    SphereMapCoordX,
    SphereMapCoordY,
    LocalToWorldPointX,
    LocalToWorldPointY,
    LocalToWorldPointZ,
    LocalToWorldVectorX,
    LocalToWorldVectorY,
    LocalToWorldVectorZ,
    VarianceShadow,
    BlinnPhongSpecular,
    AnisotropicSpecular,
    Fresnel,
    TintColorX,
    TintColorY,
    TintColorZ,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl xc3_shader::expr::Operation for Operation {
    fn query_operation_args<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Self, Vec<&'a Expr>)> {
        // TODO: Figure out why op_mix doesn't work with simplification.
        // TODO: detect reflect to simplify cube maps

        op_normal_map(graph, expr)
            // .or_else(|| op_mix(graph, expr))
            .or_else(|| op_blinn_phong_spec(graph, expr))
            .or_else(|| op_blinn_phong_spec_anisotropic(graph, expr))
            .or_else(|| op_fresnel(graph, expr))
            .or_else(|| op_variance_shadow(graph, expr))
            .or_else(|| op_sphere_map_coords(graph, expr))
            .or_else(|| op_local_to_world_point(graph, expr))
            .or_else(|| op_local_to_world_vector(graph, expr))
            .or_else(|| op_tint_color(graph, expr))
            .or_else(|| op_normalize(graph, expr))
            .or_else(|| op_pow(graph, expr))
            .or_else(|| op_sqrt(graph, expr))
            .or_else(|| op_dot(graph, expr))
            .or_else(|| op_div(graph, expr))
            .or_else(|| ternary(graph, expr))
            .or_else(|| binary_op(graph, expr, BinaryOp::Add, Operation::Add))
            .or_else(|| binary_op(graph, expr, BinaryOp::Sub, Operation::Sub))
            .or_else(|| binary_op(graph, expr, BinaryOp::Mul, Operation::Mul))
            .or_else(|| unary_op(graph, expr, UnaryOp::Negate, Operation::Negate))
            .or_else(|| op_func(graph, expr, "clamp", Operation::Clamp))
            .or_else(|| op_func(graph, expr, "min", Operation::Min))
            .or_else(|| op_func(graph, expr, "max", Operation::Max))
            .or_else(|| op_func(graph, expr, "inversesqrt", Operation::InverseSqrt))
            .or_else(|| op_func(graph, expr, "abs", Operation::Abs))
            .or_else(|| op_func(graph, expr, "floor", Operation::Floor))
            .or_else(|| op_func(graph, expr, "fma", Operation::Fma))
            .or_else(|| op_func(graph, expr, "sin", Operation::Sin))
            .or_else(|| op_func(graph, expr, "cos", Operation::Cos))
            .or_else(|| op_func(graph, expr, "log2", Operation::Log2))
            .or_else(|| op_func(graph, expr, "exp2", Operation::Exp2))
            .or_else(|| op_func(graph, expr, "fract", Operation::Fract))
            .or_else(|| op_func(graph, expr, "intBitsToFloat", Operation::IntBitsToFloat))
            .or_else(|| op_func(graph, expr, "floatBitsToInt", Operation::FloatBitsToInt))
            .or_else(|| binary_op(graph, expr, BinaryOp::Equal, Operation::Equal))
            .or_else(|| binary_op(graph, expr, BinaryOp::NotEqual, Operation::NotEqual))
            .or_else(|| binary_op(graph, expr, BinaryOp::Greater, Operation::Greater))
            .or_else(|| binary_op(graph, expr, BinaryOp::GreaterEqual, Operation::GreaterEqual))
            .or_else(|| binary_op(graph, expr, BinaryOp::Less, Operation::Less))
            .or_else(|| binary_op(graph, expr, BinaryOp::LessEqual, Operation::LessEqual))
            .or_else(|| {
                error!("Unsupported expression {expr:?}");
                None
            })
    }

    fn preprocess_expr<'a>(graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr> {
        if let Some(new_expr) =
            latte_texture_cube_coords(graph, expr).or_else(|| fragment_tangent(graph, expr))
        {
            Cow::Owned(new_expr)
        } else {
            Cow::Borrowed(expr)
        }
    }

    fn preprocess_value_expr<'a>(_graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr> {
        Cow::Borrowed(expr)
    }
}

pub fn shader_from_glsl(vertex: GlslGraph, fragment: GlslGraph) -> ShaderProgram {
    // Create a combined graph that links vertex outputs to fragment inputs.
    // This effectively moves all shader logic to the fragment shader.
    // This simplifies generating shader code or material nodes in 3D applications.
    let frag_attributes = fragment.attributes.clone();
    let graph = merge_vertex_fragment(
        GlslGraph {
            graph: vertex.graph.simplify(),
            attributes: vertex.attributes,
        },
        fragment,
        modify_attributes,
    );
    let graph = graph.simplify();

    let mut exprs = ExprCache::default();

    let mut output_dependencies = IndexMap::default();

    for output_name in frag_attributes.output_locations.left_values() {
        for c in "xyzw".chars() {
            let dependent_lines = graph.dependencies_recursive(output_name, Some(c), None);

            let last_node_index = dependent_lines.last().unwrap();
            let last_node = graph.nodes.get(*last_node_index).unwrap();
            let expr = &graph.exprs[last_node.input];

            let value = output_expr(expr, &graph, &mut exprs);
            output_dependencies.insert(format!("{output_name}.{c}").into(), value);
        }
    }

    let exprs = exprs.into_exprs();

    // Merge XYZ channels during database creation to simplify consuming code.
    let mut exprs_xyz = ExprCacheXyz::default();
    let mut output_dependencies_xyz = IndexMap::default();

    for output_name in frag_attributes.output_locations.left_values() {
        if let (Some(x), Some(y), Some(z)) = (
            output_dependencies.get(&SmolStr::from(&format!("{output_name}.x"))),
            output_dependencies.get(&SmolStr::from(&format!("{output_name}.y"))),
            output_dependencies.get(&SmolStr::from(&format!("{output_name}.z"))),
        ) && let Some(xyz) = merge_xyz_exprs(*x, *y, *z, &exprs, &mut exprs_xyz)
        {
            output_dependencies_xyz.insert(format!("{output_name}.xyz").into(), xyz);
        }
    }

    ShaderProgram {
        output_dependencies,
        exprs,
        output_dependencies_xyz,
        exprs_xyz: exprs_xyz.into_exprs(),
    }
}

pub fn convert_expr(e: OutputExpr<Operation>) -> OutputExpr<sm4sh_model::database::Operation> {
    match e {
        OutputExpr::Value(value) => OutputExpr::Value(value),
        OutputExpr::Func { op, args } => OutputExpr::Func {
            op: op.into(),
            args,
        },
    }
}

impl From<Operation> for sm4sh_model::database::Operation {
    fn from(value: Operation) -> Self {
        match value {
            Operation::Unk => Self::Unk,
            Operation::Add => Self::Add,
            Operation::Sub => Self::Sub,
            Operation::Mul => Self::Mul,
            Operation::Div => Self::Div,
            Operation::Mix => Self::Mix,
            Operation::Clamp => Self::Clamp,
            Operation::Min => Self::Min,
            Operation::Max => Self::Max,
            Operation::Abs => Self::Abs,
            Operation::Floor => Self::Floor,
            Operation::Power => Self::Power,
            Operation::Sqrt => Self::Sqrt,
            Operation::InverseSqrt => Self::InverseSqrt,
            Operation::Fma => Self::Fma,
            Operation::Dot => Self::Dot,
            Operation::Sin => Self::Sin,
            Operation::Cos => Self::Cos,
            Operation::Exp2 => Self::Exp2,
            Operation::Log2 => Self::Log2,
            Operation::Fract => Self::Fract,
            Operation::FloatBitsToInt => Self::FloatBitsToInt,
            Operation::IntBitsToFloat => Self::IntBitsToFloat,
            Operation::Select => Self::Select,
            Operation::Negate => Self::Negate,
            Operation::Equal => Self::Equal,
            Operation::NotEqual => Self::NotEqual,
            Operation::Less => Self::Less,
            Operation::Greater => Self::Greater,
            Operation::LessEqual => Self::LessEqual,
            Operation::GreaterEqual => Self::GreaterEqual,
            Operation::NormalMapX => Self::NormalMapX,
            Operation::NormalMapY => Self::NormalMapY,
            Operation::NormalMapZ => Self::NormalMapZ,
            Operation::NormalizeX => Self::NormalizeX,
            Operation::NormalizeY => Self::NormalizeY,
            Operation::NormalizeZ => Self::NormalizeZ,
            Operation::SphereMapCoordX => Self::SphereMapCoordX,
            Operation::SphereMapCoordY => Self::SphereMapCoordY,
            Operation::LocalToWorldPointX => Self::LocalToWorldPointX,
            Operation::LocalToWorldPointY => Self::LocalToWorldPointY,
            Operation::LocalToWorldPointZ => Self::LocalToWorldPointZ,
            Operation::LocalToWorldVectorX => Self::LocalToWorldVectorX,
            Operation::LocalToWorldVectorY => Self::LocalToWorldVectorY,
            Operation::LocalToWorldVectorZ => Self::LocalToWorldVectorZ,
            Operation::VarianceShadow => Self::VarianceShadow,
            Operation::BlinnPhongSpecular => Self::BlinnPhongSpecular,
            Operation::AnisotropicSpecular => Self::AnisotropicSpecular,
            Operation::Fresnel => Self::Fresnel,
            Operation::TintColorX => Self::TintColorX,
            Operation::TintColorY => Self::TintColorY,
            Operation::TintColorZ => Self::TintColorZ,
        }
    }
}

fn modify_attributes(graph: &Graph, expr: &Expr) -> Expr {
    // Remove attribute transforms so queries can detect attribute channels.
    // TODO: keep track of what space each attribute is in like model, view, etc.
    // TODO: replace with functions that transform to a specific space like world to view?
    if let Some(new_expr) = local_to_world_normal(graph, expr)
        .cloned()
        .or_else(|| local_to_world_binormal(graph, expr))
    {
        new_expr
    } else if let Some(new_expr) = eye_vector(graph, expr)
        .or_else(|| light_position(graph, expr))
        .or_else(|| light_map_position(graph, expr))
    {
        new_expr
    } else {
        expr.clone()
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
pub enum OperationXyz {
    #[default]
    Unk,
    Add,
    Sub,
    Mul,
    Div,
    Mix,
    Clamp,
    Min,
    Max,
    Abs,
    Floor,
    Power,
    Sqrt,
    InverseSqrt,
    Fma,
    Dot,
    Sin,
    Cos,
    Exp2,
    Log2,
    Fract,
    IntBitsToFloat,
    FloatBitsToInt,
    Select,
    Negate,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    NormalMapX,
    NormalMapY,
    NormalMapZ,
    Normalize,
    SphereMapCoordX,
    SphereMapCoordY,
    LocalToWorldPoint,
    LocalToWorldVector,
    VarianceShadow,
    BlinnPhongSpecular,
    AnisotropicSpecular,
    Fresnel,
    TintColor,
}

impl std::fmt::Display for OperationXyz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl OperationXyzChannel for Operation {
    type OperationXyz = OperationXyz;

    fn operation_xyz_channel(&self) -> Option<(Self::OperationXyz, Option<char>)> {
        match self {
            Operation::Unk => Some((OperationXyz::Unk, None)),
            Operation::Add => Some((OperationXyz::Add, None)),
            Operation::Sub => Some((OperationXyz::Sub, None)),
            Operation::Mul => Some((OperationXyz::Mul, None)),
            Operation::Div => Some((OperationXyz::Div, None)),
            Operation::Mix => Some((OperationXyz::Mix, None)),
            Operation::Clamp => Some((OperationXyz::Clamp, None)),
            Operation::Min => Some((OperationXyz::Min, None)),
            Operation::Max => Some((OperationXyz::Max, None)),
            Operation::Abs => Some((OperationXyz::Abs, None)),
            Operation::Floor => Some((OperationXyz::Floor, None)),
            Operation::Power => Some((OperationXyz::Power, None)),
            Operation::Sqrt => Some((OperationXyz::Sqrt, None)),
            Operation::InverseSqrt => Some((OperationXyz::InverseSqrt, None)),
            Operation::Fma => Some((OperationXyz::Fma, None)),
            Operation::Dot => Some((OperationXyz::Dot, None)),
            Operation::Sin => Some((OperationXyz::Sin, None)),
            Operation::Cos => Some((OperationXyz::Cos, None)),
            Operation::Exp2 => Some((OperationXyz::Exp2, None)),
            Operation::Log2 => Some((OperationXyz::Log2, None)),
            Operation::Fract => Some((OperationXyz::Fract, None)),
            Operation::IntBitsToFloat => Some((OperationXyz::IntBitsToFloat, None)),
            Operation::FloatBitsToInt => Some((OperationXyz::FloatBitsToInt, None)),
            Operation::Select => Some((OperationXyz::Select, None)),
            Operation::Negate => Some((OperationXyz::Negate, None)),
            Operation::Equal => Some((OperationXyz::Equal, None)),
            Operation::NotEqual => Some((OperationXyz::NotEqual, None)),
            Operation::Less => Some((OperationXyz::Less, None)),
            Operation::Greater => Some((OperationXyz::Greater, None)),
            Operation::LessEqual => Some((OperationXyz::LessEqual, None)),
            Operation::GreaterEqual => Some((OperationXyz::GreaterEqual, None)),
            Operation::NormalMapX => Some((OperationXyz::NormalMapX, None)),
            Operation::NormalMapY => Some((OperationXyz::NormalMapY, None)),
            Operation::NormalMapZ => Some((OperationXyz::NormalMapZ, None)),
            Operation::NormalizeX => Some((OperationXyz::Normalize, Some('x'))),
            Operation::NormalizeY => Some((OperationXyz::Normalize, Some('y'))),
            Operation::NormalizeZ => Some((OperationXyz::Normalize, Some('z'))),
            Operation::SphereMapCoordX => Some((OperationXyz::SphereMapCoordX, None)),
            Operation::SphereMapCoordY => Some((OperationXyz::SphereMapCoordY, None)),
            Operation::LocalToWorldPointX => Some((OperationXyz::LocalToWorldPoint, Some('x'))),
            Operation::LocalToWorldPointY => Some((OperationXyz::LocalToWorldPoint, Some('y'))),
            Operation::LocalToWorldPointZ => Some((OperationXyz::LocalToWorldPoint, Some('z'))),
            Operation::LocalToWorldVectorX => Some((OperationXyz::LocalToWorldVector, Some('x'))),
            Operation::LocalToWorldVectorY => Some((OperationXyz::LocalToWorldVector, Some('y'))),
            Operation::LocalToWorldVectorZ => Some((OperationXyz::LocalToWorldVector, Some('z'))),
            Operation::VarianceShadow => Some((OperationXyz::VarianceShadow, None)),
            Operation::BlinnPhongSpecular => Some((OperationXyz::BlinnPhongSpecular, None)),
            Operation::AnisotropicSpecular => Some((OperationXyz::AnisotropicSpecular, None)),
            Operation::Fresnel => Some((OperationXyz::Fresnel, None)),
            Operation::TintColorX => Some((OperationXyz::TintColor, Some('x'))),
            Operation::TintColorY => Some((OperationXyz::TintColor, Some('y'))),
            Operation::TintColorZ => Some((OperationXyz::TintColor, Some('z'))),
        }
    }
}

impl MergeXyzArgs<Operation> for OperationXyz {
    fn merge_xyz_args(
        &self,
        args_x: &[usize],
        args_y: &[usize],
        args_z: &[usize],
        exprs: &[OutputExpr<Operation>],
        exprs_xyz: &mut ExprCacheXyz<Self>,
    ) -> Option<Vec<usize>> {
        let mut args = Vec::new();

        for ((x, y), z) in args_x.iter().zip(args_y.iter()).zip(args_z.iter()) {
            let arg = merge_xyz_exprs(*x, *y, *z, exprs, exprs_xyz)?;
            args.push(arg);
        }

        Some(args)
    }
}
