// crates/domain/src/lib.rs
//
// This is the ROOT FILE of the "domain" crate.
// In Rust, every library crate starts here.
//
// What does "lib.rs" do?
//   It declares which modules (sub-files) exist in this crate.
//   Think of it as the "table of contents" for our domain logic.
//
// DOMAIN-DRIVEN DESIGN STRUCTURE:
// We organise code by DOMAIN CONCEPTS (things in the aquarium world),
// not by technical role (not "models", "utils", "helpers").
//
// Each module below maps to a real concept in our aquarium domain:
//   • sensor   — readings from physical sensors (temperature, pH, etc.)
//   • water    — water quality and water-change logic
//   • lighting — light cycle management
//   • feeding  — feeding schedules
//
// Right now they are all EMPTY PLACEHOLDERS.
// We will fill them one small step at a time.

/// Module for sensor domain concepts.
/// A "sensor" in our domain measures something about the aquarium.
pub mod sensor;

/// Module for water-related domain concepts.
/// Covers water quality thresholds and water-change rules.
pub mod water;

/// Module for pump and water-change decision concepts.
/// Answers: "should we run the pumps right now?"
pub mod pump;
