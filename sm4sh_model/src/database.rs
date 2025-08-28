use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShaderDatabase {
    pub programs: BTreeMap<String, ShaderProgram>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShaderProgram {
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
