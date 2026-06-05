// crates/domain/src/water.rs
//
// Step 2: WaterQuality domain concepts.
//
// DDD CONCEPTS USED HERE:
//   • VALUE OBJECT — WaterParameters, WaterThresholds
//     They have no unique ID. Two WaterParameters structs with the same
//     values are considered identical.
//   • DOMAIN SERVICE — the assess() method
//     Business logic that doesn't naturally belong to a single entity.
//
// The key idea: this file answers the question
//   "Is the aquarium water in good condition right now?"
// with ZERO hardware or network code.

use crate::sensor::SensorReading;

// ---------------------------------------------------------------------------
// WATER PARAMETERS — a snapshot of all readings at one moment
// ---------------------------------------------------------------------------

/// A complete snapshot of water conditions at a single point in time.
///
/// Think of this as a "water quality report card":
/// all measurements taken together so we can judge the overall health.
///
/// All fields are `Option<f32>` (optional float) because not every
/// aquarium has every sensor. `None` means "no sensor for this".
/// `Some(25.3)` means "sensor present, value is 25.3".
///
/// In Rust, `Option<T>` is the safe way to say "this might not exist".
/// It forces YOU to handle the missing case — no null pointer crashes!
#[derive(Debug, Clone, PartialEq)]
pub struct WaterParameters {
    /// Water temperature in °C
    pub temperature_celsius: Option<f32>,
    /// pH level (0–14 scale)
    pub ph: Option<f32>,
    /// Dissolved oxygen in mg/L
    pub dissolved_oxygen_mg_l: Option<f32>,
    /// Turbidity (cloudiness) in NTU
    pub turbidity_ntu: Option<f32>,
}

impl WaterParameters {
    /// Creates a WaterParameters with no readings yet.
    ///
    /// Useful as a starting point — then set individual fields.
    pub fn empty() -> Self {
        Self {
            temperature_celsius: None,
            ph: None,
            dissolved_oxygen_mg_l: None,
            turbidity_ntu: None,
        }
    }

    /// Convenience: create parameters with only a temperature reading.
    /// Useful in tests where we only care about one value.
    pub fn with_temperature(celsius: f32) -> Self {
        Self {
            temperature_celsius: Some(celsius),
            ..Self::empty()   // `..` fills all OTHER fields with empty()'s values
        }
    }

    /// Convenience: create a "typical healthy aquarium" snapshot.
    /// Great for tests as a baseline.
    pub fn typical_healthy() -> Self {
        Self {
            temperature_celsius: Some(26.0),
            ph: Some(7.2),
            dissolved_oxygen_mg_l: Some(7.0),
            turbidity_ntu: Some(2.0),
        }
    }

    /// Updates a specific field from a SensorReading.
    ///
    /// This lets us feed raw sensor readings into our parameters
    /// without coupling the sensor layer to the water layer.
    ///
    /// `&mut self` means "modify this struct in place".
    pub fn apply_reading(&mut self, reading: &SensorReading) {
        use crate::sensor::SensorKind;
        // Match on what kind of sensor produced this reading
        match reading.kind {
            SensorKind::Temperature     => self.temperature_celsius = Some(reading.value),
            SensorKind::Ph              => self.ph = Some(reading.value),
            SensorKind::DissolvedOxygen => self.dissolved_oxygen_mg_l = Some(reading.value),
            SensorKind::Turbidity       => self.turbidity_ntu = Some(reading.value),
            // Conductivity doesn't have a field yet — we ignore it for now
            SensorKind::Conductivity    => {}
        }
    }
}

// ---------------------------------------------------------------------------
// WATER THRESHOLDS — what counts as "acceptable" for this aquarium
// ---------------------------------------------------------------------------

/// The acceptable range for one measured parameter.
///
/// If the value falls below `min` or above `max`, it triggers a warning.
/// If it falls below `critical_min` or above `critical_max`, it is critical.
///
/// Example for temperature in a tropical fish tank:
///   critical_min: 18.0  ← fish may die below this
///   min:          24.0  ← getting too cold, take action
///   max:          28.0  ← getting too warm, take action
///   critical_max: 32.0  ← fish may die above this
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterRange {
    pub critical_min: f32,
    pub min: f32,
    pub max: f32,
    pub critical_max: f32,
}

impl ParameterRange {
    /// Creates a new ParameterRange.
    ///
    /// # Panics
    ///
    /// Panics if the ordering invariant `critical_min ≤ min ≤ max ≤ critical_max`
    /// is violated. This is a programming error — ranges are defined by developers
    /// at configuration time, never from live sensor data, so a panic is
    /// appropriate (it surfaces the mistake immediately rather than silently
    /// producing wrong assess() results at runtime).
    ///
    /// ```
    /// // Valid — values are in order:
    /// use domain::water::ParameterRange;
    /// let _r = ParameterRange::new(18.0, 24.0, 28.0, 32.0);
    /// ```
    pub fn new(critical_min: f32, min: f32, max: f32, critical_max: f32) -> Self {
        // Enforce: critical_min ≤ min ≤ max ≤ critical_max.
        // Without this ordering, assess() would silently misclassify readings —
        // e.g. a value inside the "warning" band could appear "critical".
        assert!(
            critical_min <= min && min <= max && max <= critical_max,
            "ParameterRange invariant violated: expected critical_min({}) ≤ min({}) ≤ max({}) ≤ critical_max({})",
            critical_min, min, max, critical_max
        );
        Self { critical_min, min, max, critical_max }
    }
}

/// Thresholds for all water parameters in a specific aquarium setup.
///
/// Different fish species need different conditions.
/// By making thresholds a value object, operators can configure
/// their own ranges without changing any business logic.
#[derive(Debug, Clone, PartialEq)]
pub struct WaterThresholds {
    pub temperature: ParameterRange,
    pub ph: ParameterRange,
    pub dissolved_oxygen: ParameterRange,
    pub turbidity: ParameterRange,
}

impl WaterThresholds {
    /// Default thresholds suitable for a typical tropical freshwater aquarium.
    ///
    /// These are safe starting values — a real operator would tune them
    /// for their specific fish species.
    pub fn tropical_freshwater() -> Self {
        Self {
            temperature: ParameterRange::new(18.0, 24.0, 28.0, 32.0),
            ph:          ParameterRange::new( 5.5,  6.5,  7.5,  8.5),
            dissolved_oxygen: ParameterRange::new(4.0, 6.0, 12.0, 15.0),
            turbidity:   ParameterRange::new( 0.0,  0.0,  5.0, 20.0),
        }
    }
}

// ---------------------------------------------------------------------------
// WATER QUALITY STATUS — the result of assessing parameters vs thresholds
// ---------------------------------------------------------------------------

/// The overall health status of the aquarium water.
///
/// This is the answer to "should I be worried right now?"
///
/// The variants are ordered from best to worst.
/// We derive `PartialOrd` so we can compare: `Critical > Warning > Good`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WaterQualityStatus {
    /// All measured parameters are within acceptable ranges.
    Good,
    /// At least one parameter is outside the preferred range
    /// but not yet at a dangerous level.
    Warning,
    /// At least one parameter is at a dangerous level.
    /// Immediate action recommended.
    Critical,
}

/// The result of assessing water quality.
///
/// Includes the overall status AND which parameter triggered it,
/// so the operator knows exactly what to fix.
#[derive(Debug, Clone, PartialEq)]
pub struct WaterQualityAssessment {
    /// The worst status found across all parameters.
    pub status: WaterQualityStatus,
    /// Human-readable description of what was found.
    /// Uses a `String` (heap-allocated text) for flexibility.
    pub reason: String,
}

impl WaterQualityAssessment {
    fn good() -> Self {
        Self {
            status: WaterQualityStatus::Good,
            reason: "All parameters within acceptable range.".to_string(),
        }
    }

    fn warning(reason: impl Into<String>) -> Self {
        Self { status: WaterQualityStatus::Warning, reason: reason.into() }
    }

    fn critical(reason: impl Into<String>) -> Self {
        Self { status: WaterQualityStatus::Critical, reason: reason.into() }
    }
}

// ---------------------------------------------------------------------------
// ASSESS — the core domain service
// ---------------------------------------------------------------------------

/// Assesses water quality by comparing parameters against thresholds.
///
/// This is a pure function (no side effects, no hardware, no network).
/// Given the same inputs, it always returns the same output.
/// This makes it trivial to test and reason about.
///
/// Returns the WORST status found across all parameters — if any single
/// parameter is Critical, the whole assessment is Critical.
///
/// # Must use
///
/// The result must not be discarded — ignoring it silently means the
/// aquarium is never alerted to a problem. The compiler will warn if you
/// call `assess(...)` without using the returned `WaterQualityAssessment`.
#[must_use]
pub fn assess(params: &WaterParameters, thresholds: &WaterThresholds)
    -> WaterQualityAssessment
{
    // We'll collect the worst status we find as we check each parameter.
    // Start optimistic: assume everything is Good.
    let mut worst = WaterQualityAssessment::good();

    // Helper closure: checks one optional value against one range.
    // A closure is like a mini-function defined inline.
    // `|value, range, name|` are the parameters.
    let mut check = |value: Option<f32>, range: &ParameterRange, name: &str| {
        // If there's no sensor for this parameter, skip it (None → ignore)
        let v = match value {
            Some(v) => v,
            None    => return,   // no sensor present, skip
        };

        // Check critical boundaries first (most severe)
        if v < range.critical_min || v > range.critical_max {
            let msg = format!(
                "{} is CRITICAL: {:.1} (critical range: {:.1}–{:.1})",
                name, v, range.critical_min, range.critical_max
            );
            let candidate = WaterQualityAssessment::critical(msg);
            // Only upgrade if this is worse than what we already have
            if candidate.status > worst.status {
                worst = candidate;
            }
        // Then check warning boundaries
        } else if v < range.min || v > range.max {
            let msg = format!(
                "{} is outside preferred range: {:.1} (preferred: {:.1}–{:.1})",
                name, v, range.min, range.max
            );
            let candidate = WaterQualityAssessment::warning(msg);
            if candidate.status > worst.status {
                worst = candidate;
            }
        }
        // Otherwise: this parameter is Good — no action needed
    };

    // Run the check for each parameter we track
    check(params.temperature_celsius,    &thresholds.temperature,       "Temperature");
    check(params.ph,                      &thresholds.ph,               "pH");
    check(params.dissolved_oxygen_mg_l,  &thresholds.dissolved_oxygen,  "Dissolved oxygen");
    check(params.turbidity_ntu,           &thresholds.turbidity,        "Turbidity");

    worst
}

// ---------------------------------------------------------------------------
// TESTS
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor::SensorReading;

    fn thresholds() -> WaterThresholds {
        WaterThresholds::tropical_freshwater()
    }

    #[test]
    fn healthy_water_is_good() {
        let params = WaterParameters::typical_healthy();
        let result = assess(&params, &thresholds());
        assert_eq!(result.status, WaterQualityStatus::Good);
    }

    #[test]
    fn high_temperature_is_warning() {
        let params = WaterParameters {
            temperature_celsius: Some(30.0), // above max 28°C, below critical 32°C
            ..WaterParameters::typical_healthy()
        };
        let result = assess(&params, &thresholds());
        assert_eq!(result.status, WaterQualityStatus::Warning);
    }

    #[test]
    fn very_high_temperature_is_critical() {
        let params = WaterParameters {
            temperature_celsius: Some(35.0), // above critical_max 32°C
            ..WaterParameters::typical_healthy()
        };
        let result = assess(&params, &thresholds());
        assert_eq!(result.status, WaterQualityStatus::Critical);
    }

    #[test]
    fn low_ph_is_warning() {
        let params = WaterParameters {
            ph: Some(6.0), // below min 6.5, above critical_min 5.5
            ..WaterParameters::typical_healthy()
        };
        let result = assess(&params, &thresholds());
        assert_eq!(result.status, WaterQualityStatus::Warning);
    }

    #[test]
    fn critical_beats_warning_when_multiple_problems() {
        // temperature is Warning, pH is Critical → overall should be Critical
        let params = WaterParameters {
            temperature_celsius: Some(30.0), // Warning
            ph: Some(4.0),                   // Critical (below 5.5)
            ..WaterParameters::typical_healthy()
        };
        let result = assess(&params, &thresholds());
        assert_eq!(result.status, WaterQualityStatus::Critical);
    }

    #[test]
    fn missing_sensor_is_ignored() {
        // Only temperature sensor present, all others None
        let params = WaterParameters::with_temperature(26.0);
        let result = assess(&params, &thresholds());
        assert_eq!(result.status, WaterQualityStatus::Good);
    }

    #[test]
    fn empty_parameters_is_good() {
        // No sensors at all → nothing to fail
        let params = WaterParameters::empty();
        let result = assess(&params, &thresholds());
        assert_eq!(result.status, WaterQualityStatus::Good);
    }

    #[test]
    fn apply_reading_updates_correct_field() {
        let mut params = WaterParameters::empty();
        let reading = SensorReading::temperature(25.5, 1000);
        params.apply_reading(&reading);
        assert_eq!(params.temperature_celsius, Some(25.5));
        assert_eq!(params.ph, None); // untouched
    }

    #[test]
    fn apply_ph_reading_updates_ph_field() {
        let mut params = WaterParameters::empty();
        let reading = SensorReading::ph(7.0, 500);
        params.apply_reading(&reading);
        assert_eq!(params.ph, Some(7.0));
    }

    // --- ParameterRange invariant tests ---

    #[test]
    fn valid_parameter_range_is_created_successfully() {
        // critical_min ≤ min ≤ max ≤ critical_max — must not panic
        let _r = ParameterRange::new(18.0, 24.0, 28.0, 32.0);
    }

    #[test]
    fn parameter_range_allows_equal_adjacent_bounds() {
        // turbidity has critical_min == min == 0.0 — a valid edge case
        let _r = ParameterRange::new(0.0, 0.0, 5.0, 20.0);
    }

    #[test]
    #[should_panic(expected = "ParameterRange invariant violated")]
    fn parameter_range_panics_when_min_below_critical_min() {
        // min < critical_min → ordering broken
        let _r = ParameterRange::new(24.0, 18.0, 28.0, 32.0);
    }

    #[test]
    #[should_panic(expected = "ParameterRange invariant violated")]
    fn parameter_range_panics_when_max_below_min() {
        // max < min → ordering broken
        let _r = ParameterRange::new(18.0, 28.0, 24.0, 32.0);
    }

    #[test]
    #[should_panic(expected = "ParameterRange invariant violated")]
    fn parameter_range_panics_when_critical_max_below_max() {
        // critical_max < max → ordering broken
        let _r = ParameterRange::new(18.0, 24.0, 32.0, 28.0);
    }
}
