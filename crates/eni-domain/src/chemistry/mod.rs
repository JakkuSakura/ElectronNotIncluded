//! Chemistry model: substance properties, per-tile mixtures, and reactions.

mod reaction;
mod substance;
mod tile;

pub use reaction::{ReactionRule, try_react};
pub use substance::{Phase, SubstanceDefinition, SubstanceId};
pub use tile::Composition;

use std::collections::HashMap;
use std::path::Path;

use crate::data::DataError;

/// Lookup table of all known substances, keyed by id.
#[derive(Clone, Debug, Default)]
pub struct SubstanceRegistry(HashMap<SubstanceId, SubstanceDefinition>);

impl SubstanceRegistry {
    pub fn get(&self, id: &SubstanceId) -> Option<&SubstanceDefinition> {
        self.0.get(id)
    }

    pub fn contains(&self, id: &SubstanceId) -> bool {
        self.0.contains_key(id)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&SubstanceId, &SubstanceDefinition)> {
        self.0.iter()
    }

    /// Build a registry directly from definitions, validating unique ids.
    /// Used by both `load` and tests.
    pub fn from_definitions(defs: Vec<SubstanceDefinition>) -> Result<Self, DataError> {
        let mut map = HashMap::new();
        for def in defs {
            if map.insert(def.id.clone(), def).is_some() {
                return Err(DataError::DuplicateId {
                    path: "<in-memory>".to_string(),
                });
            }
        }
        if map.is_empty() {
            return Err(DataError::Empty {
                path: "<in-memory>".to_string(),
            });
        }
        Ok(Self(map))
    }

    /// Load substance definitions from a JSON file (array of `SubstanceDefinition`).
    pub fn load(path: impl AsRef<Path>) -> Result<Self, DataError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| DataError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let defs: Vec<SubstanceDefinition> =
            serde_json::from_str(&text).map_err(|source| DataError::Parse {
                path: path.display().to_string(),
                source,
            })?;

        let mut map = HashMap::new();
        for def in defs {
            if map.insert(def.id.clone(), def).is_some() {
                return Err(DataError::DuplicateId {
                    path: path.display().to_string(),
                });
            }
        }
        if map.is_empty() {
            return Err(DataError::Empty {
                path: path.display().to_string(),
            });
        }
        Ok(Self(map))
    }
}
