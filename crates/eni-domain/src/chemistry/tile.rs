//! Per-tile chemical state: a mixture of substances at a shared temperature.

use std::collections::HashMap;

use super::SubstanceRegistry;
use super::substance::SubstanceId;

/// The contents of one tile: a mass of each substance present, plus a single
/// shared temperature (tiles are small enough that we do not model internal
/// temperature gradients).
///
/// There is deliberately no `separate()`/`extract()` method: once two
/// substances mix, mass is merged permanently. Un-mixing (e.g. desalination)
/// must be modeled as an explicit process/reaction elsewhere, never as an
/// inverse of `mix`.
#[derive(Clone, Debug, PartialEq)]
pub struct Composition {
    pub mass_kg: HashMap<SubstanceId, f32>,
    pub temperature_k: f32,
}

impl Default for Composition {
    fn default() -> Self {
        Self {
            mass_kg: HashMap::new(),
            // Ambient room temperature; an "empty" tile is vacuum at ambient temp.
            temperature_k: 293.15,
        }
    }
}

impl Composition {
    pub fn total_mass(&self) -> f32 {
        self.mass_kg.values().sum()
    }

    /// Heat capacity (J/K) of the current contents, used to weight temperature
    /// blending. Substances not present in the registry fall back to water's
    /// approximate specific heat so an unknown substance does not silently
    /// dominate or vanish from the average.
    fn heat_capacity(&self, registry: &SubstanceRegistry) -> f32 {
        self.mass_kg
            .iter()
            .map(|(id, mass)| {
                let specific_heat = registry
                    .get(id)
                    .map(|def| def.specific_heat_j_per_kg_k)
                    .unwrap_or(4186.0);
                mass * specific_heat
            })
            .sum()
    }

    /// Merge `other` into `self`, summing mass per-substance and blending
    /// temperature by heat capacity (not just mass), which is what actually
    /// conserves thermal energy when mixing dissimilar substances.
    pub fn mix(&mut self, other: Composition, registry: &SubstanceRegistry) {
        let self_capacity = self.heat_capacity(registry);
        let other_capacity = other.heat_capacity(registry);
        let total_capacity = self_capacity + other_capacity;

        for (id, mass) in other.mass_kg {
            *self.mass_kg.entry(id).or_insert(0.0) += mass;
        }

        if total_capacity > 0.0 {
            self.temperature_k = (self.temperature_k * self_capacity
                + other.temperature_k * other_capacity)
                / total_capacity;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chemistry::{Phase, SubstanceDefinition};

    fn test_registry() -> SubstanceRegistry {
        SubstanceRegistry::from_definitions(vec![
            SubstanceDefinition {
                id: SubstanceId::new("water"),
                name: "Water".into(),
                phase_at_stp: Phase::Liquid,
                density_kg_per_m3: 1000.0,
                specific_heat_j_per_kg_k: 4186.0,
                melting_point_k: 273.15,
                boiling_point_k: 373.15,
                thermal_conductivity: 0.6,
            },
            SubstanceDefinition {
                id: SubstanceId::new("nacl"),
                name: "Salt".into(),
                phase_at_stp: Phase::Solid,
                density_kg_per_m3: 2170.0,
                specific_heat_j_per_kg_k: 880.0,
                melting_point_k: 1074.0,
                boiling_point_k: 1686.0,
                thermal_conductivity: 6.5,
            },
        ])
        .expect("test registry must build")
    }

    #[test]
    fn mixing_merges_mass_without_losing_either_substance() {
        let registry = test_registry();
        let water_id = SubstanceId::new("water");
        let salt_id = SubstanceId::new("nacl");

        let mut water = Composition {
            mass_kg: HashMap::from([(water_id.clone(), 10.0)]),
            temperature_k: 300.0,
        };
        let salt = Composition {
            mass_kg: HashMap::from([(salt_id.clone(), 1.0)]),
            temperature_k: 350.0,
        };

        water.mix(salt, &registry);

        // Both substances must be present, masses preserved, no "separate" shortcut exists.
        assert_eq!(water.mass_kg.get(&water_id), Some(&10.0));
        assert_eq!(water.mass_kg.get(&salt_id), Some(&1.0));
        assert!((water.total_mass() - 11.0).abs() < 1e-6);
        // Temperature should land strictly between the two inputs.
        assert!(water.temperature_k > 300.0 && water.temperature_k < 350.0);
    }
}
