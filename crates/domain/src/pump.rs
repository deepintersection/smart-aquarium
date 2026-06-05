// crates/domain/src/pump.rs
//
// Step 3: Water-change decision domain concepts.
//
// DDD CONCEPTS USED HERE:
//   • VALUE OBJECT  — WaterChangeDecision (defined by its values, not an ID)
//   • DOMAIN SERVICE — decide_water_change() (pure business rule that doesn't
//     belong to a single entity; it sits between WaterQuality and the pump)
//
// REAL-WORLD CONTEXT:
// The aquarium has two bottles connected to peristaltic pumps:
//   • "out" pump  — drains dirty water from the tank into the waste bottle
//   • "in" pump   — pumps fresh water from the supply bottle into the tank
//
// A water change is needed when quality degrades AND enough time has
// passed since the last change (to avoid over-stressing the fish).
//
// This file answers ONE question:
//   "Given what we know about the water and the history of changes,
//    should we run the pumps right now?"
//
// Zero hardware. Zero network. Fully testable on a laptop.

use crate::water::{WaterQualityAssessment, WaterQualityStatus};

// ---------------------------------------------------------------------------
// WATER CHANGE DECISION — the result of the domain service
// ---------------------------------------------------------------------------

/// What the system should do about water at this moment.
///
/// A `WaterChangeDecision` is a VALUE OBJECT: it is immutable and has no
/// identity of its own. Two decisions with the same variant are equal.
///
/// # Variants (ordered from least to most urgent)
///
/// - `NotNeeded`  — quality is fine, no action required
/// - `Recommended` — quality is degrading; a change would help but isn't urgent
/// - `Required`   — quality is at a critical level; pump must run now
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WaterChangeDecision {
    /// Water quality is acceptable. No change needed.
    NotNeeded,
    /// Water quality is outside preferred range.
    /// A change is recommended but not immediately critical.
    Recommended,
    /// Water quality is at a critical level.
    /// A water change must happen as soon as possible.
    Required,
}

// ---------------------------------------------------------------------------
// COOLDOWN POLICY — prevents over-frequent water changes
// ---------------------------------------------------------------------------

/// Controls how often water changes are allowed.
///
/// Changing water too frequently stresses fish (temperature shock, pH swings).
/// This policy enforces a minimum gap between changes.
///
/// `min_gap_secs` is in seconds so it works with our timestamp system
/// (seconds since device boot) without requiring a real-time clock yet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CooldownPolicy {
    /// Minimum seconds that must pass between any two water changes.
    /// A typical value: 3600 (one hour).
    pub min_gap_secs: u64,
}

impl CooldownPolicy {
    /// Creates a new CooldownPolicy with the given minimum gap.
    pub fn new(min_gap_secs: u64) -> Self {
        Self { min_gap_secs }
    }

    /// A sensible default: no more than one water change per hour.
    pub fn one_hour() -> Self {
        Self::new(3600)
    }

    /// Returns `true` if enough time has passed since the last change.
    ///
    /// # Arguments
    /// * `last_change_secs` — timestamp of the last water change (seconds
    ///    since boot), or `None` if no change has ever been performed.
    /// * `now_secs`         — current timestamp (seconds since boot).
    ///
    /// `Option<u64>` for `last_change_secs`: `None` means "never changed",
    /// which always passes the cooldown (change is always allowed).
    pub fn is_elapsed(&self, last_change_secs: Option<u64>, now_secs: u64) -> bool {
        match last_change_secs {
            // Never changed before → cooldown has trivially elapsed
            None => true,
            // Changed before → check if enough time has passed
            Some(last) => {
                // Saturating subtraction: if now < last (clock wrap), gives 0
                // rather than panicking or overflowing.
                now_secs.saturating_sub(last) >= self.min_gap_secs
            }
        }
    }
}

// ---------------------------------------------------------------------------
// DECIDE — the core domain service
// ---------------------------------------------------------------------------

/// Decides whether a water change should happen right now.
///
/// This is a **pure domain service**: given the same inputs it always
/// returns the same output, with no side effects.
///
/// # Arguments
/// * `assessment`        — result of `water::assess()` for the current moment
/// * `cooldown`          — policy controlling how often changes are allowed
/// * `last_change_secs`  — when the last water change ran (seconds since boot),
///                         or `None` if no change has ever been performed
/// * `now_secs`          — current time (seconds since boot)
///
/// # Decision logic
///
/// | Water status | Cooldown elapsed? | Decision      |
/// |-------------|-------------------|---------------|
/// | Good        | any               | NotNeeded     |
/// | Warning     | no                | NotNeeded     |
/// | Warning     | yes               | Recommended   |
/// | Critical    | no                | Recommended   |
/// | Critical    | yes               | Required      |
///
/// Even when quality is Critical, if the cooldown has not elapsed we
/// return `Recommended` rather than `Required` — the fish are stressed
/// but another immediate change would make things worse, not better.
#[must_use]
pub fn decide_water_change(
    assessment: &WaterQualityAssessment,
    cooldown: &CooldownPolicy,
    last_change_secs: Option<u64>,
    now_secs: u64,
) -> WaterChangeDecision {
    let elapsed = cooldown.is_elapsed(last_change_secs, now_secs);

    match (assessment.status, elapsed) {
        // Quality is fine → never change regardless of cooldown
        (WaterQualityStatus::Good, _) => WaterChangeDecision::NotNeeded,

        // Warning + cooldown not elapsed → wait, don't stress the fish
        (WaterQualityStatus::Warning, false) => WaterChangeDecision::NotNeeded,

        // Warning + cooldown elapsed → a change would help
        (WaterQualityStatus::Warning, true) => WaterChangeDecision::Recommended,

        // Critical + cooldown not elapsed → urgent but can't change yet safely
        (WaterQualityStatus::Critical, false) => WaterChangeDecision::Recommended,

        // Critical + cooldown elapsed → must change now
        (WaterQualityStatus::Critical, true) => WaterChangeDecision::Required,
    }
}

// ---------------------------------------------------------------------------
// TESTS
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::water::{WaterQualityAssessment, WaterQualityStatus};

    // Helper: build an assessment with a given status (reason doesn't matter
    // for the decide_water_change() logic).
    fn assessment(status: WaterQualityStatus) -> WaterQualityAssessment {
        let reason = match status {
            WaterQualityStatus::Good     => "All good.",
            WaterQualityStatus::Warning  => "pH slightly low.",
            WaterQualityStatus::Critical => "Temperature critical.",
        };
        WaterQualityAssessment { status, reason: reason.to_string() }
    }

    fn one_hour() -> CooldownPolicy { CooldownPolicy::one_hour() }
    const NOW: u64 = 10_000; // arbitrary "current" timestamp

    // --- CooldownPolicy::is_elapsed ---

    #[test]
    fn cooldown_always_elapsed_when_never_changed() {
        // None means no change has ever happened → always allow
        assert!(one_hour().is_elapsed(None, NOW));
    }

    #[test]
    fn cooldown_not_elapsed_when_changed_recently() {
        // Changed 30 minutes ago, policy requires 60 → not elapsed
        let last = NOW - 1_800; // 30 min ago
        assert!(!one_hour().is_elapsed(Some(last), NOW));
    }

    #[test]
    fn cooldown_elapsed_when_enough_time_passed() {
        // Changed 90 minutes ago, policy requires 60 → elapsed
        let last = NOW - 5_400; // 90 min ago
        assert!(one_hour().is_elapsed(Some(last), NOW));
    }

    #[test]
    fn cooldown_elapsed_exactly_at_boundary() {
        // Changed exactly 60 minutes ago → elapsed (>= not just >)
        let last = NOW - 3_600;
        assert!(one_hour().is_elapsed(Some(last), NOW));
    }

    // --- decide_water_change: Good quality ---

    #[test]
    fn good_quality_is_never_needed_regardless_of_cooldown() {
        let a = assessment(WaterQualityStatus::Good);
        // Never changed before (cooldown trivially elapsed)
        assert_eq!(
            decide_water_change(&a, &one_hour(), None, NOW),
            WaterChangeDecision::NotNeeded
        );
        // Changed recently (cooldown not elapsed)
        assert_eq!(
            decide_water_change(&a, &one_hour(), Some(NOW - 60), NOW),
            WaterChangeDecision::NotNeeded
        );
    }

    // --- decide_water_change: Warning quality ---

    #[test]
    fn warning_quality_not_needed_when_cooldown_not_elapsed() {
        let a = assessment(WaterQualityStatus::Warning);
        let last = NOW - 60; // changed 1 minute ago
        assert_eq!(
            decide_water_change(&a, &one_hour(), Some(last), NOW),
            WaterChangeDecision::NotNeeded
        );
    }

    #[test]
    fn warning_quality_recommended_when_cooldown_elapsed() {
        let a = assessment(WaterQualityStatus::Warning);
        // Never changed → cooldown elapsed
        assert_eq!(
            decide_water_change(&a, &one_hour(), None, NOW),
            WaterChangeDecision::Recommended
        );
    }

    // --- decide_water_change: Critical quality ---

    #[test]
    fn critical_quality_recommended_when_cooldown_not_elapsed() {
        let a = assessment(WaterQualityStatus::Critical);
        let last = NOW - 60; // changed 1 minute ago
        assert_eq!(
            decide_water_change(&a, &one_hour(), Some(last), NOW),
            WaterChangeDecision::Recommended
        );
    }

    #[test]
    fn critical_quality_required_when_cooldown_elapsed() {
        let a = assessment(WaterQualityStatus::Critical);
        // Never changed → cooldown elapsed → must change now
        assert_eq!(
            decide_water_change(&a, &one_hour(), None, NOW),
            WaterChangeDecision::Required
        );
    }

    // --- ordering ---

    #[test]
    fn decision_variants_are_ordered_by_urgency() {
        // Required > Recommended > NotNeeded
        assert!(WaterChangeDecision::Required > WaterChangeDecision::Recommended);
        assert!(WaterChangeDecision::Recommended > WaterChangeDecision::NotNeeded);
    }
}
