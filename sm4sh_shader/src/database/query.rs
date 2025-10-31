use std::sync::LazyLock;

use indoc::{formatdoc, indoc};
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

fn op_normal_map_query(c: char) -> String {
    // TBN matrix from texas_cross.105.frag.
    // TODO: Also check attribute channels and/or cross product to differentiate xyz.
    // TODO: tangent.xyz = cross(bitangent.xyz, normal.xyz) * bitangent.w
    // TODO: binormal channels require eliminating transforms from vertex shader.
    formatdoc! {"
        void main() {{
            normal_map_x = normal_map_x + -0.00196078;
            normal_map_x = normal_map_x * 2.0;
            normal_map_x = normal_map_x + -1.0;

            normal_map_y = normal_map_y + -0.00196078;
            normal_map_y = normal_map_y * 2.0;
            normal_map_y = normal_map_y + -1.0;

            normal_map_z = normal_map_z + -0.00196078;
            normal_map_z = normal_map_z * 2.0;
            normal_map_z = normal_map_z + -1.0;

            // bitangent_w = bitangent.w;
            tangent = bitangent_w * tangent;

            // bitangent = bitangent.{c};
            inverse_length_bitangent = inversesqrt(bitangent_length);
            normalize_bitangent = bitangent * inverse_length_bitangent;

            normal = normal.{c};
            inverse_length_normal = inversesqrt(normal_length);
            normalize_normal = normal * inverse_length_normal;

            result_x = normal_map_x * tangent;
            result_y = fma(normal_map_y, normalize_bitangent, result_x);
            result = fma(normal_map_z, normalize_normal, result_y);

            inverse_length_result = inversesqrt(result_length);
            result = result * inverse_length_result;
        }}
    "}
}

static OP_NORMAL_MAP_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = op_normal_map_query('x');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static OP_NORMAL_MAP_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = op_normal_map_query('y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static OP_NORMAL_MAP_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = op_normal_map_query('z');
    Graph::parse_glsl(&query).unwrap().simplify()
});

pub fn op_normal_map<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let (op, result) = query_nodes(expr, graph, &OP_NORMAL_MAP_X)
        .map(|r| (Operation::NormalMapX, r))
        .or_else(|| query_nodes(expr, graph, &OP_NORMAL_MAP_Y).map(|r| (Operation::NormalMapY, r)))
        .or_else(|| {
            query_nodes(expr, graph, &OP_NORMAL_MAP_Z).map(|r| (Operation::NormalMapZ, r))
        })?;
    let x = result.get("normal_map_x")?;
    let y = result.get("normal_map_y")?;
    let z = result.get("normal_map_z")?;
    Some((op, vec![x, y, z]))
}

// TODO: Reduce repetition in queries?
static TRANSFORM_NORMAL_X: LazyLock<Graph> = LazyLock::new(|| {
    // texas_cross.105.vert.
    let query = indoc! {"
        void main() {
            R3.x = a_Normal_x;
            R3.y = a_Normal_y;
            R3.z = a_Normal_z;
            R7.y = R3.z * PerDraw.LocalToWorldMatrix[2].z;
            R126.w = R3.z * PerDraw.LocalToWorldMatrix[2].x;
            R126.x = fma(R3.y, PerDraw.LocalToWorldMatrix[1].x, R126.w);
            PS8 = R3.z * PerDraw.LocalToWorldMatrix[2].y;
            R126.z = fma(R3.y, PerDraw.LocalToWorldMatrix[1].y, PS8);
            R124.w = fma(R3.y, PerDraw.LocalToWorldMatrix[1].z, R7.y);
            R127.x = fma(R3.x, PerDraw.LocalToWorldMatrix[0].x, R126.x);
            R3_backup.x = R3.x;
            R126.y = fma(R3_backup.x, PerDraw.LocalToWorldMatrix[0].y, R126.z);
            PV12.y = R126.y;
            R127.z = fma(R3_backup.x, PerDraw.LocalToWorldMatrix[0].z, R124.w);
            PV12.z = R127.z;
            temp13 = dot(vec4(R127.x, PV12.y, PV12.z, 0.0), vec4(R127.x, PV12.y, PV12.z, 0.0));
            PV13.x = temp13;
            R127.y = inversesqrt(PV13.x);
            PS14 = R127.y;
            R126_backup.y = R126.y;
            R3.z = R127.z * PS14;
            R1.w = R126_backup.y * PS14;
            R4.y = R127.x * R127.y;
            R124.w = FB0.bgRotInv[2].x * R3.z;
            R124.w = fma(R1.w, FB0.bgRotInv[1].x, R124.w);
            R17.x = fma(R4.y, FB0.bgRotInv[0].x, R124.w);
            result = R17.x;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static TRANSFORM_NORMAL_Y: LazyLock<Graph> = LazyLock::new(|| {
    // texas_cross.105.vert.
    let query = indoc! {"
        void main() {
            R3.x = a_Normal_x;
            R3.y = a_Normal_y;
            R3.z = a_Normal_z;
            R7.y = R3.z * PerDraw.LocalToWorldMatrix[2].z;
            R126.w = R3.z * PerDraw.LocalToWorldMatrix[2].x;
            R126.x = fma(R3.y, PerDraw.LocalToWorldMatrix[1].x, R126.w);
            PS8 = R3.z * PerDraw.LocalToWorldMatrix[2].y;
            R126.z = fma(R3.y, PerDraw.LocalToWorldMatrix[1].y, PS8);
            R124.w = fma(R3.y, PerDraw.LocalToWorldMatrix[1].z, R7.y);
            R127.x = fma(R3.x, PerDraw.LocalToWorldMatrix[0].x, R126.x);
            R3_backup.x = R3.x;
            R126.y = fma(R3_backup.x, PerDraw.LocalToWorldMatrix[0].y, R126.z);
            PV12.y = R126.y;
            R127.z = fma(R3_backup.x, PerDraw.LocalToWorldMatrix[0].z, R124.w);
            PV12.z = R127.z;
            temp13 = dot(vec4(R127.x, PV12.y, PV12.z, 0.0), vec4(R127.x, PV12.y, PV12.z, 0.0));
            PV13.x = temp13;
            R127.y = inversesqrt(PV13.x);
            PS14 = R127.y;
            R126_backup.y = R126.y;
            R3.z = R127.z * PS14;
            R1.w = R126_backup.y * PS14;
            R4.y = R127.x * R127.y;
            R124.x = FB0.bgRotInv[2].y * R3.z;
            R125.y = fma(R1.w, FB0.bgRotInv[1].y, R124.x);
            R17.y = fma(R4.y, FB0.bgRotInv[0].y, R125.y);
            result = R17.y;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static TRANSFORM_NORMAL_Z: LazyLock<Graph> = LazyLock::new(|| {
    // texas_cross.105.vert.
    let query = indoc! {"
        void main() {
            R3.x = a_Normal_x;
            R3.y = a_Normal_y;
            R3.z = a_Normal_z;
            R7.y = R3.z * PerDraw.LocalToWorldMatrix[2].z;
            R126.w = R3.z * PerDraw.LocalToWorldMatrix[2].x;
            R126.x = fma(R3.y, PerDraw.LocalToWorldMatrix[1].x, R126.w);
            PS8 = R3.z * PerDraw.LocalToWorldMatrix[2].y;
            R126.z = fma(R3.y, PerDraw.LocalToWorldMatrix[1].y, PS8);
            R124.w = fma(R3.y, PerDraw.LocalToWorldMatrix[1].z, R7.y);
            R127.x = fma(R3.x, PerDraw.LocalToWorldMatrix[0].x, R126.x);
            R3_backup.x = R3.x;
            R126.y = fma(R3_backup.x, PerDraw.LocalToWorldMatrix[0].y, R126.z);
            PV12.y = R126.y;
            R127.z = fma(R3_backup.x, PerDraw.LocalToWorldMatrix[0].z, R124.w);
            PV12.z = R127.z;
            temp13 = dot(vec4(R127.x, PV12.y, PV12.z, 0.0), vec4(R127.x, PV12.y, PV12.z, 0.0));
            PV13.x = temp13;
            R127.y = inversesqrt(PV13.x);
            PS14 = R127.y;
            R126_backup.y = R126.y;
            R3.z = R127.z * PS14;
            R1.w = R126_backup.y * PS14;
            R4.y = R127.x * R127.y;
            R126.x = FB0.bgRotInv[2].z * R3.z;
            R126_backup.x = R126.x;
            R1.y = fma(R1.w, FB0.bgRotInv[1].z, R126_backup.x);
            R17.z = fma(R4.y, FB0.bgRotInv[0].z, R1.y);
            result = R17.z;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn transform_normal<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    query_nodes(expr, graph, &TRANSFORM_NORMAL_X)
        .and_then(|r| r.get("a_Normal_x").copied())
        .or_else(|| {
            query_nodes(expr, graph, &TRANSFORM_NORMAL_Y).and_then(|r| r.get("a_Normal_y").copied())
        })
        .or_else(|| {
            query_nodes(expr, graph, &TRANSFORM_NORMAL_Z).and_then(|r| r.get("a_Normal_z").copied())
        })
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
