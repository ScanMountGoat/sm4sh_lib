use glam::Vec4;
use half::f16;
use log::error;

use crate::vertex::BoneElementType;

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
        element_type: BoneElementType,
    ) -> Self {
        let mut influence_counts = vec![0; vertex_count];
        let mut bone_indices = vec![[0; 4]; vertex_count];
        let mut bone_weights = vec![Vec4::ZERO; vertex_count];

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
                        bone_weights[i][influence_counts[i]] = weight.weight;
                        influence_counts[i] += 1;
                    }
                }
            } else {
                // TODO: This can result in bone names not working?
                error!("Influence {:?} not found in skeleton.", influence.bone_name);
            }
        }

        // In game weights are usually in descending order by weight.
        for (is, ws) in bone_indices.iter_mut().zip(bone_weights.iter_mut()) {
            let mut permutation = [0, 1, 2, 3];
            permutation.sort_by_key(|i| ordered_float::OrderedFloat::from(-ws[*i]));

            *is = permutation.map(|i| is[i]);
            *ws = permutation.map(|i| ws[i]).into();
        }

        // Linear blend skinning requires the weights to be normalized.
        // Types other than f32 require special logic to ensure decoded weights are still normalized.
        match element_type {
            BoneElementType::Float32 => {
                for weights in &mut bone_weights {
                    let weight_sum = weights.element_sum();
                    if weight_sum > 0.0 {
                        *weights /= weight_sum;
                    }
                }
            }
            BoneElementType::Float16 => {
                for weights in &mut bone_weights {
                    let f16_weights = weights.to_array().map(f16::from_f32);
                    let weight_sum: f16 = f16_weights.into_iter().sum();
                    if weight_sum.to_f32() > 0.0 {
                        *weights = f16_weights.map(|f| (f / weight_sum).to_f32()).into();
                    }
                }
            }
            BoneElementType::Byte => {
                for weights in &mut bone_weights {
                    // Normalize the integ integers with the remainder since we use uint8 for the vertex buffer.
                    // https://stackoverflow.com/questions/31121591/normalizing-integers
                    let mut u8_weights = weights.to_array().map(|f| (f * 255.0) as u8);
                    let weight_sum: u32 = u8_weights.into_iter().map(|u| u as u32).sum();
                    if weight_sum > 0 {
                        let mut remainder = 0;
                        for weight in &mut u8_weights {
                            let new_weight = *weight as u32 * 255 + remainder;
                            *weight = (new_weight / weight_sum) as u8;
                            remainder = new_weight % weight_sum;
                        }
                        *weights = u8_weights.map(|u| u as f32 / 255.0).into();
                    }
                }
            }
        }

        Self {
            bone_indices,
            bone_weights,
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
            SkinWeights::from_influences(&[], 3, &["a", "b", "c"], BoneElementType::Float32)
        );
    }

    #[test]
    fn bone_indices_weights_multiple_influences() {
        assert_eq!(
            SkinWeights {
                bone_indices: vec![[0, 0, 0, 0], [0, 0, 0, 0], [0, 1, 0, 0]],
                bone_weights: vec![
                    vec4(1.0, 0.0, 0.0, 0.0),
                    vec4(0.0, 0.0, 0.0, 0.0),
                    vec4(0.7, 0.3, 0.0, 0.0)
                ],
            },
            SkinWeights::from_influences(
                &[
                    Influence {
                        bone_name: "a".to_string(),
                        weights: vec![
                            VertexWeight {
                                vertex_index: 0,
                                weight: 0.8
                            },
                            VertexWeight {
                                vertex_index: 2,
                                weight: 0.7
                            }
                        ]
                    },
                    Influence {
                        bone_name: "b".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 0,
                            weight: 0.0
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
                            weight: 1.0
                        }]
                    }
                ],
                3,
                &["a", "c", "b"],
                BoneElementType::Float32
            )
        );
    }

    #[test]
    fn bone_indices_weights_normalize_f32() {
        assert_eq!(
            SkinWeights {
                bone_indices: vec![[0, 0, 0, 0], [1, 2, 0, 0]],
                bone_weights: vec![vec4(1.0, 0.0, 0.0, 0.0), vec4(0.5, 0.5, 0.0, 0.0),],
            },
            SkinWeights::from_influences(
                &[
                    Influence {
                        bone_name: "a".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 0,
                            weight: 0.5
                        },]
                    },
                    Influence {
                        bone_name: "b".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 1,
                            weight: 1.0
                        }]
                    },
                    Influence {
                        bone_name: "c".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 1,
                            weight: 1.0
                        }]
                    },
                ],
                2,
                &["a", "b", "c"],
                BoneElementType::Float32
            )
        );
    }

    #[test]
    fn bone_indices_weights_normalize_f16() {
        assert_eq!(
            SkinWeights {
                bone_indices: vec![[0, 0, 0, 0], [1, 2, 0, 0]],
                bone_weights: vec![vec4(1.0, 0.0, 0.0, 0.0), vec4(0.5, 0.5, 0.0, 0.0),],
            },
            SkinWeights::from_influences(
                &[
                    Influence {
                        bone_name: "a".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 0,
                            weight: 0.5
                        },]
                    },
                    Influence {
                        bone_name: "b".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 1,
                            weight: 1.0
                        }]
                    },
                    Influence {
                        bone_name: "c".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 1,
                            weight: 1.0
                        }]
                    },
                ],
                2,
                &["a", "b", "c"],
                BoneElementType::Float16
            )
        );
    }

    #[test]
    fn bone_indices_weights_normalize_u8() {
        // Weights should sum to 255.0 / 255.0.
        assert_eq!(
            SkinWeights {
                bone_indices: vec![[2, 1, 0, 0], [1, 2, 0, 0]],
                bone_weights: vec![
                    vec4(127.0 / 255.0, 85.0 / 255.0, 43.0 / 255.0, 0.0),
                    vec4(127.0 / 255.0, 128.0 / 255.0, 0.0, 0.0),
                ],
            },
            SkinWeights::from_influences(
                &[
                    Influence {
                        bone_name: "a".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 0,
                            weight: 0.25
                        },]
                    },
                    Influence {
                        bone_name: "b".to_string(),
                        weights: vec![
                            VertexWeight {
                                vertex_index: 0,
                                weight: 0.5
                            },
                            VertexWeight {
                                vertex_index: 1,
                                weight: 1.0
                            }
                        ]
                    },
                    Influence {
                        bone_name: "c".to_string(),
                        weights: vec![
                            VertexWeight {
                                vertex_index: 0,
                                weight: 0.75
                            },
                            VertexWeight {
                                vertex_index: 1,
                                weight: 1.0
                            }
                        ]
                    },
                ],
                2,
                &["a", "b", "c"],
                BoneElementType::Byte
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
