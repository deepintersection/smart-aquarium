# 🐟 Smart Aquarium

A smart aquarium controller built with **Rust** running on **ESP32**, using Domain-Driven Design (DDD).

## What does this project do?

- Reads sensors (temperature, pH, dissolved oxygen, turbidity, conductivity)
- Controls water pumps for automated water changes (two bottles: in/out)
- Integrates cameras for visual monitoring
- Connects to a local **n8n** AI server for recommendations and alerts

---

## Architecture: Domain-Driven Design (DDD)

The project is split into layers, each with a clear responsibility:

```
smart-aquarium/
├── crates/
│   ├── domain/          ← 🧠 BUSINESS LOGIC (no hardware, no network)
│   │   └── src/
│   │       ├── sensor.rs    ← SensorReading, SensorKind, validation
│   │       └── water.rs     ← (coming soon) WaterQuality, thresholds
│   │
│   ├── application/     ← (coming soon) Use cases: "check water quality"
│   ├── infrastructure/  ← (coming soon) Real drivers: I2C, HTTP, GPIO
│   └── firmware/        ← (coming soon) ESP32 binary entry point
│
├── .coderabbit.yaml     ← AI code review config
├── Cargo.toml           ← Workspace root
└── README.md
```

### Why DDD?

| Layer | Rule | Benefit |
|-------|------|---------|
| `domain` | No hardware, no network | Testable on your laptop |
| `application` | Calls domain, no hardware | Testable without device |
| `infrastructure` | Implements hardware traits | Swappable (real vs mock) |
| `firmware` | Wires everything together | Thin, just composition |

---

## Development Progress

| Step | Status | Description |
|------|--------|-------------|
| Step 1 | ✅ Done | Project scaffold, DDD structure, `SensorReading` domain entity |
| Step 2 | ⏳ Next | `WaterQuality` value object and thresholds |
| Step 3 | 🔜 | `WaterChangeDecision` domain service |
| Step 4 | 🔜 | Application layer: `CheckWaterQualityUseCase` |
| Step 5 | 🔜 | Infrastructure: simulated sensor driver |
| Step 6 | 🔜 | ESP32 `no_std` firmware entry point |

---

## Running Tests

```bash
# Run all domain tests (no ESP32 needed!)
cargo test -p domain

# Run with output visible
cargo test -p domain -- --nocapture
```

---

## Hardware (planned)

- **MCU**: ESP32 (Espressif)
- **Temperature**: DS18B20 (1-Wire) or NTC thermistor
- **pH**: Analog pH probe + ADS1115 ADC
- **Dissolved Oxygen**: DO probe + ADS1115 ADC
- **Turbidity**: Optical turbidity sensor
- **Camera**: OV2640 (ESP32-CAM)
- **Pumps**: Peristaltic pumps via L298N motor driver
- **AI Server**: n8n (local, self-hosted)

---

## Prerequisites (for local development)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify
rustc --version   # should be 1.75+
cargo --version
```

ESP32 toolchain will be added in a later step.
