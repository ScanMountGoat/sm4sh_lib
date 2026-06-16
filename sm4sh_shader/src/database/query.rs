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
            normal_map_x = normal_map_x - 0.00196078;
            normal_map_x = normal_map_x * 2.0;
            normal_map_x = normal_map_x - 1.0;

            normal_map_y = normal_map_y - 0.00196078;
            normal_map_y = normal_map_y * 2.0;
            normal_map_y = normal_map_y - 1.0;

            normal_map_z = normal_map_z - 0.00196078;
            normal_map_z = normal_map_z * 2.0;
            normal_map_z = normal_map_z - 1.0;

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

    Some((
        op,
        vec![
            result.get("normal_map_x")?,
            result.get("normal_map_y")?,
            result.get("normal_map_z")?,
        ],
    ))
}

fn transform_normal_query(c: char) -> String {
    // texas_cross.105.vert.
    formatdoc! {"
        void main() {{
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
            R124.w = FB0.bgRotInv[2].{c} * R3.z;
            R124.w = fma(R1.w, FB0.bgRotInv[1].{c}, R124.w);
            R17.x = fma(R4.y, FB0.bgRotInv[0].{c}, R124.w);
            result = R17.x;
        }}
    "}
}

static TRANSFORM_NORMAL_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = transform_normal_query('x');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static TRANSFORM_NORMAL_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = transform_normal_query('y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static TRANSFORM_NORMAL_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = transform_normal_query('z');
    Graph::parse_glsl(&query).unwrap().simplify()
});

pub fn local_to_world_normal<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    query_nodes(expr, graph, &TRANSFORM_NORMAL_X)
        .and_then(|r| r.get("a_Normal_x").copied())
        .or_else(|| {
            query_nodes(expr, graph, &TRANSFORM_NORMAL_Y).and_then(|r| r.get("a_Normal_y").copied())
        })
        .or_else(|| {
            query_nodes(expr, graph, &TRANSFORM_NORMAL_Z).and_then(|r| r.get("a_Normal_z").copied())
        })
}

fn transform_binormal_query(c: char) -> String {
    // texas_cross.105.vert.
    formatdoc! {"
        void main() {{
            R1.x = a_Binormal_x;
            R1.y = a_Binormal_y;
            R1.z = a_Binormal_z;
            R0.z = R1.z * PerDraw.LocalToWorldMatrix[2].y;
            PS13 = R1.z * PerDraw.LocalToWorldMatrix[2].z;
            R126.x = fma(R1.y, PerDraw.LocalToWorldMatrix[1].z, PS13);
            PV15.w = R1.z * PerDraw.LocalToWorldMatrix[2].x;
            R2.x = fma(R1.x, PerDraw.LocalToWorldMatrix[0].z, R126.x);
            R123.y = fma(R1.y, PerDraw.LocalToWorldMatrix[1].x, PV15.w);
            PV16.y = R123.y;
            R123.w = fma(R1.y, PerDraw.LocalToWorldMatrix[1].y, R0.z);
            PV16.w = R123.w;
            R1_backup.x = R1.x;
            R1.x = fma(R1_backup.x, PerDraw.LocalToWorldMatrix[0].y, PV16.w);
            R3.y = fma(R1_backup.x, PerDraw.LocalToWorldMatrix[0].x, PV16.y);
            R126.z = FB0.bgRotInv[2].{c} * R2.x;
            R126_backup.z = R126.z;
            R127.w = fma(R1.x, FB0.bgRotInv[1].{c}, R126_backup.z);
            R15.x = fma(R3.y, FB0.bgRotInv[0].{c}, R127.w);
            result = R15.x;
        }}
    "}
}

static TRANSFORM_BINORMAL_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = transform_binormal_query('x');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static TRANSFORM_BINORMAL_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = transform_binormal_query('y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static TRANSFORM_BINORMAL_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = transform_binormal_query('z');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static TRANSFORM_BINORMAL_W: LazyLock<Graph> = LazyLock::new(|| {
    // texas_cross.105.vert
    let query = indoc! {"
        void main() {
            R1.x = a_Binormal_x;
            R1.y = a_Binormal_y;
            R1.z = a_Binormal_z;
            R3.x = a_Normal_x;
            R3.y = a_Normal_y;
            R3.z = a_Normal_z;
            R5.x = a_Tangent_x;
            R5.y = a_Tangent_y;
            R5.z = a_Tangent_z;
            R126.z = R1.y * R3.z;
            R1.w = R1.z * R3.x;
            R125.x = fma(-R3.y, R1.z, R126.z);
            R127.z = R1.x * R3.y;
            R124.y = fma(-R3.z, R1.x, R1.w);
            R3_backup.x = R3.x;
            R3.x = fma(-R3_backup.x, R1.y, R127.z);
            temp18 = dot(vec4(R5.x, R5.y, R5.z, 0.0), vec4(R125.x, R124.y, R3.x, 0.0));
            PV18.x = temp18;
            R1.z = PV18.x > 0.0 ? 1.0 : 0.0;
            R3.w = 0.0 > PV18.x ? 1.0 : 0.0;
            R15.w = R1.z - R3.w;
            result = R15.w;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn local_to_world_binormal(graph: &Graph, expr: &Expr) -> Option<Expr> {
    query_nodes(expr, graph, &TRANSFORM_BINORMAL_X)
        .and_then(|r| r.get("a_Binormal_x").copied().cloned())
        .or_else(|| {
            query_nodes(expr, graph, &TRANSFORM_BINORMAL_Y)
                .and_then(|r| r.get("a_Binormal_y").copied().cloned())
        })
        .or_else(|| {
            query_nodes(expr, graph, &TRANSFORM_BINORMAL_Z)
                .and_then(|r| r.get("a_Binormal_z").copied().cloned())
        })
        .or_else(|| {
            // The sign can be calculated in consuming applications.
            query_nodes(expr, graph, &TRANSFORM_BINORMAL_W).map(|_| Expr::Global {
                name: "bitangent_sign".into(),
                channel: None,
            })
        })
}

fn eye_vector_query(c: char) -> String {
    // texas_cross.105.vert.
    formatdoc! {"
        void main() {{
            R4.x = a_Position_x;
            R4.y = a_Position_y;
            R4.z = a_Position_z;
            PV0.w = PerDraw.LocalToWorldMatrix[3].z * 1.0;
            R127.w = fma(R4.z, PerDraw.LocalToWorldMatrix[2].z, PV0.w);
            R125.z = fma(R4.y, PerDraw.LocalToWorldMatrix[1].z, R127.w);
            R126.y = PerDraw.LocalToWorldMatrix[3].x * 1.0;
            PV8.x = PerDraw.LocalToWorldMatrix[3].y * 1.0;
            R123.x = fma(R4.z, PerDraw.LocalToWorldMatrix[2].y, PV8.x);
            PV9.x = R123.x;
            R123.y = fma(R4.z, PerDraw.LocalToWorldMatrix[2].x, R126.y);
            PV9.y = R123.y;
            R123.x = fma(R4.y, PerDraw.LocalToWorldMatrix[1].y, PV9.x);
            PV10.x = R123.x;
            R123.y = fma(R4.y, PerDraw.LocalToWorldMatrix[1].x, PV9.y);
            PV10.y = R123.y;
            R8.x = fma(R4.x, PerDraw.LocalToWorldMatrix[0].x, PV10.y);
            R9.y = fma(R4.x, PerDraw.LocalToWorldMatrix[0].y, PV10.x);
            R4.z = fma(R4.x, PerDraw.LocalToWorldMatrix[0].z, R125.z);
            R124.w = PerView.WorldToViewMatrix[2].x;
            R124.y = PerView.WorldToViewMatrix[2].y;
            R127.z = PerView.WorldToViewMatrix[2].z;
            R0.w = PerView.WorldToViewMatrix[1].x;
            R8.y = PerView.WorldToViewMatrix[1].y;
            R5.z = PerView.WorldToViewMatrix[1].z;
            temp31 = dot(vec4(R124.w, R124.y, R127.z, 0.0), vec4(PerView.WorldToViewMatrix[3].x, PerView.WorldToViewMatrix[3].y, PerView.WorldToViewMatrix[3].z, 0.0));
            PV31.x = temp31;
            R1.y = PerView.WorldToViewMatrix[0].x;
            R2.w = PerView.WorldToViewMatrix[0].y;
            R5.y = -PV31.x;
            temp34 = dot(vec4(R0.w, R8.y, R5.z, 0.0), vec4(PerView.WorldToViewMatrix[3].x, PerView.WorldToViewMatrix[3].y, PerView.WorldToViewMatrix[3].z, 0.0));
            PV34.x = temp34;
            R126.z = -R4.z + R5.y;
            PV35.y = -PV34.x;
            R127.z = PerView.WorldToViewMatrix[0].z;
            R125.w = -R9.y + PV35.y;
            temp37 = dot(vec4(R1.y, R2.w, R127.z, 0.0), vec4(PerView.WorldToViewMatrix[3].x, PerView.WorldToViewMatrix[3].y, PerView.WorldToViewMatrix[3].z, 0.0));
            PV37.x = temp37;
            R125.x = -PV37.x;
            R127.y = R126.z * FB0.bgRotInv[2].{c};
            R125.x = -R8.x + R125.x;
            R127_backup.y = R127.y;
            R125.z = fma(R125.w, FB0.bgRotInv[1].{c}, R127_backup.y);
            R14.x = fma(R125.x, FB0.bgRotInv[0].{c}, R125.z);
            result = R14.x;
        }}
    "}
}

static EYE_VECTOR_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = eye_vector_query('x');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static EYE_VECTOR_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = eye_vector_query('y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static EYE_VECTOR_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = eye_vector_query('z');
    Graph::parse_glsl(&query).unwrap().simplify()
});

pub fn eye_vector(graph: &Graph, expr: &Expr) -> Option<Expr> {
    // The eye vector can easily be calculated in consuming code.
    query_nodes(expr, graph, &EYE_VECTOR_X)
        .map(|_| Expr::Global {
            name: "eye".into(),
            channel: Some('x'),
        })
        .or_else(|| {
            query_nodes(expr, graph, &EYE_VECTOR_Y).map(|_| Expr::Global {
                name: "eye".into(),
                channel: Some('y'),
            })
        })
        .or_else(|| {
            query_nodes(expr, graph, &EYE_VECTOR_Z).map(|_| Expr::Global {
                name: "eye".into(),
                channel: Some('z'),
            })
        })
}

fn light_position_query(c: char) -> String {
    // texas_cross.105.vert.
    formatdoc! {"
        void main() {{
            R4.x = a_Position_x;
            R4.y = a_Position_y;
            R4.z = a_Position_z;
            R127.x = PerDraw.LocalToWorldMatrix[3].w * 1.0;
            PV0.x = R127.x;
            PV0.w = PerDraw.LocalToWorldMatrix[3].z * 1.0;
            PV1.x = PV0.x;
            R127.w = fma(R4.z, PerDraw.LocalToWorldMatrix[2].z, PV0.w);
            R123.z = fma(R4.z, PerDraw.LocalToWorldMatrix[2].w, PV1.x);
            PV2.z = R123.z;
            R125.z = fma(R4.y, PerDraw.LocalToWorldMatrix[1].z, R127.w);
            R124.z = fma(R4.y, PerDraw.LocalToWorldMatrix[1].w, PV2.z);
            R126.y = PerDraw.LocalToWorldMatrix[3].x * 1.0;
            PV8.x = PerDraw.LocalToWorldMatrix[3].y * 1.0;
            R123.x = fma(R4.z, PerDraw.LocalToWorldMatrix[2].y, PV8.x);
            PV9.x = R123.x;
            R123.y = fma(R4.z, PerDraw.LocalToWorldMatrix[2].x, R126.y);
            PV9.y = R123.y;
            R123.x = fma(R4.y, PerDraw.LocalToWorldMatrix[1].y, PV9.x);
            PV10.x = R123.x;
            R123.y = fma(R4.y, PerDraw.LocalToWorldMatrix[1].x, PV9.y);
            PV10.y = R123.y;
            R8.x = fma(R4.x, PerDraw.LocalToWorldMatrix[0].x, PV10.y);
            R9.y = fma(R4.x, PerDraw.LocalToWorldMatrix[0].y, PV10.x);
            R4.z = fma(R4.x, PerDraw.LocalToWorldMatrix[0].z, R125.z);
            R6.w = fma(R4.x, PerDraw.LocalToWorldMatrix[0].w, R124.z);
            R7.x = R6.w * FB0.ShadowMapMatrix[3].w;
            R126.w = R6.w * FB0.ShadowMapMatrix[3].{c};
            R127.x = fma(R4.z, FB0.ShadowMapMatrix[2].w, R7.x);
            R126.w = fma(R4.z, FB0.ShadowMapMatrix[2].{c}, R126.w);
            R127.x = fma(R9.y, FB0.ShadowMapMatrix[1].w, R127.x);
            R126.w = fma(R9.y, FB0.ShadowMapMatrix[1].{c}, R126.w);
            R127.x = fma(R8.x, FB0.ShadowMapMatrix[0].w, R127.x);
            R126.w = fma(R8.x, FB0.ShadowMapMatrix[0].{c}, R126.w);
            R125.w = 1.0 / R127.x;
            R16.x = R126.w * R125.w;
            result = R16.x;
        }}
    "}
}

static LIGHT_POSITION_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = light_position_query('x');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static LIGHT_POSITION_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = light_position_query('y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static LIGHT_POSITION_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = light_position_query('z');
    Graph::parse_glsl(&query).unwrap().simplify()
});

pub fn light_position(graph: &Graph, expr: &Expr) -> Option<Expr> {
    // The position in light space for shadow mapping can easily be calculated in consuming code.
    query_nodes(expr, graph, &LIGHT_POSITION_X)
        .map(|_| Expr::Global {
            name: "light_position".into(),
            channel: Some('x'),
        })
        .or_else(|| {
            query_nodes(expr, graph, &LIGHT_POSITION_Y).map(|_| Expr::Global {
                name: "light_position".into(),
                channel: Some('y'),
            })
        })
        .or_else(|| {
            query_nodes(expr, graph, &LIGHT_POSITION_Z).map(|_| Expr::Global {
                name: "light_position".into(),
                channel: Some('z'),
            })
        })
}

static LIGHT_MAP_POS_X: LazyLock<Graph> = LazyLock::new(|| {
    // texas_cross.105.vert
    let query = indoc! {"
        void main() {
            R4.x = a_Position_x;
            R4.y = a_Position_y;
            R4.z = a_Position_z;
            PV0.w = PerDraw.LocalToWorldMatrix[3].z * 1.0;
            R127.w = fma(R4.z, PerDraw.LocalToWorldMatrix[2].z, PV0.w);
            R125.z = fma(R4.y, PerDraw.LocalToWorldMatrix[1].z, R127.w);
            R126.y = PerDraw.LocalToWorldMatrix[3].x * 1.0;
            PV8.x = PerDraw.LocalToWorldMatrix[3].y * 1.0;
            R123.x = fma(R4.z, PerDraw.LocalToWorldMatrix[2].y, PV8.x);
            PV9.x = R123.x;
            R123.y = fma(R4.z, PerDraw.LocalToWorldMatrix[2].x, R126.y);
            PV9.y = R123.y;
            R123.x = fma(R4.y, PerDraw.LocalToWorldMatrix[1].y, PV9.x);
            PV10.x = R123.x;
            R123.y = fma(R4.y, PerDraw.LocalToWorldMatrix[1].x, PV9.y);
            PV10.y = R123.y;
            R8.x = fma(R4.x, PerDraw.LocalToWorldMatrix[0].x, PV10.y);
            R9.y = fma(R4.x, PerDraw.LocalToWorldMatrix[0].y, PV10.x);
            R4.z = fma(R4.x, PerDraw.LocalToWorldMatrix[0].z, R125.z);
            PV14.w = FB1.lightMapMatrix[3].x * 1.0;
            R126.y = fma(R4.z, FB1.lightMapMatrix[2].x, PV14.w);
            R124.w = fma(R9.y, FB1.lightMapMatrix[1].x, R126.y);
            R2.y = fma(R8.x, FB1.lightMapMatrix[0].x, R124.w);
            R2.y = FB0.lightMapPos.x + R2.y;
            R3.z = R2.y;
            result = R3.z;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static LIGHT_MAP_POS_Y: LazyLock<Graph> = LazyLock::new(|| {
    // texas_cross.105.vert
    let query = indoc! {"
        void main() {
            R4.x = a_Position_x;
            R4.y = a_Position_y;
            R4.z = a_Position_z;
            PV0.w = PerDraw.LocalToWorldMatrix[3].z * 1.0;
            R127.w = fma(R4.z, PerDraw.LocalToWorldMatrix[2].z, PV0.w);
            R125.z = fma(R4.y, PerDraw.LocalToWorldMatrix[1].z, R127.w);
            R126.y = PerDraw.LocalToWorldMatrix[3].x * 1.0;
            PV8.x = PerDraw.LocalToWorldMatrix[3].y * 1.0;
            R123.x = fma(R4.z, PerDraw.LocalToWorldMatrix[2].y, PV8.x);
            PV9.x = R123.x;
            R123.y = fma(R4.z, PerDraw.LocalToWorldMatrix[2].x, R126.y);
            PV9.y = R123.y;
            R123.x = fma(R4.y, PerDraw.LocalToWorldMatrix[1].y, PV9.x);
            PV10.x = R123.x;
            R123.y = fma(R4.y, PerDraw.LocalToWorldMatrix[1].x, PV9.y);
            PV10.y = R123.y;
            R8.x = fma(R4.x, PerDraw.LocalToWorldMatrix[0].x, PV10.y);
            R9.y = fma(R4.x, PerDraw.LocalToWorldMatrix[0].y, PV10.x);
            R4.z = fma(R4.x, PerDraw.LocalToWorldMatrix[0].z, R125.z);
            PV14.z = FB1.lightMapMatrix[3].y * 1.0;
            R124.x = fma(R4.z, FB1.lightMapMatrix[2].y, PV14.z);
            R127.z = fma(R9.y, FB1.lightMapMatrix[1].y, R124.x);
            R3.x = fma(R8.x, FB1.lightMapMatrix[0].y, R127.z);
            R3.x = FB0.lightMapPos.y + R3.x;
            R3_backup.x = R3.x;
            R3.w = R3_backup.x;
            result = R3.w;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn light_map_position(graph: &Graph, expr: &Expr) -> Option<Expr> {
    // The light map position can easily be calculated in consuming code.
    query_nodes(expr, graph, &LIGHT_MAP_POS_X)
        .map(|_| Expr::Global {
            name: "light_map_position".into(),
            channel: Some('x'),
        })
        .or_else(|| {
            query_nodes(expr, graph, &LIGHT_MAP_POS_Y).map(|_| Expr::Global {
                name: "light_map_position".into(),
                channel: Some('y'),
            })
        })
}

static SPHERE_MAP_COORD_X: LazyLock<Graph> = LazyLock::new(|| {
    // texas_cross.79.vert
    // TODO: Is the param always MC.reflectionParams.y?
    let query = indoc! {"
        void main() {
            R3.x = a_Normal_x;
            R3.y = a_Normal_y;
            R3.z = a_Normal_z;
            R4.x = a_Position_x;
            R4.y = a_Position_y;
            R4.z = a_Position_z;
            temp12 = dot(vec4(R3.x, R3.y, R3.z, 0.0), vec4(R3.x, R3.y, R3.z, 0.0));
            PV12.x = temp12;
            R6.z = inversesqrt(PV12.x);
            PV36.x = R4.y * param;
            PV36.y = R4.x * param;
            PV36.w = R4.z * param;
            R3_backup.x = R3.x;
            R3.x = fma(R3.z, R6.z, PV36.w);
            R3.y = fma(R3.y, R6.z, PV36.x);
            R3.z = fma(R3_backup.x, R6.z, PV36.y);
            R4.w = fma(-param, 0.25, 0.5);
            PV43.z = PerDraw.LocalToViewMatrix[2].x * R3.x;
            R123.x = fma(R3.y, PerDraw.LocalToViewMatrix[1].x, PV43.z);
            PV44.x = R123.x;
            R123.x = fma(R3.z, PerDraw.LocalToViewMatrix[0].x, PV44.x);
            PV45.x = R123.x;
            R127.x = R4.w * PV45.x;
            R127_backup.x = R127.x;
            PS48 = R127_backup.x + 0.5;
            R13.z = PS48;
            result = R13.z;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static SPHERE_MAP_COORD_Y: LazyLock<Graph> = LazyLock::new(|| {
    // texas_cross.79.vert
    // TODO: Is the param always MC.reflectionParams.y?
    let query = indoc! {"
        void main() {
            R3.x = a_Normal_x;
            R3.y = a_Normal_y;
            R3.z = a_Normal_z;
            R4.x = a_Position_x;
            R4.y = a_Position_y;
            R4.z = a_Position_z;
            temp12 = dot(vec4(R3.x, R3.y, R3.z, 0.0), vec4(R3.x, R3.y, R3.z, 0.0));
            PV12.x = temp12;
            R6.z = inversesqrt(PV12.x);
            PV36.x = R4.y * param;
            PV36.y = R4.x * param;
            PV36.w = R4.z * param;
            R3_backup.x = R3.x;
            R3.x = fma(R3.z, R6.z, PV36.w);
            R3.y = fma(R3.y, R6.z, PV36.x);
            R3.z = fma(R3_backup.x, R6.z, PV36.y);
            R4.w = fma(-param, 0.25, 0.5);
            PV43.y = PerDraw.LocalToViewMatrix[2].y * R3.x;
            R123.w = fma(R3.y, PerDraw.LocalToViewMatrix[1].y, PV43.y);
            PV44.w = R123.w;
            R123.y = fma(R3.z, PerDraw.LocalToViewMatrix[0].y, PV44.w);
            PV45.y = R123.y;
            R126.w = R4.w * PV45.y;
            R124.z = -R126.w + 0.5;
            R124_backup.z = R124.z;
            R13.w = R124_backup.z;
            result = R13.w;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_sphere_map_coords<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    // The sphere map coordinates can easily be calculated in consuming code.
    query_nodes(expr, graph, &SPHERE_MAP_COORD_X)
        .and_then(|r| Some((Operation::SphereMapCoordX, vec![r.get("param").copied()?])))
        .or_else(|| {
            query_nodes(expr, graph, &SPHERE_MAP_COORD_Y)
                .and_then(|r| Some((Operation::SphereMapCoordY, vec![r.get("param").copied()?])))
        })
}

fn local_to_world_point_query(c: char) -> String {
    // texas_cross.105.vert.
    // PerDraw.LocalToWorldMatrix * vec4(attribute.xyz, 1.0)
    formatdoc! {"
        void main() {{
            attribute_x = attribute_x;
            attribute_y = attribute_y;
            attribute_z = attribute_z;
            result = PerDraw.LocalToWorldMatrix[3].{c} * 1.0;
            result = fma(attribute_z, PerDraw.LocalToWorldMatrix[2].{c}, result);
            result = fma(attribute_y, PerDraw.LocalToWorldMatrix[1].{c}, result);
            result = fma(attribute_x, PerDraw.LocalToWorldMatrix[0].{c}, result);
        }}
    "}
}

static LOCAL_TO_WORLD_POINT_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = local_to_world_point_query('x');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static LOCAL_TO_WORLD_POINT_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = local_to_world_point_query('y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static LOCAL_TO_WORLD_POINT_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = local_to_world_point_query('z');
    Graph::parse_glsl(&query).unwrap().simplify()
});

pub fn op_local_to_world_point<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    query_nodes(expr, graph, &LOCAL_TO_WORLD_POINT_X)
        .and_then(|r| {
            Some((
                Operation::LocalToWorldPointX,
                vec![
                    r.get("attribute_x").copied()?,
                    r.get("attribute_y").copied()?,
                    r.get("attribute_z").copied()?,
                ],
            ))
        })
        .or_else(|| {
            query_nodes(expr, graph, &LOCAL_TO_WORLD_POINT_Y).and_then(|r| {
                Some((
                    Operation::LocalToWorldPointY,
                    vec![
                        r.get("attribute_x").copied()?,
                        r.get("attribute_y").copied()?,
                        r.get("attribute_z").copied()?,
                    ],
                ))
            })
        })
        .or_else(|| {
            query_nodes(expr, graph, &LOCAL_TO_WORLD_POINT_Z).and_then(|r| {
                Some((
                    Operation::LocalToWorldPointZ,
                    vec![
                        r.get("attribute_x").copied()?,
                        r.get("attribute_y").copied()?,
                        r.get("attribute_z").copied()?,
                    ],
                ))
            })
        })
}

fn local_to_world_vector_query(c: char) -> String {
    // texas_cross.105.vert.
    // PerDraw.LocalToWorldMatrix * vec4(attribute.xyz, 0.0)
    formatdoc! {"
        void main() {{
            attribute_x = attribute_x;
            attribute_y = attribute_y;
            attribute_z = attribute_z;
            result = attribute_z * PerDraw.LocalToWorldMatrix[2].{c};
            result = fma(attribute_y, PerDraw.LocalToWorldMatrix[1].{c}, result);
            result = fma(attribute_x, PerDraw.LocalToWorldMatrix[0].{c}, result);
        }}
    "}
}

static LOCAL_TO_WORLD_VECTOR_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = local_to_world_vector_query('x');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static LOCAL_TO_WORLD_VECTOR_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = local_to_world_vector_query('y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static LOCAL_TO_WORLD_VECTOR_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = local_to_world_vector_query('z');
    Graph::parse_glsl(&query).unwrap().simplify()
});

pub fn op_local_to_world_vector<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    query_nodes(expr, graph, &LOCAL_TO_WORLD_VECTOR_X)
        .and_then(|r| {
            Some((
                Operation::LocalToWorldVectorX,
                vec![
                    r.get("attribute_x").copied()?,
                    r.get("attribute_y").copied()?,
                    r.get("attribute_z").copied()?,
                ],
            ))
        })
        .or_else(|| {
            query_nodes(expr, graph, &LOCAL_TO_WORLD_VECTOR_Y).and_then(|r| {
                Some((
                    Operation::LocalToWorldVectorY,
                    vec![
                        r.get("attribute_x").copied()?,
                        r.get("attribute_y").copied()?,
                        r.get("attribute_z").copied()?,
                    ],
                ))
            })
        })
        .or_else(|| {
            query_nodes(expr, graph, &LOCAL_TO_WORLD_VECTOR_Z).and_then(|r| {
                Some((
                    Operation::LocalToWorldVectorZ,
                    vec![
                        r.get("attribute_x").copied()?,
                        r.get("attribute_y").copied()?,
                        r.get("attribute_z").copied()?,
                    ],
                ))
            })
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
    Some((
        Operation::Mix,
        vec![result.get("a")?, result.get("b")?, result.get("ratio")?],
    ))
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
    Some((Operation::Power, vec![result.get("a")?, result.get("b")?]))
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
    Some((Operation::Sqrt, vec![result.get("result")?]))
}

static OP_DOT4: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            result = dot(vec4(ax, ay, az, aw), vec4(bx, by, bz, bw));
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_dot4<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    query_nodes(expr, graph, &OP_DOT4).and_then(|result| {
        Some((
            Operation::Dot4,
            vec![
                *result.get("ax")?,
                *result.get("ay")?,
                *result.get("az")?,
                *result.get("aw")?,
                *result.get("bx")?,
                *result.get("by")?,
                *result.get("bz")?,
                *result.get("bw")?,
            ],
        ))
    })
}

static OP_DOT3: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            result = dot(vec4(ax, ay, az, 0.0), vec4(bx, by, bz, 0.0));
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_dot3<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    query_nodes(expr, graph, &OP_DOT3).and_then(|result| {
        Some((
            Operation::Dot3,
            vec![
                *result.get("ax")?,
                *result.get("ay")?,
                *result.get("az")?,
                *result.get("bx")?,
                *result.get("by")?,
                *result.get("bz")?,
            ],
        ))
    })
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
    Some((Operation::Div, vec![result.get("a")?, result.get("b")?]))
}

static OP_NORMALIZE_XYZ: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            length = dot(vec4(x, y, z, 0.0), vec4(x, y, z, 0.0));
            inverse_length = inversesqrt(length);
            result = value * inverse_length;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_normalize<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_NORMALIZE_XYZ)?;
    let value = result.get("value")?;
    let x = result.get("x")?;
    let y = result.get("y")?;
    let z = result.get("z")?;

    let op = if value == x {
        Operation::NormalizeX
    } else if value == y {
        Operation::NormalizeY
    } else if value == z {
        Operation::NormalizeZ
    } else {
        return None;
    };

    Some((op, vec![x, y, z]))
}

static OP_NEG_REFLECT: LazyLock<Graph> = LazyLock::new(|| {
    // -reflect(I, N) = -(I - 2.0 * dot(N, I) * N)
    // -2.0 * dot(-I, N) * N - I = 2 * dot(N, I) * N - I = -(I - 2 * dot(I, N) * N)
    let query = indoc! {"
        void main() {
            dot_product = dot(vec4(-I_x, -I_y, -I_z, -0.0), vec4(N_x, N_y, N_z, -0.0));
            two_dot_product = dot_product + dot_product;
            result = fma(-two_dot_product, N_value, -I_value);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_neg_reflect<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_NEG_REFLECT)?;
    let n = result.get("N_value")?;
    let n_x = result.get("N_x")?;
    let n_y = result.get("N_y")?;
    let n_z = result.get("N_z")?;
    let i_x = result.get("I_x")?;
    let i_y = result.get("I_y")?;
    let i_z = result.get("I_z")?;

    let op = if n == n_x {
        Operation::NegReflectX
    } else if n == n_y {
        Operation::NegReflectY
    } else if n == n_z {
        Operation::NegReflectZ
    } else {
        return None;
    };

    Some((op, vec![i_x, i_y, i_z, n_x, n_y, n_z]))
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

fn latte_texture_cube_query(c: char) -> String {
    // cube.xyzw = cube(R.zzxy, R.yxzz)
    // texture(s0, cube.yx / abs(cube.z) + 1.5))
    formatdoc! {"
        void main() {{
            cube_z = 1.0 / abs(cube_z);
            cube_x = cube(R_z, R_y); 
            cube_y = cube(R_z, R_x); 
            result_s = fma(cube_y, cube_z, 1.5);
            result_t = fma(cube_x, cube_z, 1.5);
            result = texture(tex, vec2(result_s, result_t)).{c};
        }}
    "}
}

static LATTE_TEXTURE_CUBE_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = latte_texture_cube_query('x');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static LATTE_TEXTURE_CUBE_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = latte_texture_cube_query('y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static LATTE_TEXTURE_CUBE_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = latte_texture_cube_query('z');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static LATTE_TEXTURE_CUBE_W: LazyLock<Graph> = LazyLock::new(|| {
    let query = latte_texture_cube_query('w');
    Graph::parse_glsl(&query).unwrap().simplify()
});

pub fn latte_texture_cube_coords<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<Expr> {
    // Find the reflection vector R from latte cube coordinates.
    let (result, channel) = query_nodes(expr, graph, &LATTE_TEXTURE_CUBE_X)
        .map(|r| (r, 'x'))
        .or_else(|| query_nodes(expr, graph, &LATTE_TEXTURE_CUBE_Y).map(|r| (r, 'y')))
        .or_else(|| query_nodes(expr, graph, &LATTE_TEXTURE_CUBE_Z).map(|r| (r, 'z')))
        .or_else(|| query_nodes(expr, graph, &LATTE_TEXTURE_CUBE_W).map(|r| (r, 'w')))?;

    let args = [
        result.get("tex")?,
        result.get("R_x")?,
        result.get("R_y")?,
        result.get("R_z")?,
    ];

    // Convert a latte specific cube map lookup to a standard GLSL call.
    // TODO: add the vec3 call?
    Some(Expr::Func {
        name: "textureCube".into(),
        args: args
            .into_iter()
            .map(|a| graph.exprs.iter().position(|e| e == *a).unwrap())
            .collect(),
        channel: Some(channel),
    })
}

static OP_VARIANCE_SHADOW: LazyLock<Graph> = LazyLock::new(|| {
    // variance shadow mapping using the first and second moments from the VSM texture.
    // shadow = pow(sigma2 / (max(tDif, 0.0)^2 + sigma2), 4)
    let query = indoc! {"
        void main() {
            light_position_z = min(light_position_z, 1.0);
            sigma2 = fma(-m1, m1, m2);
            tdif = -m1 + light_position_z;
            sigma2 = sigma2 + offset;
            sigma2 = clamp(sigma2, 0.0, 1.0);
            max_tdif = max(tdif, 0.0);
            denom = fma(max_tdif, max_tdif, sigma2);
            one_over_denom = 1.0 / denom;
            shadow = sigma2 * one_over_denom;
            shadow2 = shadow * shadow;
            shadow4 = shadow2 * shadow2;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_variance_shadow<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_VARIANCE_SHADOW)?;
    Some((
        Operation::VarianceShadow,
        vec![
            result.get("m1")?,
            result.get("m2")?,
            result.get("light_position_z")?,
            result.get("offset")?,
        ],
    ))
}

static OP_BLINN_PHONG_SPEC: LazyLock<Graph> = LazyLock::new(|| {
    // Blinn-Phong using the halfway vector H from texas_cross.64.frag.
    // pow(dot(N, H), MC.specularParams.y)
    let query = indoc! {"
        void main() {
            h_inv_length = inversesqrt(h_length);
            h_x = eye_x - light_dir_x;
            h_x = h_x * h_inv_length;

            h_y = eye_y - light_dir_y;
            h_y = h_y * h_inv_length;

            h_z = eye_z - light_dir_z;
            h_z = h_z * h_inv_length;

            spec = dot(vec4(n_x, n_y, n_z, 0.0), vec4(h_x, h_y, h_z, 0.0));
            spec = max(spec, 0.001);
            spec = log2(spec);
            spec = exponent * spec;
            spec = exp2(spec);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_blinn_phong_spec<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_BLINN_PHONG_SPEC)?;
    Some((
        Operation::BlinnPhongSpecular,
        vec![
            result.get("n_x")?,
            result.get("n_y")?,
            result.get("n_z")?,
            result.get("light_dir_x")?,
            result.get("light_dir_y")?,
            result.get("light_dir_z")?,
            result.get("eye_x")?,
            result.get("eye_y")?,
            result.get("eye_z")?,
            result.get("exponent")?,
        ],
    ))
}

static OP_BLINN_PHONG_SPEC_ANISOTROPIC: LazyLock<Graph> = LazyLock::new(|| {
    // Anisotropic specular shading from texas_cross.105.frag.
    // This appears to be a slightly modified Ward BRDF.
    // The tangent is recalculated in the fragment shader.
    // log2(e) = 1.442695 is used to express exp using the faster exp2.
    // The vector b is the eye vector orthogonalized to the normal with the Gram-Schmidt process.
    let query = indoc! {"
        void main() {
            param_x = param_x * 3.0;
            param_x2 = param_x * param_x;
            one_over_param_x2 = 1.0 / param_x2;

            dot_eye_n = dot(vec4(eye_x, eye_y, eye_z, 0.0), vec4(normal_x, normal_y, normal_z, 0.0));
            dot_eye_n2 = dot_eye_n * dot_eye_n;
            one_over_dot_eye_n2 = 1.0 / dot_eye_n2;
            dot_eye_n2_minus_one = dot_eye_n2 - 1.0;

            b_x = fma(-normal_x, dot_eye_n, eye_x);
            b_y = fma(-normal_y, dot_eye_n, eye_y);
            b_z = fma(-normal_z, dot_eye_n, eye_z);

            b_x = b_x * inverse_b_length;
            b_y = b_y * inverse_b_length;
            b_z = b_z * inverse_b_length;

            dot_tangent_b = dot(vec4(tangent_x, tangent_y, tangent_z, 0.0), vec4(b_x, b_y, b_z, 0.0));
            dot_tangent_b2 = dot_tangent_b * dot_tangent_b;
            
            x_term = dot_tangent_b2 * one_over_param_x2;

            param_y = param_y * 3.0;
            param_y2 = param_y * param_y;
            one_over_param_y2 = 1.0 / param_y2;

            one_minus_dot_tangent_b2 = 1.0 - dot_tangent_b2;
            y_term = one_minus_dot_tangent_b2 * one_over_param_y2;

            xy_terms = x_term + y_term;

            spec = dot_eye_n2_minus_one * one_over_dot_eye_n2;
            spec = spec * xy_terms;
            spec = spec * 1.442695;
            spec = exp2(spec);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_blinn_phong_spec_anisotropic<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_BLINN_PHONG_SPEC_ANISOTROPIC)?;

    Some((
        Operation::AnisotropicSpecular,
        vec![
            result.get("normal_x")?,
            result.get("normal_y")?,
            result.get("normal_z")?,
            result.get("tangent_x")?,
            result.get("tangent_y")?,
            result.get("tangent_z")?,
            result.get("eye_x")?,
            result.get("eye_y")?,
            result.get("eye_z")?,
            result.get("param_x")?,
            result.get("param_y")?,
        ],
    ))
}

static OP_FRESNEL: LazyLock<Graph> = LazyLock::new(|| {
    // Fresnel shading using MC.fresnelParams.x as the exponent from texas_cross.64.frag.
    let query = indoc! {"
        void main() {
            dot_product = dot(vec4(eye_x, eye_y, eye_z, 0.0), vec4(n_x, n_y, n_z, 0.0));
            dot_product = clamp(dot_product, 0.0, 1.0);
            fresnel = -dot_product + 1.0;
            exponent = param + 1.0;
            fresnel = log2(fresnel);
            fresnel = exponent * fresnel;
            fresnel = exp2(fresnel);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_fresnel<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_FRESNEL)?;
    Some((
        Operation::Fresnel,
        vec![
            result.get("n_x")?,
            result.get("n_y")?,
            result.get("n_z")?,
            result.get("eye_x")?,
            result.get("eye_y")?,
            result.get("eye_z")?,
            result.get("param")?,
        ],
    ))
}

fn fragment_tangent_query(a0: char, a1: char, b0: char, b1: char) -> String {
    // tangent = cross(bitangent, normal) from texas_cross.105.frag.
    // cross(a, b).{c} has the form a0*b0 - b1*a1.
    formatdoc! {"
        void main() {{
            inv_b_length = inversesqrt(b_length);
            b0 = b.{b0} * inv_b_length;
            b1 = b.{b1} * inv_b_length;

            inv_a_length = inversesqrt(a_length);
            a0 = a.{a0} * inv_a_length;
            a1 = a.{a1} * inv_a_length;

            a0_b0 = a0 * b0;
            tangent = fma(-b1, a1, a0_b0);
            result = bitangent_sign * tangent;
        }}
    "}
}

static FRAGMENT_TANGENT_X: LazyLock<Graph> = LazyLock::new(|| {
    // tangent = cross(bitangent, normal) from texas_cross.105.frag.
    // cross(a, b).x = a.y * b.z - b.y * a.z
    let query = fragment_tangent_query('y', 'z', 'z', 'y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static FRAGMENT_TANGENT_Y: LazyLock<Graph> = LazyLock::new(|| {
    // tangent = cross(bitangent, normal) from texas_cross.105.frag.
    // cross(a, b).y = a.z * b.x - b.z * a.x
    let query = fragment_tangent_query('z', 'x', 'x', 'z');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static FRAGMENT_TANGENT_Z: LazyLock<Graph> = LazyLock::new(|| {
    // tangent = cross(bitangent, normal) from texas_cross.105.frag.
    // cross(a, b).z = a.x * b.y - b.x * a.y
    let query = fragment_tangent_query('x', 'y', 'y', 'x');
    Graph::parse_glsl(&query).unwrap().simplify()
});

pub fn fragment_tangent(graph: &Graph, expr: &Expr) -> Option<Expr> {
    // tangent = cross(normal, bitangent) from texas_cross.105.frag.
    // TODO: Should this be a separate attribute than the vertex tangent?
    query_nodes(expr, graph, &FRAGMENT_TANGENT_X)
        .map(|_| Expr::Global {
            name: "a_Tangent".into(),
            channel: Some('x'),
        })
        .or_else(|| {
            query_nodes(expr, graph, &FRAGMENT_TANGENT_Y).map(|_| Expr::Global {
                name: "a_Tangent".into(),
                channel: Some('y'),
            })
        })
        .or_else(|| {
            query_nodes(expr, graph, &FRAGMENT_TANGENT_Z).map(|_| Expr::Global {
                name: "a_Tangent".into(),
                channel: Some('z'),
            })
        })
}

static OP_TINT_COLOR: LazyLock<Graph> = LazyLock::new(|| {
    // Color tint from diffuse color from texas_cross.105.frag.
    // The amount is typically controlled by alpha like NU_specularColor.a.
    let query = indoc! {"
        void main() {
            max_component = max(color_x, color_y);
            max_component = max(color_z, max_component);
            result = color - max_component;
            result = fma(result, amount, 1.0);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_tint_color<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_TINT_COLOR)?;

    let x = result.get("color_x")?;
    let y = result.get("color_y")?;
    let z = result.get("color_z")?;
    let color = result.get("color")?;
    let amount = result.get("amount")?;

    let op = if color == x {
        Operation::TintColorX
    } else if color == y {
        Operation::TintColorY
    } else if color == z {
        Operation::TintColorZ
    } else {
        return None;
    };

    Some((op, vec![x, y, z, amount]))
}

static OP_EFFECT_LIGHT: LazyLock<Graph> = LazyLock::new(|| {
    // Add enough code to remove unsupported effect light integer indexing code.
    // TODO: Is this some sort of point lighting?
    let query = indoc! {"
        void main() {
            cond = cond >= 0.0 ? 1.0 : 0.0;
            unk0 = unk0 * cond;
            unk0 = unk0 * unk_clamp;

            unk_dot = dot(vec4(a_x, a_y, a_z, 0.0), vec4(b_x, b_y, b_z, 0.0));
            unk_dot = clamp(unk_dot, 0.0, 1.0);

            result = fma(unk0, unk_dot, unk1);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_effect_light<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // TODO: create an operation enum variant for effect lighting.
    let _result = query_nodes(expr, graph, &OP_EFFECT_LIGHT)?;
    Some((Operation::Unk, vec![]))
}
