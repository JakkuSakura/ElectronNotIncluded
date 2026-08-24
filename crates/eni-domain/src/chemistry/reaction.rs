//! Data-driven reaction rules and resolution.

use serde::Deserialize;

use super::substance::SubstanceId;
use super::tile::Composition;

/// A reaction that converts some reactant masses into product masses, gated
/// by an optional temperature window, releasing or absorbing energy.
#[derive(Clone, Debug, Deserialize)]
pub struct ReactionRule {
    pub reactants: Vec<(SubstanceId, f32)>,
    pub min_temperature_k: Option<f32>,
    pub max_temperature_k: Option<f32>,
    pub products: Vec<(SubstanceId, f32)>,
    pub energy_delta_j_per_kg: f32,
}

/// Try each rule in order against `composition`, returning the resulting
/// composition for the first rule whose reactants and temperature window are
/// satisfied. One reaction "batch" (as specified by the rule's reactant
/// amounts) is consumed per tick; leftover reactant mass simply remains for a
/// future tick.
pub fn try_react(composition: &Composition, rules: &[ReactionRule]) -> Option<Composition> {
    'rule: for rule in rules {
        if let Some(min) = rule.min_temperature_k
            && composition.temperature_k < min
        {
            continue;
        }
        if let Some(max) = rule.max_temperature_k
            && composition.temperature_k > max
        {
            continue;
        }
        for (id, amount) in &rule.reactants {
            let have = composition.mass_kg.get(id).copied().unwrap_or(0.0);
            if have + 1e-9 < *amount {
                continue 'rule;
            }
        }

        let mut result = composition.clone();
        let mut consumed_mass = 0.0f32;
        for (id, amount) in &rule.reactants {
            consumed_mass += amount;
            if let Some(entry) = result.mass_kg.get_mut(id) {
                *entry -= amount;
                if *entry <= 1e-6 {
                    result.mass_kg.remove(id);
                }
            }
        }
        for (id, amount) in &rule.products {
            *result.mass_kg.entry(id.clone()).or_insert(0.0) += amount;
        }

        let total_mass = result.total_mass().max(1e-6);
        result.temperature_k += rule.energy_delta_j_per_kg * consumed_mass / total_mass;

        return Some(result);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn matching_reaction_produces_products_and_consumes_reactants() {
        let hydrogen = SubstanceId::new("hydrogen");
        let oxygen = SubstanceId::new("oxygen");
        let water = SubstanceId::new("water");

        let rule = ReactionRule {
            reactants: vec![(hydrogen.clone(), 2.0), (oxygen.clone(), 16.0)],
            min_temperature_k: None,
            max_temperature_k: None,
            products: vec![(water.clone(), 18.0)],
            energy_delta_j_per_kg: 1000.0,
        };

        let composition = Composition {
            mass_kg: HashMap::from([(hydrogen.clone(), 3.0), (oxygen.clone(), 20.0)]),
            temperature_k: 300.0,
        };

        let result = try_react(&composition, &[rule]).expect("reaction should fire");

        assert_eq!(result.mass_kg.get(&hydrogen), Some(&1.0));
        assert_eq!(result.mass_kg.get(&oxygen), Some(&4.0));
        assert_eq!(result.mass_kg.get(&water), Some(&18.0));
        assert!(result.temperature_k > 300.0);
    }
}
