use std::borrow::Cow;

use log::error;
use smol_str::{SmolStr, format_smolstr};
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
pub struct Operation(sm4sh_model::database::Operation);

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl xc3_shader::expr::Operation for Operation {
    fn query_operation_args<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Self, Vec<&'a Expr>)> {
        use sm4sh_model::database::Operation as Op;
        op_normal_map(graph, expr)
            .or_else(|| op_effect_light(graph, expr))
            .or_else(|| op_blinn_phong_spec(graph, expr))
            .or_else(|| op_blinn_phong_spec_anisotropic(graph, expr))
            .or_else(|| op_fresnel(graph, expr))
            .or_else(|| op_variance_shadow(graph, expr))
            .or_else(|| op_sphere_map_coords(graph, expr))
            .or_else(|| op_local_to_world_point(graph, expr))
            .or_else(|| op_local_to_world_vector(graph, expr))
            .or_else(|| op_tint_color(graph, expr))
            .or_else(|| op_normalize(graph, expr))
            .or_else(|| op_neg_reflect(graph, expr))
            .or_else(|| op_pow(graph, expr))
            .or_else(|| op_sqrt(graph, expr))
            .or_else(|| op_dot3(graph, expr))
            .or_else(|| op_dot4(graph, expr))
            .or_else(|| op_div(graph, expr))
            .or_else(|| ternary(graph, expr))
            .or_else(|| binary_op(graph, expr, BinaryOp::Add, Op::Add.into()))
            .or_else(|| binary_op(graph, expr, BinaryOp::Sub, Op::Sub.into()))
            .or_else(|| binary_op(graph, expr, BinaryOp::Mul, Op::Mul.into()))
            .or_else(|| unary_op(graph, expr, UnaryOp::Negate, Op::Negate.into()))
            .or_else(|| op_func(graph, expr, "clamp", Op::Clamp.into()))
            .or_else(|| op_func(graph, expr, "min", Op::Min.into()))
            .or_else(|| op_func(graph, expr, "max", Op::Max.into()))
            .or_else(|| op_func(graph, expr, "inversesqrt", Op::InverseSqrt.into()))
            .or_else(|| op_func(graph, expr, "abs", Op::Abs.into()))
            .or_else(|| op_func(graph, expr, "floor", Op::Floor.into()))
            .or_else(|| op_func(graph, expr, "fma", Op::Fma.into()))
            .or_else(|| op_func(graph, expr, "sin", Op::Sin.into()))
            .or_else(|| op_func(graph, expr, "cos", Op::Cos.into()))
            .or_else(|| op_func(graph, expr, "log2", Op::Log2.into()))
            .or_else(|| op_func(graph, expr, "exp2", Op::Exp2.into()))
            .or_else(|| op_func(graph, expr, "fract", Op::Fract.into()))
            .or_else(|| op_func(graph, expr, "intBitsToFloat", Op::IntBitsToFloat.into()))
            .or_else(|| op_func(graph, expr, "floatBitsToInt", Op::FloatBitsToInt.into()))
            .or_else(|| binary_op(graph, expr, BinaryOp::Equal, Op::Equal.into()))
            .or_else(|| binary_op(graph, expr, BinaryOp::NotEqual, Op::NotEqual.into()))
            .or_else(|| binary_op(graph, expr, BinaryOp::Greater, Op::Greater.into()))
            .or_else(|| binary_op(graph, expr, BinaryOp::GreaterEqual, Op::GreaterEqual.into()))
            .or_else(|| binary_op(graph, expr, BinaryOp::Less, Op::Less.into()))
            .or_else(|| binary_op(graph, expr, BinaryOp::LessEqual, Op::LessEqual.into()))
            .or_else(|| {
                error!("Unsupported expression {expr:?}");
                None
            })
    }

    fn preprocess_expr<'a>(graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr> {
        // Remove attribute transforms so queries can detect attribute channels.
        // TODO: keep track of what space each attribute is in like model, view, etc.
        // TODO: replace with functions that transform to a specific space like world to view?
        if let Some(new_expr) = latte_texture_cube_coords(graph, expr)
            .or_else(|| fragment_tangent(graph, expr))
            .or_else(|| eye_vector(graph, expr))
            .or_else(|| light_position(graph, expr))
            .or_else(|| light_map_position(graph, expr))
            .or_else(|| local_to_world_binormal(graph, expr))
        {
            Cow::Owned(new_expr)
        } else if let Some(new_expr) = local_to_world_normal(graph, expr) {
            Cow::Borrowed(new_expr)
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
    let graph = merge_vertex_fragment(&vertex, &fragment);
    let graph = graph.simplify();

    let mut exprs = ExprCache::default();

    let mut output_dependencies = IndexMap::default();

    for output_name in frag_attributes.output_locations.left_values() {
        for c in "xyzw".chars() {
            // Find the most recent assignment for the output variable.
            let node = graph
                .nodes
                .iter()
                .rfind(|n| &n.output.name == output_name && n.output.channel == Some(c))
                .unwrap();
            let expr = &graph.exprs[node.input];

            let value = output_expr(expr, &graph, &mut exprs);
            output_dependencies.insert(format_smolstr!("{output_name}.{c}"), value);
        }
    }

    let exprs = exprs.into_exprs();

    // Merge XYZ channels during database creation to simplify consuming code.
    let mut exprs_xyz = ExprCacheXyz::default();
    let mut output_dependencies_xyz = IndexMap::default();

    for output_name in frag_attributes.output_locations.left_values() {
        if let (Some(x), Some(y), Some(z)) = (
            output_dependencies.get(&format_smolstr!("{output_name}.x")),
            output_dependencies.get(&format_smolstr!("{output_name}.y")),
            output_dependencies.get(&format_smolstr!("{output_name}.z")),
        ) && let Some(xyz) = merge_xyz_exprs(*x, *y, *z, &exprs, &mut exprs_xyz)
        {
            output_dependencies_xyz.insert(format_smolstr!("{output_name}.xyz"), xyz);
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

pub fn convert_expr_xyz(
    e: OutputExprXyz<OperationXyz>,
) -> OutputExprXyz<sm4sh_model::database::OperationXyz> {
    match e {
        OutputExprXyz::Value(value) => OutputExprXyz::Value(value),
        OutputExprXyz::Func { op, args, channel } => OutputExprXyz::Func {
            op: op.into(),
            args,
            channel,
        },
    }
}

impl From<Operation> for sm4sh_model::database::Operation {
    fn from(value: Operation) -> Self {
        value.0
    }
}

impl From<sm4sh_model::database::Operation> for Operation {
    fn from(value: sm4sh_model::database::Operation) -> Self {
        Self(value)
    }
}

impl From<OperationXyz> for sm4sh_model::database::OperationXyz {
    fn from(value: OperationXyz) -> Self {
        value.0
    }
}

impl From<sm4sh_model::database::OperationXyz> for OperationXyz {
    fn from(value: sm4sh_model::database::OperationXyz) -> Self {
        Self(value)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
pub struct OperationXyz(sm4sh_model::database::OperationXyz);

impl std::fmt::Display for OperationXyz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl OperationXyzChannel for Operation {
    type OperationXyz = OperationXyz;

    fn operation_xyz_channel(&self) -> Option<(Self::OperationXyz, Option<char>)> {
        use sm4sh_model::database::Operation as Op;
        use sm4sh_model::database::OperationXyz as OpXyz;

        match self.0 {
            Op::Unk => Some((OpXyz::Unk.into(), None)),
            Op::Add => Some((OpXyz::Add.into(), None)),
            Op::Sub => Some((OpXyz::Sub.into(), None)),
            Op::Mul => Some((OpXyz::Mul.into(), None)),
            Op::Div => Some((OpXyz::Div.into(), None)),
            Op::Mix => Some((OpXyz::Mix.into(), None)),
            Op::Clamp => Some((OpXyz::Clamp.into(), None)),
            Op::Min => Some((OpXyz::Min.into(), None)),
            Op::Max => Some((OpXyz::Max.into(), None)),
            Op::Abs => Some((OpXyz::Abs.into(), None)),
            Op::Floor => Some((OpXyz::Floor.into(), None)),
            Op::Power => Some((OpXyz::Power.into(), None)),
            Op::Sqrt => Some((OpXyz::Sqrt.into(), None)),
            Op::InverseSqrt => Some((OpXyz::InverseSqrt.into(), None)),
            Op::Fma => Some((OpXyz::Fma.into(), None)),
            Op::Dot3 => Some((OpXyz::Dot.into(), None)),
            Op::Dot4 => None, // TODO: support dot4?
            Op::Sin => Some((OpXyz::Sin.into(), None)),
            Op::Cos => Some((OpXyz::Cos.into(), None)),
            Op::Exp2 => Some((OpXyz::Exp2.into(), None)),
            Op::Log2 => Some((OpXyz::Log2.into(), None)),
            Op::Fract => Some((OpXyz::Fract.into(), None)),
            Op::IntBitsToFloat => Some((OpXyz::IntBitsToFloat.into(), None)),
            Op::FloatBitsToInt => Some((OpXyz::FloatBitsToInt.into(), None)),
            Op::Select => Some((OpXyz::Select.into(), None)),
            Op::Negate => Some((OpXyz::Negate.into(), None)),
            Op::Equal => Some((OpXyz::Equal.into(), None)),
            Op::NotEqual => Some((OpXyz::NotEqual.into(), None)),
            Op::Less => Some((OpXyz::Less.into(), None)),
            Op::Greater => Some((OpXyz::Greater.into(), None)),
            Op::LessEqual => Some((OpXyz::LessEqual.into(), None)),
            Op::GreaterEqual => Some((OpXyz::GreaterEqual.into(), None)),
            Op::NormalMapX => Some((OpXyz::NormalMap.into(), Some('x'))),
            Op::NormalMapY => Some((OpXyz::NormalMap.into(), Some('y'))),
            Op::NormalMapZ => Some((OpXyz::NormalMap.into(), Some('z'))),
            Op::NormalizeX => Some((OpXyz::Normalize.into(), Some('x'))),
            Op::NormalizeY => Some((OpXyz::Normalize.into(), Some('y'))),
            Op::NormalizeZ => Some((OpXyz::Normalize.into(), Some('z'))),
            Op::SphereMapCoordX => None,
            Op::SphereMapCoordY => None,
            Op::LocalToWorldPointX => Some((OpXyz::LocalToWorldPoint.into(), Some('x'))),
            Op::LocalToWorldPointY => Some((OpXyz::LocalToWorldPoint.into(), Some('y'))),
            Op::LocalToWorldPointZ => Some((OpXyz::LocalToWorldPoint.into(), Some('z'))),
            Op::LocalToWorldVectorX => Some((OpXyz::LocalToWorldVector.into(), Some('x'))),
            Op::LocalToWorldVectorY => Some((OpXyz::LocalToWorldVector.into(), Some('y'))),
            Op::LocalToWorldVectorZ => Some((OpXyz::LocalToWorldVector.into(), Some('z'))),
            Op::VarianceShadow => Some((OpXyz::VarianceShadow.into(), None)),
            Op::BlinnPhongSpecular => Some((OpXyz::BlinnPhongSpecular.into(), None)),
            Op::AnisotropicSpecular => Some((OpXyz::AnisotropicSpecular.into(), None)),
            Op::Fresnel => Some((OpXyz::Fresnel.into(), None)),
            Op::TintColorX => Some((OpXyz::TintColor.into(), Some('x'))),
            Op::TintColorY => Some((OpXyz::TintColor.into(), Some('y'))),
            Op::TintColorZ => Some((OpXyz::TintColor.into(), Some('z'))),
            Op::NegReflectX => Some((OpXyz::NegReflect.into(), Some('x'))),
            Op::NegReflectY => Some((OpXyz::NegReflect.into(), Some('y'))),
            Op::NegReflectZ => Some((OpXyz::NegReflect.into(), Some('z'))),
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
        use sm4sh_model::database::OperationXyz as OpXyz;
        match self.0 {
            OpXyz::Normalize => {
                // TODO: Check that all args are the same.
                let arg = merge_xyz_exprs(args_x[0], args_x[1], args_x[2], exprs, exprs_xyz)?;
                Some(vec![arg])
            }
            OpXyz::Dot => {
                // TODO: Check that all args are the same.
                let a = merge_xyz_exprs(args_x[0], args_x[1], args_x[2], exprs, exprs_xyz)?;
                let b = merge_xyz_exprs(args_x[3], args_x[4], args_x[5], exprs, exprs_xyz)?;

                Some(vec![a, b])
            }
            OpXyz::LocalToWorldPoint => {
                // TODO: Check that all args are the same.
                let arg = merge_xyz_exprs(args_x[0], args_x[1], args_x[2], exprs, exprs_xyz)?;
                Some(vec![arg])
            }
            OpXyz::LocalToWorldVector => {
                // TODO: Check that all args are the same.
                let arg = merge_xyz_exprs(args_x[0], args_x[1], args_x[2], exprs, exprs_xyz)?;
                Some(vec![arg])
            }
            OpXyz::NormalMap => {
                // TODO: Check that all args are the same.
                let arg = merge_xyz_exprs(args_x[0], args_y[1], args_z[2], exprs, exprs_xyz)?;
                Some(vec![arg])
            }
            OpXyz::BlinnPhongSpecular => {
                // TODO: Check that all args are the same.
                let normal = merge_xyz_exprs(args_x[0], args_x[1], args_x[2], exprs, exprs_xyz)?;
                let light_dir = merge_xyz_exprs(args_x[3], args_x[4], args_x[5], exprs, exprs_xyz)?;
                let eye = merge_xyz_exprs(args_x[6], args_x[7], args_x[8], exprs, exprs_xyz)?;

                let exponent = merge_xyz_exprs(args_x[9], args_y[9], args_z[9], exprs, exprs_xyz)?;

                Some(vec![normal, light_dir, eye, exponent])
            }
            OpXyz::AnisotropicSpecular => {
                // TODO: Check that all args are the same.
                let normal = merge_xyz_exprs(args_x[0], args_x[1], args_x[2], exprs, exprs_xyz)?;
                let tangent = merge_xyz_exprs(args_x[3], args_x[4], args_x[5], exprs, exprs_xyz)?;
                let eye = merge_xyz_exprs(args_x[6], args_x[7], args_x[8], exprs, exprs_xyz)?;

                let param_x = merge_xyz_exprs(args_x[9], args_y[9], args_z[9], exprs, exprs_xyz)?;
                let param_y =
                    merge_xyz_exprs(args_x[10], args_y[10], args_z[10], exprs, exprs_xyz)?;

                Some(vec![normal, tangent, eye, param_x, param_y])
            }
            OpXyz::Fresnel => {
                // TODO: Check that all args are the same.
                let normal = merge_xyz_exprs(args_x[0], args_x[1], args_x[2], exprs, exprs_xyz)?;
                let eye = merge_xyz_exprs(args_x[3], args_x[4], args_x[5], exprs, exprs_xyz)?;

                let param = merge_xyz_exprs(args_x[6], args_y[6], args_z[6], exprs, exprs_xyz)?;

                Some(vec![normal, eye, param])
            }
            OpXyz::TintColor => {
                // TODO: Check that all args are the same.
                let rgb = merge_xyz_exprs(args_x[0], args_x[1], args_x[2], exprs, exprs_xyz)?;

                let param = merge_xyz_exprs(args_x[3], args_y[3], args_z[3], exprs, exprs_xyz)?;

                Some(vec![rgb, param])
            }
            OpXyz::NegReflect => {
                // TODO: Check that all args are the same.
                let eye = merge_xyz_exprs(args_x[0], args_x[1], args_x[2], exprs, exprs_xyz)?;
                let normal = merge_xyz_exprs(args_x[3], args_x[4], args_x[5], exprs, exprs_xyz)?;

                Some(vec![eye, normal])
            }
            _ => {
                let mut args = Vec::new();

                for ((x, y), z) in args_x.iter().zip(args_y.iter()).zip(args_z.iter()) {
                    let arg = merge_xyz_exprs(*x, *y, *z, exprs, exprs_xyz)?;
                    args.push(arg);
                }

                Some(args)
            }
        }
    }
}
