//! Data-driven game content: substances and reactions, loaded from JSON so
//! new chemistry can be added without recompiling.

use std::path::Path;

use bevy::prelude::Resource;
use thiserror::Error;

use crate::chemistry::{ReactionRule, SubstanceRegistry};

#[derive(Debug, Error)]
pub enum DataError {
    #[error("failed to read data file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse data file {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("data file {path} contains a duplicate id")]
    DuplicateId { path: String },
    #[error("data file {path} is empty")]
    Empty { path: String },
    #[error("reaction in {path} references unknown substance id `{id}`")]
    UnknownSubstance { path: String, id: String },
}

/// All statically loaded game content.
#[derive(Clone, Debug, Default, Resource)]
pub struct GameData {
    pub substances: SubstanceRegistry,
    pub reactions: Vec<ReactionRule>,
}

impl GameData {
    /// Load `substances.json` and `reactions.json` from `data_dir` (typically
    /// `assets/data`), validating that every reaction references known
    /// substance ids.
    pub fn load(data_dir: impl AsRef<Path>) -> Result<Self, DataError> {
        let dir = data_dir.as_ref();
        let substances = SubstanceRegistry::load(dir.join("substances.json"))?;

        let reactions_path = dir.join("reactions.json");
        let text = std::fs::read_to_string(&reactions_path).map_err(|source| DataError::Io {
            path: reactions_path.display().to_string(),
            source,
        })?;
        let reactions: Vec<ReactionRule> =
            serde_json::from_str(&text).map_err(|source| DataError::Parse {
                path: reactions_path.display().to_string(),
                source,
            })?;

        for rule in &reactions {
            for (id, _) in rule.reactants.iter().chain(rule.products.iter()) {
                if !substances.contains(id) {
                    return Err(DataError::UnknownSubstance {
                        path: reactions_path.display().to_string(),
                        id: id.0.clone(),
                    });
                }
            }
        }

        Ok(Self {
            substances,
            reactions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_data_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data")
    }

    #[test]
    fn loads_bundled_substances_and_reactions() {
        let data = GameData::load(workspace_data_dir()).expect("bundled data must load");
        assert!(!data.substances.is_empty());
    }
}
