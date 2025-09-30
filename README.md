# ESP32 Rust Firmware

This repository contains an example firmware project targeting the ESP32 using the [`esp-hal`](https://github.com/esp-rs/esp-hal) ecosystem. The default build target is `xtensa-esp32-none-elf` and requires the Espressif Rust toolchain.

## Prerequisites

1. Install [`espup`](https://github.com/esp-rs/espup):
   ```sh
   cargo install espup
   ```
2. Provision the Espressif toolchain and Xtensa Rust components:
   ```sh
   espup install
   ```
3. Configure the environment in each new shell session:
   ```sh
   . "$HOME/export-esp.sh"
   ```

## Configure Wi-Fi credentials

Wi-Fi credentials are embedded into the binary at compile time. To keep them out of version control:

1. Copy the provided template and populate it with your network details:
   ```sh
   cp .env.example .env
   ```
2. Edit `.env` and set `ESP_WIFI_SSID` and `ESP_WIFI_PASSWORD`.

During builds the script reads `.env` (or falls back to host environment variables) and exports the credentials using `cargo:rustc-env`. The build will fail if either value is missing or empty, ensuring secrets are configured before flashing. The `.env` file is ignored by Git by default.

## Build

With the environment configured, build the firmware in release mode:

```sh
cargo build -r
```

The build artifacts will be placed under `target/xtensa-esp32-none-elf/release/`.

## Flash & Monitor

The cargo runner is configured to use [`espflash`](https://github.com/esp-rs/espflash`). After connecting the board, run:

```sh
cargo run
```

This will flash the firmware and open a serial monitor using defmt log formatting.

## Troubleshooting

- If the linker (`xtensa-esp32-elf-gcc`) cannot be found, ensure you've sourced `export-esp.sh` in the current shell.
- The global allocator is initialized from a static heap; the compiler emits a warning about mutable statics, which is expected for this pattern in `no_std` environments.
