//! Static properties of a substance, loaded from `assets/data/substances.json`.

use serde::Deserialize;

/// Newtype identifier for a substance (e.g. "water", "nacl").
///
/// Kept as a plain string wrapper rather than an enum so new substances can be
/// added purely through data files, without a code change.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub struct SubstanceId(pub String);

impl SubstanceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for SubstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
}

/// Physical properties of a substance at standard conditions.
///
/// Values are real-world approximations (SI units) so the simulation behaves
/// plausibly even though it is not a full physics engine.
#[derive(Clone, Debug, Deserialize)]
pub struct SubstanceDefinition {
    pub id: SubstanceId,
    pub name: String,
    pub phase_at_stp: Phase,
    pub density_kg_per_m3: f32,
    pub specific_heat_j_per_kg_k: f32,
    pub melting_point_k: f32,
    pub boiling_point_k: f32,
    pub thermal_conductivity: f32,
}
