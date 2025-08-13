use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ShaderDatabase {
    pub programs: BTreeMap<String, ShaderProgram>,
}

#[derive(Serialize, Deserialize)]
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
}
