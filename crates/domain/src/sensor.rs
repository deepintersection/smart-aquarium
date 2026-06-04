// crates/domain/src/sensor.rs
//
// DDD CONCEPT: VALUE OBJECT
// A "SensorReading" is a Value Object — it has no identity of its own,
// it is defined entirely by its VALUES (what it measures + when).
// Two readings with the same temperature at the same time are identical.
//
// In contrast, an "Aquarium" would be an ENTITY — it has a unique ID
// and persists over time even as its properties change.
//
// We start with the simplest possible model: a temperature reading.
// No generics, no traits, just a plain struct with a value and a unit.

// ---------------------------------------------------------------------------
// SENSOR TYPE
// ---------------------------------------------------------------------------

/// Represents what kind of physical sensor took a measurement.
///
/// In Rust, an `enum` (enumeration) is a type that can be one of
/// several named variants. This is perfect for listing sensor kinds.
///
/// `#[derive(...)]` automatically generates useful code for us:
///   • `Debug`   — lets us print the value with {:?} for debugging
///   • `Clone`   — lets us make a copy with .clone()
///   • `Copy`    — lets Rust copy small values automatically (no .clone() needed)
///   • `PartialEq` — lets us compare with == and !=
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SensorKind {
    /// Measures water temperature (°C)
    Temperature,
    /// Measures water acidity/alkalinity (0–14 scale)
    Ph,
    /// Measures dissolved oxygen in water (mg/L)
    DissolvedOxygen,
    /// Measures how cloudy the water is (NTU units)
    Turbidity,
    /// Measures dissolved salts (µS/cm)
    Conductivity,
}

// ---------------------------------------------------------------------------
// MEASUREMENT UNIT
// ---------------------------------------------------------------------------

/// The unit of a sensor measurement.
///
/// Keeping the unit attached to the value prevents bugs like
/// mixing up Celsius and Fahrenheit readings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unit {
    Celsius,
    PhScale,    // dimensionless 0–14
    MilligramPerLiter,   // for dissolved oxygen
    Ntu,        // Nephelometric Turbidity Units
    MicrosiemensPerCm,   // for conductivity
}

// ---------------------------------------------------------------------------
// SENSOR READING — the core Value Object
// ---------------------------------------------------------------------------

/// A single measurement taken by a sensor at a point in time.
///
/// This is the most fundamental concept in our aquarium domain:
/// "at some moment, a sensor measured this value".
///
/// FIELDS:
///   `kind`      — which sensor type produced this reading
///   `value`     — the numeric measurement (e.g. 25.3)
///   `unit`      — what unit the value is in (e.g. Celsius)
///   `timestamp` — when it was taken, in seconds since boot
///                 (we use u64 instead of a real clock for now)
///
/// `pub` means other crates can see and use this struct.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorReading {
    pub kind: SensorKind,
    pub value: f32,        // f32 = 32-bit floating point number
    pub unit: Unit,
    pub timestamp_secs: u64, // seconds since device boot
}

impl SensorReading {
    /// Creates a new SensorReading.
    ///
    /// `impl SensorReading { ... }` is where we write methods for the struct.
    /// This is Rust's way of doing what other languages call a "constructor".
    ///
    /// `fn new(...)` — a "constructor function" by convention in Rust.
    /// It takes the fields as arguments and returns a `Self` (= SensorReading).
    pub fn new(kind: SensorKind, value: f32, unit: Unit, timestamp_secs: u64) -> Self {
        // `Self { ... }` constructs the struct with the given field values.
        Self {
            kind,
            value,
            unit,
            timestamp_secs,
        }
    }

    /// Convenience constructor for a temperature reading.
    ///
    /// Instead of remembering the unit every time, callers just write:
    ///   `SensorReading::temperature(25.3, 1000)`
    pub fn temperature(celsius: f32, timestamp_secs: u64) -> Self {
        Self::new(SensorKind::Temperature, celsius, Unit::Celsius, timestamp_secs)
    }

    /// Convenience constructor for a pH reading.
    pub fn ph(ph_value: f32, timestamp_secs: u64) -> Self {
        Self::new(SensorKind::Ph, ph_value, Unit::PhScale, timestamp_secs)
    }
}

// ---------------------------------------------------------------------------
// VALIDATION ERRORS
// ---------------------------------------------------------------------------

/// Errors that can occur when validating a sensor reading.
///
/// DDD principle: the domain should defend its own invariants.
/// If a temperature of -999°C arrives, we reject it HERE,
/// not in the hardware driver or the HTTP handler.
///
/// `#[derive(Debug, PartialEq)]` so we can assert errors in tests.
#[derive(Debug, PartialEq)]
pub enum SensorError {
    /// The measured value is outside the physically possible range.
    ValueOutOfRange { value: f32, min: f32, max: f32 },
}

impl SensorReading {
    /// Validates that the reading value is within a physically sensible range.
    ///
    /// Returns `Ok(())` if valid, or `Err(SensorError)` if not.
    ///
    /// In Rust, `Result<T, E>` is the standard way to return either
    /// a success value (`Ok(T)`) or an error (`Err(E)`).
    /// `()` means "nothing" — we just want to know if it succeeded.
    pub fn validate(&self) -> Result<(), SensorError> {
        // Define valid ranges for each sensor type.
        // These are real-world physical limits.
        let (min, max) = match self.kind {
            SensorKind::Temperature      => (-10.0, 50.0),   // °C, aquarium range
            SensorKind::Ph               => (0.0,   14.0),   // pH scale
            SensorKind::DissolvedOxygen  => (0.0,   20.0),   // mg/L
            SensorKind::Turbidity        => (0.0, 1000.0),   // NTU
            SensorKind::Conductivity     => (0.0, 5000.0),   // µS/cm
        };

        // Check if the value falls outside the allowed range.
        if self.value < min || self.value > max {
            // Return an error with details about what went wrong.
            return Err(SensorError::ValueOutOfRange {
                value: self.value,
                min,
                max,
            });
        }

        // Everything is fine — return Ok with "nothing" inside.
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// TESTS
// ---------------------------------------------------------------------------
//
// In Rust, tests live in the SAME FILE as the code they test.
// The `#[cfg(test)]` attribute means this block is only compiled
// when running `cargo test` — it won't be in the final firmware binary.
//
// Run tests with:  cargo test -p domain
#[cfg(test)]
mod tests {
    // `use super::*` imports everything from the parent module (this file).
    use super::*;

    #[test]
    fn temperature_reading_is_created_correctly() {
        // Arrange: create a temperature reading at 25.5°C, 1000 seconds after boot
        let reading = SensorReading::temperature(25.5, 1000);

        // Assert: check each field has the expected value
        assert_eq!(reading.kind, SensorKind::Temperature);
        assert_eq!(reading.value, 25.5);
        assert_eq!(reading.unit, Unit::Celsius);
        assert_eq!(reading.timestamp_secs, 1000);
    }

    #[test]
    fn valid_temperature_passes_validation() {
        let reading = SensorReading::temperature(26.0, 500);
        // `.is_ok()` returns true if the Result is Ok(...)
        assert!(reading.validate().is_ok());
    }

    #[test]
    fn temperature_above_max_fails_validation() {
        // 999°C is obviously wrong for an aquarium sensor
        let reading = SensorReading::temperature(999.0, 500);
        assert_eq!(
            reading.validate(),
            Err(SensorError::ValueOutOfRange {
                value: 999.0,
                min: -10.0,
                max: 50.0,
            })
        );
    }

    #[test]
    fn ph_reading_is_created_correctly() {
        let reading = SensorReading::ph(7.2, 200);
        assert_eq!(reading.kind, SensorKind::Ph);
        assert_eq!(reading.value, 7.2);
        assert_eq!(reading.unit, Unit::PhScale);
    }

    #[test]
    fn ph_out_of_range_fails_validation() {
        // pH cannot exceed 14
        let reading = SensorReading::ph(15.0, 200);
        assert!(reading.validate().is_err());
    }
}
