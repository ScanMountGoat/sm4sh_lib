use std::borrow::Cow;

use glsl_lang::ast::TranslationUnit;
use indexmap::{IndexMap, IndexSet};
use smol_str::SmolStr;
use xc3_shader::{
    expr::{OutputExpr, output_expr},
    graph::{
        BinaryOp, Expr, Graph, UnaryOp,
        glsl::{find_attribute_locations, merge_vertex_fragment},
        query::assign_x_recursive,
    },
};

mod query;
use query::*;

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
    Select,
    Negate,
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
        op_mix(graph, expr)
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
    }

    fn preprocess_expr<'a>(graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr> {
        Cow::Borrowed(assign_x_recursive(graph, expr))
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
    let graph = merge_vertex_fragment(vert, &vert_attributes, frag, &frag_attributes);

    let mut exprs = IndexSet::new();
    let mut expr_to_index = IndexMap::new();
    let mut output_dependencies = IndexMap::new();
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
            Operation::Select => Self::Select,
            Operation::Negate => Self::Negate,
            Operation::Unk => Self::Unk,
        }
    }
}
