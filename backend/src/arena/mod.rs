// =============================================================================
// Arena Module — Thompson Sampling Profile Selection (Future Phase 5)
// =============================================================================
//
// The Arena system enables multi-strategy competition using Thompson Sampling
// to dynamically select the best-performing strategy profile for the current
// market regime. This is a stub module — full implementation is planned for
// Phase 5 of the Aurora roadmap.
//
// Architecture:
//   - Each StrategyProfile defines a distinct trading personality (Momentum,
//     MeanRevert, Breakout, Scalp).
//   - Profiles accumulate wins/losses parameterised by a Beta distribution.
//   - Thompson Sampling draws from each profile's posterior and selects the
//     one with the highest sample — a principled explore/exploit approach.

pub mod profile;
