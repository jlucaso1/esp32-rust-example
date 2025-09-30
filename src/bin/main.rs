#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

extern crate alloc;

use alloc::string::String;

use blocking_network_stack::{ipv4, Stack};
use defmt::info;
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    rng::Rng,
    time::{Duration, Instant},
    timer::timg::TimerGroup,
};
use esp_wifi::{
    config::PowerSaveMode,
    wifi::{AuthMethod, ClientConfiguration, Configuration, WifiController, WifiDevice},
};
use smoltcp::{
    iface::{Interface, SocketSet, SocketStorage},
    wire::{EthernetAddress, HardwareAddress},
};
use {esp_backtrace as _, esp_println as _};

esp_bootloader_esp_idf::esp_app_desc!();

const WIFI_SSID: &str = env!("ESP_WIFI_SSID");
const WIFI_PASSWORD: &str = env!("ESP_WIFI_PASSWORD");

const _: () = {
    assert!(!WIFI_SSID.is_empty(), "ESP_WIFI_SSID must not be empty");
    assert!(
        !WIFI_PASSWORD.is_empty(),
        "ESP_WIFI_PASSWORD must not be empty"
    );
};

fn current_millis() -> u64 {
    esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_millis()
}

fn timestamp() -> smoltcp::time::Instant {
    smoltcp::time::Instant::from_micros(
        esp_hal::time::Instant::now()
            .duration_since_epoch()
            .as_micros() as i64,
    )
}

fn create_interface(device: &mut WifiDevice<'_>) -> Interface {
    let mac = EthernetAddress::from_bytes(&device.mac_address());
    let mut config = smoltcp::iface::Config::new(HardwareAddress::Ethernet(mac));
    config.random_seed = esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_secs();

    Interface::new(config, device, timestamp())
}

fn configure_client(controller: &mut WifiController<'_>) {
    let client = ClientConfiguration {
        ssid: String::from(WIFI_SSID),
        password: String::from(WIFI_PASSWORD),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    };

    controller
        .set_configuration(&Configuration::Client(client))
        .expect("failed to apply Wi-Fi client configuration");

    controller
        .set_power_saving(PowerSaveMode::None)
        .expect("failed to update Wi-Fi power save mode");
}

#[main]
fn main() -> ! {
    esp_alloc::heap_allocator!(size: 96 * 1024);

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let mut led = Output::new(peripherals.GPIO2, Level::High, OutputConfig::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut wdt0 = timg0.wdt;
    wdt0.disable();

    let mut rng = Rng::new(peripherals.RNG);
    let random_seed = rng.random();

    let controller = esp_wifi::init(timg0.timer0, rng).expect("failed to initialize Wi-Fi");

    let (mut wifi_controller, interfaces) =
        esp_wifi::wifi::new(&controller, peripherals.WIFI).expect("failed to create Wi-Fi");

    configure_client(&mut wifi_controller);

    let mut wifi_device = interfaces.sta;
    let iface = create_interface(&mut wifi_device);

    let mut socket_storage: [SocketStorage; 6] = Default::default();
    let socket_set = SocketSet::new(&mut socket_storage[..]);
    let mut stack = Stack::new(iface, wifi_device, socket_set, current_millis, random_seed);

    stack
        .set_iface_configuration(&ipv4::Configuration::Client(
            ipv4::ClientConfiguration::DHCP(Default::default()),
        ))
        .expect("failed to configure DHCP");
    stack.reset_dhcp();

    info!("Starting Wi-Fi connection");
    esp_println::println!("Connecting to SSID \"{}\"", WIFI_SSID);

    wifi_controller
        .start()
        .expect("failed to start Wi-Fi controller");
    wifi_controller
        .connect()
        .expect("failed to start Wi-Fi connection");

    let mut led_timer = Instant::now();
    let mut ip_reported = false;

    loop {
        stack.work();

        match wifi_controller.is_connected() {
            Ok(true) => {
                if !ip_reported && stack.is_iface_up() {
                    if let Ok(ip_info) = stack.get_ip_info() {
                        esp_println::println!(
                            "Network ready: ip={} gateway={} netmask={}",
                            ip_info.ip,
                            ip_info.subnet.gateway,
                            ip_info.subnet.mask
                        );
                        ip_reported = true;
                    }
                }

                if led_timer.elapsed() >= Duration::from_millis(750) {
                    led.toggle();
                    led_timer = Instant::now();
                }
            }
            Ok(false) => {
                ip_reported = false;

                if led_timer.elapsed() >= Duration::from_millis(200) {
                    led.toggle();
                    led_timer = Instant::now();
                }
            }
            Err(err) => {
                esp_println::println!("Wi-Fi status error: {:?}", err);
            }
        }
    }
}
