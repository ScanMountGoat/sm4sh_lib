use std::{collections::BTreeMap, path::Path};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
pub use xc3_shader::expr::{Attribute, OutputExpr, Parameter, Texture, Value};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShaderDatabase {
    pub programs: BTreeMap<String, ShaderProgram>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShaderProgram {
    /// Indices into [exprs](#structfield.exprs) for values assigned to a fragment output.
    pub output_dependencies: IndexMap<SmolStr, usize>,

    /// Unique exprs used for this program.
    pub exprs: Vec<OutputExpr<Operation>>,

    pub attributes: BTreeMap<usize, String>,
    pub samplers: BTreeMap<usize, String>,
    pub parameters: BTreeMap<usize, String>,
}

impl ShaderDatabase {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        // TODO: Avoid unwrap.
        let json = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    pub fn get_shader(&self, shader_id: u32) -> Option<&ShaderProgram> {
        self.programs.get(&format!("{shader_id:X?}"))
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
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
