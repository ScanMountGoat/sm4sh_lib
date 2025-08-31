use glsl_lang::ast::TranslationUnit;
use xc3_shader::graph::{
    Graph,
    glsl::{find_attribute_locations, merge_vertex_fragment},
};

pub fn shader_from_glsl(vertex: Option<&TranslationUnit>, fragment: &TranslationUnit) {
    let frag = Graph::from_glsl(fragment);
    let frag_attributes = find_attribute_locations(fragment);

    let vertex = vertex.map(|v| (Graph::from_glsl(v), find_attribute_locations(v)));
    let (vert, vert_attributes) = vertex.unzip();

    // Create a combined graph that links vertex outputs to fragment inputs.
    // This effectively moves all shader logic to the fragment shader.
    // This simplifies generating shader code or material nodes in 3D applications.
    let graph = if let (Some(vert), Some(vert_attributes)) = (vert, vert_attributes) {
        merge_vertex_fragment(vert, &vert_attributes, frag, &frag_attributes)
    } else {
        frag
    };

    // TODO: convert to outputexpr
}
