use std::sync::LazyLock;

use indoc::indoc;
use xc3_shader::graph::{BinaryOp, Expr, Graph, UnaryOp, query::query_nodes};

use crate::database::Operation;

pub fn op_func<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
    func: &str,
    op: Operation,
) -> Option<(Operation, Vec<&'a Expr>)> {
    match expr {
        Expr::Func { name, args, .. } => {
            if name == func {
                Some((op, args.iter().map(|a| &graph.exprs[*a]).collect()))
            } else {
                None
            }
        }
        _ => None,
    }
}

static OP_NORMAL_MAP_X: LazyLock<Graph> = LazyLock::new(|| {
    // TBN matrix from texas_cross.105.frag.
    // TODO: Check attribute channels and/or cross product to differentiate xyz.
    // TODO: tangent.xyz = cross(bitangent.xyz, normal.xyz) * bitangent.w
    // TODO: Attribute channels require eliminating transforms from vertex shader.
    let query = indoc! {"
        void main() {
            normal_map_x = normal_map_x + -0.00196078;
            normal_map_x = normal_map_x * 2.0;
            normal_map_x = normal_map_x + -1.0;

            normal_map_y = normal_map_y + -0.00196078;
            normal_map_y = normal_map_y * 2.0;
            normal_map_y = normal_map_y + -1.0;

            normal_map_z = normal_map.z + -0.00196078;
            normal_map_z = normal_map_z * 2.0;
            normal_map_z = normal_map_z + -1.0;

            // bitangent_w = bitangent.w;
            tangent_x = bitangent_w * tangent_x;

            // bitangent_x = bitangent.x;
            inverse_length_bitangent = inversesqrt(bitangent_length);
            normalize_bitangent_x = bitangent_x * inverse_length_bitangent;

            // normal_x = normal.x;
            inverse_length_normal = inversesqrt(normal_length);
            normalize_normal_x = normal_x * inverse_length_normal;

            result_x = normal_map_x * tangent_x;
            result_y = fma(normal_map_y, normalize_bitangent_x, result_x);
            result = fma(normal_map_z, normalize_normal_x, result_y);

            inverse_length_result = inversesqrt(result_length);
            result = result * inverse_length_result;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_normal_map<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_NORMAL_MAP_X)?;
    let x = result.get("normal_map_x")?;
    let y = result.get("normal_map_y")?;
    let z = result.get("normal_map_z")?;
    Some((Operation::NormalMapX, vec![x, y, z]))
}

static OP_MIX: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            neg_a = 0.0 - a;
            b_minus_a = neg_a + b;
            result = fma(b_minus_a, ratio, a);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static OP_MIX2: LazyLock<Graph> = LazyLock::new(|| {
    // Alternative form used for some shaders.
    let query = indoc! {"
        void main() {
            neg_ratio = 0.0 - ratio;
            a_inv_ratio = fma(a, neg_ratio, a);
            result = fma(b, ratio, a_inv_ratio);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_mix<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // TODO: This is matching things it shouldn't but only when simplified?
    let result =
        query_nodes(expr, graph, &OP_MIX).or_else(|| query_nodes(expr, graph, &OP_MIX2))?;
    print!("{}", OP_MIX.simplify().to_glsl());
    print!("{}", OP_MIX2.simplify().to_glsl());
    let a = result.get("a")?;
    let b = result.get("b")?;
    let ratio = result.get("ratio")?;
    Some((Operation::Mix, vec![a, b, ratio]))
}

static OP_POW: LazyLock<Graph> = LazyLock::new(|| {
    // Equivalent to pow(a, b)
    let query = indoc! {"
        void main() {
            a = abs(a);
            a = log2(a);
            a = a * b;
            a = exp2(a);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static OP_POW2: LazyLock<Graph> = LazyLock::new(|| {
    // Equivalent to pow(a, b)
    let query = indoc! {"
        void main() {
            a = log2(a);
            a = a * b;
            a = exp2(a);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_pow<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result =
        query_nodes(expr, graph, &OP_POW).or_else(|| query_nodes(expr, graph, &OP_POW2))?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Power, vec![a, b]))
}

static OP_SQRT: LazyLock<Graph> = LazyLock::new(|| {
    // Equivalent to sqrt(result)
    let query = indoc! {"
        void main() {
            result = inversesqrt(result);
            result = 1.0 / result;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static OP_SQRT2: LazyLock<Graph> = LazyLock::new(|| {
    Graph::parse_glsl("void main() { result = sqrt(result); }")
        .unwrap()
        .simplify()
});

pub fn op_sqrt<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result =
        query_nodes(expr, graph, &OP_SQRT).or_else(|| query_nodes(expr, graph, &OP_SQRT2))?;
    let result = result.get("result")?;
    Some((Operation::Sqrt, vec![result]))
}

static OP_DOT4: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            result = dot(vec4(ax, ay, az, aw), vec4(bx, by, bz, bw));
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_dot<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_DOT4)?;

    let ax = result.get("ax")?;
    let ay = result.get("ay")?;
    let az = result.get("az")?;
    let aw = result.get("aw")?;

    let bx = result.get("bx")?;
    let by = result.get("by")?;
    let bz = result.get("bz")?;
    let bw = result.get("bw")?;

    Some((Operation::Dot4, vec![ax, ay, az, aw, bx, by, bz, bw]))
}

pub fn ternary<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    if let Expr::Ternary(cond, a, b) = expr {
        Some((
            Operation::Select,
            vec![&graph.exprs[*cond], &graph.exprs[*a], &graph.exprs[*b]],
        ))
    } else {
        None
    }
}

static OP_DIV: LazyLock<Graph> = LazyLock::new(|| {
    Graph::parse_glsl("void main() { result = a / b; }")
        .unwrap()
        .simplify()
});

static OP_DIV2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
            void main() {
                one_over_b = 1.0 / b;
                result = a * one_over_b;
            }
        "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_div<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result =
        query_nodes(expr, graph, &OP_DIV).or_else(|| query_nodes(expr, graph, &OP_DIV2))?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Div, vec![a, b]))
}

pub fn binary_op<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
    binary_op: BinaryOp,
    operation: Operation,
) -> Option<(Operation, Vec<&'a Expr>)> {
    if let Expr::Binary(op, a0, a1) = expr
        && *op == binary_op
    {
        Some((operation, vec![&graph.exprs[*a0], &graph.exprs[*a1]]))
    } else {
        None
    }
}

pub fn unary_op<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
    unary_op: UnaryOp,
    operation: Operation,
) -> Option<(Operation, Vec<&'a Expr>)> {
    if let Expr::Unary(op, a) = expr
        && *op == unary_op
    {
        Some((operation, vec![&graph.exprs[*a]]))
    } else {
        None
    }
}
