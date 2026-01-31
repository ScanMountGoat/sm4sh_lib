use std::{collections::BTreeMap, path::Path};

use binrw::BinResult;
use smol_str::SmolStr;
use strum::FromRepr;
pub use xc3_shader::expr::{Attribute, OutputExpr, Parameter, Texture, Value};

use crate::database::uniforms::uniform_parameter_value;

mod io;
// TODO: Find a nicer way to handle uniform buffers.
mod uniforms;

// Faster than the default hash implementation.
type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

#[derive(Debug, PartialEq, Clone)]
pub struct ShaderDatabase {
    programs: BTreeMap<u32, ShaderProgram>,
}

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
        // Store non indexed programs to avoid converting an index program more than once.
        let indexed = io::ShaderDatabaseIndexed::from_file(path)?;
        Ok(Self {
            programs: indexed.programs(),
        })
    }

    /// Serialize and save the database data to `path`.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> BinResult<()> {
        let indexed = io::ShaderDatabaseIndexed::from_programs(&self.programs);
        indexed.save(path)?;
        Ok(())
    }

    pub fn get_shader(&self, shader_id: u32) -> Option<&ShaderProgram> {
        self.programs.get(&shader_id)
    }

    /// Create the internal database representation from non indexed data.
    pub fn from_programs(programs: BTreeMap<u32, ShaderProgram>) -> Self {
        Self { programs }
    }
}

impl ShaderProgram {
    pub fn parameter_value(&self, parameter: &Parameter) -> Option<f32> {
        // TODO: Is there a better way to pass global parameters to consumers like Python?
        uniform_parameter_value(self, parameter)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, FromRepr, Default)]
pub enum Operation {
    #[default]
    Unk,
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
    Dot,
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
    SphereMapCoordX,
    SphereMapCoordY,
    LocalToWorldPointX,
    LocalToWorldPointY,
    LocalToWorldPointZ,
    LocalToWorldVectorX,
    LocalToWorldVectorY,
    LocalToWorldVectorZ,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
