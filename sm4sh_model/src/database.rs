use std::{collections::BTreeMap, path::Path};

use binrw::BinResult;
use indexmap::IndexMap;
use smol_str::SmolStr;
use strum::FromRepr;
pub use xc3_shader::expr::{Attribute, OutputExpr, Parameter, Texture, Value};

use crate::database::uniforms::uniform_parameter_value;

mod io;
// TODO: Find a nicer way to handle uniform buffers.
mod uniforms;

#[derive(Debug, PartialEq, Clone)]
pub struct ShaderDatabase(io::ShaderDatabaseIndexed);

#[derive(Debug, PartialEq, Clone)]
pub struct ShaderProgram {
    /// Indices into [exprs](#structfield.exprs) for values assigned to a fragment output.
    pub output_dependencies: IndexMap<SmolStr, usize>,

    /// Unique exprs used for this program.
    pub exprs: Vec<OutputExpr<Operation>>,

    // Used for validation.
    pub attributes: Vec<SmolStr>,
    pub samplers: Vec<SmolStr>,
    pub parameters: Vec<SmolStr>,
}

impl ShaderDatabase {
    /// Load the database data from `path`.
    pub fn from_file<P: AsRef<Path>>(path: P) -> BinResult<Self> {
        // Keep the indexed database to improve load times and reduce memory usage.
        io::ShaderDatabaseIndexed::from_file(path).map(Self)
    }

    /// Serialize and save the database data to `path`.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> BinResult<()> {
        self.0.save(path)?;
        Ok(())
    }

    pub fn get_shader(&self, shader_id: u32) -> Option<ShaderProgram> {
        self.0.get_shader(shader_id)
    }

    /// Create the internal database representation from non indexed data.
    pub fn from_programs(programs: BTreeMap<u32, ShaderProgram>) -> Self {
        Self(io::ShaderDatabaseIndexed::from_programs(programs))
    }
}

impl ShaderProgram {
    pub fn parameter_value(&self, parameter: &Parameter) -> Option<f32> {
        // TODO: Is there a better way to pass global parameters to consumers like Python?
        uniform_parameter_value(self, parameter)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, FromRepr)]
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
