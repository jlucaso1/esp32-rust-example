fn main() {
    linker_be_nice();
    configure_wifi_env();
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    // make sure linkall.x is the last linker script (otherwise might cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}

fn configure_wifi_env() {
    use std::path::Path;

    const SSID_KEY: &str = "ESP_WIFI_SSID";
    const PASSWORD_KEY: &str = "ESP_WIFI_PASSWORD";

    // Re-run the build script when credentials change.
    println!("cargo:rerun-if-env-changed={SSID_KEY}");
    println!("cargo:rerun-if-env-changed={PASSWORD_KEY}");
    println!("cargo:rerun-if-changed=.env");

    let mut ssid = std::env::var(SSID_KEY).ok();
    let mut password = std::env::var(PASSWORD_KEY).ok();

    if (ssid.is_none() || password.is_none()) && Path::new(".env").exists() {
        match dotenvy::from_path_iter(".env") {
            Ok(iter) => {
                for entry in iter {
                    let (key, value) = entry.expect("failed to parse entry in .env file");

                    if ssid.is_none() && key == SSID_KEY {
                        ssid = Some(value);
                    } else if password.is_none() && key == PASSWORD_KEY {
                        password = Some(value);
                    }
                }
            }
            Err(err) => {
                panic!("failed to load Wi-Fi credentials from .env: {err}");
            }
        }
    }

    let (ssid, password) = match (ssid, password) {
        (Some(ssid), Some(password)) => (ssid, password),
        _ => {
            panic!(
                "Wi-Fi credentials missing. Set {SSID_KEY} and {PASSWORD_KEY} as environment variables or provide a .env file."
            );
        }
    };

    println!("cargo:rustc-env={SSID_KEY}={ssid}");
    println!("cargo:rustc-env={PASSWORD_KEY}={password}");
}

fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                "_defmt_timestamp" => {
                    eprintln!();
                    eprintln!("💡 `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`");
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!("💡 Is the linker script `linkall.x` missing?");
                    eprintln!();
                }
                "esp_wifi_preempt_enable"
                | "esp_wifi_preempt_yield_task"
                | "esp_wifi_preempt_task_create" => {
                    eprintln!();
                    eprintln!("💡 `esp-wifi` has no scheduler enabled. Make sure you have the `builtin-scheduler` feature enabled, or that you provide an external scheduler.");
                    eprintln!();
                }
                "embedded_test_linker_file_not_added_to_rustflags" => {
                    eprintln!();
                    eprintln!("💡 `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests");
                    eprintln!();
                }
                _ => (),
            },
            // we don't have anything helpful for "missing-lib" yet
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=-Wl,--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}
