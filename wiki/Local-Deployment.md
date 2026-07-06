# Chronos — Local Deployment

This guide explains how to set up your environment, build, verify, and run **Chronos** locally.

---

## 📋 Prerequisites

To compile the application, ensure you have the Rust compiler (v1.92+) installed.

### 🐧 Linux Dependencies
On Linux systems, install the following packages for GUI rendering (`egui`/`eframe`) and system tray support:

```bash
sudo apt-get update && sudo apt-get install -y \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxcb1-dev libx11-dev libxi-dev libxtst-dev libxkbcommon-dev \
  libgtk-3-dev libatk1.0-dev libcairo2-dev libglib2.0-dev libpango1.0-dev \
  libssl-dev pkg-config
```

---

## 🛠️ Build and Verify

Chronos matches the CI workflows used by Thoth. To verify code formatting, clippy warnings, tests, and compilation:

1. **Run Local CI Checks:**
   ```bash
   ./local-ci.sh
   ```
2. **Run Tests Individually:**
   ```bash
   cargo test
   ```
3. **Build Release Binary:**
   ```bash
   cargo build --release
   ```

---

## 🚀 Execution

To start the built application:
```bash
cargo run --release
```
The application will launch and minimize to the system tray, waiting for your click interactions.
