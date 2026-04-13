// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! CP-SAT optimization model for city selection.
//!
//! Selects the best city within budget and talent constraints,
//! maximizing a weighted composite score.

use converge_optimization::cp::{CpModel, CpStatus, IntVarId};

use crate::spike_market::scenario::CandidateCity;

/// Result of the city optimization.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CityOptResult {
    /// Index of the selected city in the candidates list.
    pub selected_index: usize,
    /// Name of the selected city.
    pub city_name: String,
    /// Objective value (scaled weighted score).
    pub objective_value: i64,
    /// Solve status.
    pub status: CpStatus,
    /// Solve wall time in seconds.
    pub wall_time: f64,
}

/// Scale factor for converting float scores to integer coefficients.
/// We multiply by 100 to preserve 2 decimal places.
const SCALE: i64 = 100;

/// Run the CP-SAT optimization to select the best city.
///
/// - Variables: 8 boolean vars (one per city)
/// - Constraints: exactly 1 selected, entry cost ≤ budget, talent ≥ minimum
/// - Objective: maximize weighted score (0.3×talent + 0.25×market + 0.25×tax + 0.2×cost_eff)
///
/// If `score_overrides` is provided, those scores replace the city's `weighted_score()`.
pub fn optimize_city_selection(
    cities: &[CandidateCity],
    budget_k: u32,
    min_talent: u32,
    score_overrides: Option<&[f64]>,
) -> CityOptResult {
    let n = cities.len();
    let mut model = CpModel::new();

    // Create boolean variable per city
    let vars: Vec<IntVarId> = (0..n)
        .map(|i| model.new_bool_var(&cities[i].name))
        .collect();

    // Constraint: select exactly 1 city
    let ones: Vec<i64> = vec![1; n];
    model.add_linear_eq(&vars, &ones, 1);

    // Constraint: entry cost ≤ budget
    let costs: Vec<i64> = cities.iter().map(|c| i64::from(c.entry_cost_k)).collect();
    model.add_linear_le(&vars, &costs, i64::from(budget_k));

    // Constraint: selected city talent ≥ minimum
    // For bool vars: sum(talent_i * x_i) >= min_talent
    let talents: Vec<i64> = cities.iter().map(|c| i64::from(c.talent_score)).collect();
    model.add_linear_ge(&vars, &talents, i64::from(min_talent));

    // Objective: maximize weighted score
    let scores: Vec<i64> = if let Some(overrides) = score_overrides {
        overrides
            .iter()
            .map(|s| (s * SCALE as f64) as i64)
            .collect()
    } else {
        cities
            .iter()
            .map(|c| (c.weighted_score() * SCALE as f64) as i64)
            .collect()
    };
    model.maximize(&vars, &scores);

    let solution = model.solve();

    if solution.status.is_success() {
        // Find selected city
        let selected = vars
            .iter()
            .enumerate()
            .find(|(_, v)| solution.value(**v) == 1)
            .map(|(i, _)| i)
            .unwrap_or(0);

        CityOptResult {
            selected_index: selected,
            city_name: cities[selected].name.clone(),
            objective_value: solution.objective_value.unwrap_or(0),
            status: solution.status,
            wall_time: solution.wall_time,
        }
    } else {
        CityOptResult {
            selected_index: 0,
            city_name: String::new(),
            objective_value: 0,
            status: solution.status,
            wall_time: solution.wall_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spike_market::scenario::{MAX_BUDGET_K, MIN_TALENT_SCORE, candidate_cities};

    #[test]
    fn selects_optimal_city_within_constraints() {
        let cities = candidate_cities();
        let result = optimize_city_selection(&cities, MAX_BUDGET_K, MIN_TALENT_SCORE, None);

        assert_eq!(result.status, CpStatus::Optimal);
        assert!(!result.city_name.is_empty(), "should select a city");

        // Verify the selected city meets constraints
        let selected = &cities[result.selected_index];
        assert!(
            selected.entry_cost_k <= MAX_BUDGET_K,
            "selected city {} has cost {}K > budget {}K",
            selected.name,
            selected.entry_cost_k,
            MAX_BUDGET_K
        );
        assert!(
            selected.talent_score >= MIN_TALENT_SCORE,
            "selected city {} has talent {} < min {}",
            selected.name,
            selected.talent_score,
            MIN_TALENT_SCORE
        );
    }

    #[test]
    fn infeasible_with_impossible_budget() {
        let cities = candidate_cities();
        // Budget of 100K — no city qualifies
        let result = optimize_city_selection(&cities, 100, MIN_TALENT_SCORE, None);
        assert_eq!(result.status, CpStatus::Infeasible);
    }

    #[test]
    fn respects_score_overrides() {
        let cities = candidate_cities();
        // Override: give Dublin the highest score
        let mut overrides = vec![0.0; cities.len()];
        let dublin_idx = cities.iter().position(|c| c.name == "Dublin").unwrap();
        overrides[dublin_idx] = 99.0;

        let result =
            optimize_city_selection(&cities, MAX_BUDGET_K, MIN_TALENT_SCORE, Some(&overrides));

        assert_eq!(result.status, CpStatus::Optimal);
        assert_eq!(result.city_name, "Dublin");
    }

    #[test]
    fn zurich_excluded_at_strict_budget() {
        let cities = candidate_cities();
        // Budget 499K — Zurich (500K) should be excluded
        let result = optimize_city_selection(&cities, 499, MIN_TALENT_SCORE, None);

        assert_eq!(result.status, CpStatus::Optimal);
        assert_ne!(
            result.city_name, "Zurich",
            "Zurich at 500K should be excluded with 499K budget"
        );
    }

    #[test]
    fn weighted_scores_are_consistent() {
        let cities = candidate_cities();
        // Verify that the optimization picks the city with the highest feasible weighted score
        let result = optimize_city_selection(&cities, MAX_BUDGET_K, MIN_TALENT_SCORE, None);

        let selected_score = cities[result.selected_index].weighted_score();

        for (i, city) in cities.iter().enumerate() {
            if i != result.selected_index
                && city.entry_cost_k <= MAX_BUDGET_K
                && city.talent_score >= MIN_TALENT_SCORE
            {
                assert!(
                    city.weighted_score() <= selected_score + 0.01,
                    "city {} (score {:.2}) should not beat selected {} (score {:.2})",
                    city.name,
                    city.weighted_score(),
                    result.city_name,
                    selected_score
                );
            }
        }
    }
}
