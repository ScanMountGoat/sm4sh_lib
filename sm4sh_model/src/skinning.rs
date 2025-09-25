use glam::Vec4;
use log::error;

#[derive(Debug, PartialEq)]
pub struct Influence {
    pub bone_name: String,
    pub weights: Vec<VertexWeight>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VertexWeight {
    pub vertex_index: u32,
    pub weight: f32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SkinWeights {
    pub bone_indices: Vec<[u32; 4]>,
    pub bone_weights: Vec<Vec4>,
}

impl SkinWeights {
    // TODO: How should this handle of out range indices?
    /// Convert the per-vertex indices and weights to per bone influences.
    ///
    /// The `skeleton` defines the mapping from bone indices to bone names.
    ///
    /// The `bone_names` should match the skinning bone list used for these skin weights.
    pub fn to_influences(&self, bone_names: &[String]) -> Vec<crate::skinning::Influence> {
        let mut influences: Vec<_> = bone_names
            .iter()
            .map(|bone_name| Influence {
                bone_name: bone_name.clone(),
                weights: Vec::new(),
            })
            .collect();

        for (i, (indices, weights)) in self.bone_indices.iter().zip(&self.bone_weights).enumerate()
        {
            for (bone_index, weight) in indices.iter().zip(weights.to_array()) {
                // Skip zero weights since they have no effect.
                if weight > 0.0 {
                    // The vertex attributes use the bone order of the mxmd skeleton.
                    influences[*bone_index as usize].weights.push(VertexWeight {
                        vertex_index: i as u32,
                        weight,
                    });
                }
            }
        }

        influences.retain(|i| !i.weights.is_empty());

        influences
    }

    /// Convert the per-bone `influences` to per-vertex indices and weights.
    ///
    /// The `bone_names` provide the mapping from bone names to bone indices.
    /// Only the first 4 influences for each vertex will be included.
    pub fn from_influences<S: AsRef<str>>(
        influences: &[Influence],
        vertex_count: usize,
        bone_names: &[S],
    ) -> Self {
        let mut influence_counts = vec![0; vertex_count];
        let mut bone_indices = vec![[0; 4]; vertex_count];
        let mut weights = vec![Vec4::ZERO; vertex_count];

        // Assign up to 4 influences to each vertex.
        for influence in influences {
            if let Some(bone_index) = bone_names
                .iter()
                .position(|n| n.as_ref() == influence.bone_name)
            {
                for weight in &influence.weights {
                    let i = weight.vertex_index as usize;
                    // Ignore empty weights since they have no effect.
                    if influence_counts[i] < 4 && weight.weight > 0.0 {
                        bone_indices[i][influence_counts[i]] = bone_index as u32;
                        weights[i][influence_counts[i]] = weight.weight;
                        influence_counts[i] += 1;
                    }
                }
            } else {
                // TODO: This can result in bone names not working?
                error!("Influence {:?} not found in skeleton.", influence.bone_name);
            }
        }

        // In game weights are in ascending order by weight.
        for (is, ws) in bone_indices.iter_mut().zip(weights.iter_mut()) {
            let mut permutation = [0, 1, 2, 3];
            permutation.sort_by_key(|i| ordered_float::OrderedFloat::from(-ws[*i]));

            *is = permutation.map(|i| is[i]);
            *ws = permutation.map(|i| ws[i]).into();
        }

        Self {
            bone_indices,
            bone_weights: weights,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use glam::{Vec4, vec4};

    #[test]
    fn bone_indices_weights_no_influences() {
        assert_eq!(
            SkinWeights {
                bone_indices: vec![[0; 4]; 3],
                bone_weights: vec![Vec4::ZERO; 3],
            },
            SkinWeights::from_influences(&[], 3, &["a", "b", "c"])
        );
    }

    #[test]
    fn bone_indices_weights_multiple_influences() {
        assert_eq!(
            SkinWeights {
                bone_indices: vec![[2, 0, 0, 0], [0, 0, 0, 0], [1, 0, 0, 0]],
                bone_weights: vec![
                    vec4(0.2, 0.0, 0.0, 0.0),
                    vec4(0.0, 0.0, 0.0, 0.0),
                    vec4(0.3, 0.11, 0.0, 0.0)
                ],
            },
            SkinWeights::from_influences(
                &[
                    Influence {
                        bone_name: "a".to_string(),
                        weights: vec![
                            VertexWeight {
                                vertex_index: 0,
                                weight: 0.0
                            },
                            VertexWeight {
                                vertex_index: 2,
                                weight: 0.11
                            }
                        ]
                    },
                    Influence {
                        bone_name: "b".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 0,
                            weight: 0.2
                        }]
                    },
                    Influence {
                        bone_name: "c".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 2,
                            weight: 0.3
                        }]
                    },
                    Influence {
                        bone_name: "d".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 1,
                            weight: 0.4
                        }]
                    }
                ],
                3,
                &["a", "c", "b"]
            )
        );
    }

    #[test]
    fn bone_influences_empty() {
        assert!(
            SkinWeights {
                bone_indices: Vec::new(),
                bone_weights: Vec::new(),
            }
            .to_influences(&[])
            .is_empty()
        );
    }

    #[test]
    fn bone_influences_zero_weights() {
        assert!(
            SkinWeights {
                bone_indices: vec![[0; 4], [0; 4]],
                bone_weights: vec![Vec4::ZERO, Vec4::ZERO],
            }
            .to_influences(&["root".to_string()])
            .is_empty()
        );
    }

    #[test]
    fn bone_influences_multiple_bones() {
        assert_eq!(
            vec![
                Influence {
                    bone_name: "D".to_string(),
                    weights: vec![VertexWeight {
                        vertex_index: 0,
                        weight: 0.2
                    }]
                },
                Influence {
                    bone_name: "C".to_string(),
                    weights: vec![
                        VertexWeight {
                            vertex_index: 0,
                            weight: 0.4
                        },
                        VertexWeight {
                            vertex_index: 1,
                            weight: 0.3
                        }
                    ]
                },
                Influence {
                    bone_name: "B".to_string(),
                    weights: vec![
                        VertexWeight {
                            vertex_index: 0,
                            weight: 0.1
                        },
                        VertexWeight {
                            vertex_index: 1,
                            weight: 0.7
                        }
                    ]
                },
                Influence {
                    bone_name: "A".to_string(),
                    weights: vec![VertexWeight {
                        vertex_index: 0,
                        weight: 0.3
                    }]
                },
            ],
            SkinWeights {
                bone_indices: vec![[3, 1, 2, 0], [2, 1, 0, 0]],
                bone_weights: vec![vec4(0.3, 0.4, 0.1, 0.2), vec4(0.7, 0.3, 0.0, 0.0)],
            }
            .to_influences(&[
                "D".to_string(),
                "C".to_string(),
                "B".to_string(),
                "A".to_string(),
                "unused".to_string()
            ])
        );
    }
}
