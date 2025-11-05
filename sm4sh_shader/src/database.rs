use std::borrow::Cow;

use glsl_lang::ast::TranslationUnit;
use log::error;
use smol_str::SmolStr;
use xc3_shader::{
    expr::{OutputExpr, output_expr},
    graph::{
        BinaryOp, Expr, Graph, UnaryOp,
        glsl::{find_attribute_locations, merge_vertex_fragment},
    },
};

mod query;
use query::*;

// Faster than the default hash implementation.
type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;
type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

#[derive(Debug, PartialEq, Clone)]
pub struct ShaderProgram {
    /// Indices into [exprs](#structfield.exprs) for values assigned to a fragment output.
    pub output_dependencies: IndexMap<SmolStr, usize>,

    /// Unique exprs used for this program.
    pub exprs: Vec<OutputExpr<Operation>>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Operation {
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
    Dot4,
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
    Unk,
}

impl Default for Operation {
    fn default() -> Self {
        Self::Unk
    }
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl xc3_shader::expr::Operation for Operation {
    fn query_operation_args<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Self, Vec<&'a Expr>)> {
        // TODO: Share these queries with xc3_shader?
        // TODO: Use queries to simplify operations
        // TODO: Figure out why op_mix doesn't work with simplification.
        // TODO: query for view vector
        op_normal_map(graph, expr)
            // .or_else(|| op_mix(graph, expr))
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
                error!("Unsuported expression {expr:?}");
                None
            })
    }

    fn preprocess_expr<'a>(_graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr> {
        Cow::Borrowed(expr)
    }

    fn preprocess_value_expr<'a>(_graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr> {
        Cow::Borrowed(expr)
    }
}

pub fn shader_from_glsl(vertex: &TranslationUnit, fragment: &TranslationUnit) -> ShaderProgram {
    let frag = Graph::from_glsl(fragment);
    let frag_attributes = find_attribute_locations(fragment);

    let vert = Graph::from_glsl(vertex);
    let vert_attributes = find_attribute_locations(vertex);

    // Create a combined graph that links vertex outputs to fragment inputs.
    // This effectively moves all shader logic to the fragment shader.
    // This simplifies generating shader code or material nodes in 3D applications.
    let graph = merge_vertex_fragment(
        vert.simplify(),
        &vert_attributes,
        frag,
        &frag_attributes,
        modify_attributes,
    );
    let graph = graph.simplify();

    let mut exprs = IndexSet::default();
    let mut expr_to_index = IndexMap::default();
    let mut output_dependencies = IndexMap::default();
    for output_name in frag_attributes.output_locations.left_values() {
        for c in "xyzw".chars() {
            let dependent_lines = graph.dependencies_recursive(output_name, Some(c), None);

            let last_node_index = dependent_lines.last().unwrap();
            let last_node = graph.nodes.get(*last_node_index).unwrap();
            let expr = &graph.exprs[last_node.input];

            let value = output_expr(expr, &graph, &mut exprs, &mut expr_to_index);
            output_dependencies.insert(format!("{output_name}.{c}").into(), value);
        }
    }

    ShaderProgram {
        output_dependencies,
        exprs: exprs.into_iter().collect(),
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
            Operation::Dot4 => Self::Dot4,
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
            Operation::Unk => Self::Unk,
        }
    }
}

fn modify_attributes(graph: &Graph, expr: &Expr) -> Expr {
    // Remove attribute transforms so queries can detect attribute channels.
    // TODO: keep track of what space each attribute is in like model, view, etc.
    // TODO: replace with functions that transform to a specific space like world to view?
    if let Some(new_expr) =
        local_to_world_normal(graph, expr).or_else(|| local_to_world_binormal(graph, expr))
    {
        new_expr.clone()
    } else if let Some(new_expr) = eye_vector(graph, expr) {
        new_expr
    } else {
        expr.clone()
    }
}
