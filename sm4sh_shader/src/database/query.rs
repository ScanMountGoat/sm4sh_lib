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
            R15.w = R1.z + -R3.w;
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

static OP_NORMALIZE: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            length = dot(vec4(x, y, z, w), vec4(x, y, z, w));
            inverse_length = inversesqrt(length);
            result = value * inverse_length;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_normalize<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_NORMALIZE)?;
    let value = result.get("value")?;
    let x = result.get("x")?;
    let y = result.get("y")?;
    let z = result.get("z")?;
    let w = result.get("w")?;

    let op = if value == x {
        Operation::NormalizeX
    } else if value == y {
        Operation::NormalizeY
    } else if value == z {
        Operation::NormalizeZ
    } else {
        return None;
    };

    Some((op, vec![x, y, z, w]))
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
