use std::{thread, time::Duration};
use esp_idf_hal::{gpio::*, prelude::*, i2c};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault, NvsPartitionId};
use esp_idf_svc::sntp::{EspSntp, SyncStatus, SntpConf, OperatingMode, SyncMode};
use esp_idf_hal::adc::{config::Config as AdcConfig, AdcChannelDriver, AdcDriver};
use esp_idf_svc::wifi::EspWifi;
use log::*;

use std::net::Ipv4Addr;
use std::time::{SystemTime, UNIX_EPOCH};

mod wifi;
mod displayctl;
mod keyevent;
mod wakepacket;
mod ui;
mod config;

use displayctl::{DisplayPanel, WiFiStatus, MessageTypes};
use keyevent::{KeySwitch, KeyEvent};
use config::ConfigData;

const SLEEP_MODE_LIGHT : &'static str = "light";
#[allow(dead_code)]
const SLEEP_MODE_DEEP : &'static str = "deep";

const GPIO_WAKEUP_INT_PIN_4 : i32 = 4;
const GPIO_WAKEUP_INT_PIN_5 : i32 = 5;
const GPIO_WAKEUP_INT_PIN_6 : i32 = 6;
const GPIO_WAKEUP_INT_PIN_9 : i32 = 9;
const GPIO_WAKEUP_INT_PIN_10 : i32 = 10;

const GPIO_WAKEUP_INT_PIN : u64 = 16 + 32;
const MAX_NVS_STR_SIZE : usize = 3072;

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Peripherals Initialize
    let peripherals = Peripherals::take().unwrap();
    // Gpio Initialize
    let up_key = peripherals.pins.gpio4;
    let down_key = peripherals.pins.gpio5;
    let left_key = peripherals.pins.gpio10;
    let right_key = peripherals.pins.gpio6;
    let enter_key = peripherals.pins.gpio9;

    let upkey_sig = Box::new(PinDriver::input(up_key)?);
    let downkey_sig = Box::new(PinDriver::input(down_key)?);
    let leftkey_sig = Box::new(PinDriver::input(left_key)?);
    let rightkey_sig = Box::new(PinDriver::input(right_key)?);
    let enterkey_sig = Box::new(PinDriver::input(enter_key)?);

    // Display Initialize
    let i2c = peripherals.i2c0;
    let scl = peripherals.pins.gpio7;
    let sda = peripherals.pins.gpio8;
    let config_i2c = i2c::I2cConfig::new().baudrate(1.MHz().into());
    let i2c = i2c::I2cDriver::new(i2c, sda, scl, &config_i2c)?;
    let mut dp = DisplayPanel::new();
    dp.start(i2c);
    dp.set_display_active(true);
    dp.set_initial_logo(true);

    // Initialize ADC
    let mut adc = AdcDriver::new(peripherals.adc1, &AdcConfig::new().calibration(true))?;
    let mut adc_pin : AdcChannelDriver<'_, {esp_idf_sys::adc_atten_t_ADC_ATTEN_DB_11}, Gpio3> = AdcChannelDriver::new(peripherals.pins.gpio3)?;
    
    // Initialize nvs
    let wakeup_reason : u32;
    unsafe {
        wakeup_reason = esp_idf_sys::esp_sleep_get_wakeup_cause();
        if wakeup_reason == esp_idf_sys::esp_sleep_source_t_ESP_SLEEP_WAKEUP_UNDEFINED {
            info!("First boot");
            if upkey_sig.is_low() {
                info!("Erase NVS flash...");
                dp.set_main_msg(&"Erase NVS flash...".to_string(), MessageTypes::Status);
                esp_idf_sys::nvs_flash_erase();
                thread::sleep(Duration::from_millis(1000));
            }    
            esp_idf_sys::nvs_flash_init();
        } else {
            info!("Wakeup reason: {:?}", wakeup_reason);
        }
    }


    let mut keysw = KeySwitch::new();
    keysw.start(upkey_sig, downkey_sig, rightkey_sig, enterkey_sig, leftkey_sig);

    // Initialize Configuration Data
    let mut config_data = ConfigData::new();

    // Initialize NVS
    let nvs_default_partition = EspNvsPartition::<NvsDefault>::take().unwrap();
    let mut nvs = match EspNvs::new(nvs_default_partition, "storage", true) {
        Ok(nvs) => { info!("NVS storage area initialized"); nvs },
        Err(ref e) => {
            dp.set_main_msg(&"NVS initialization failed.".to_string(), MessageTypes::Error); 
            panic!("NVS initialization failed {:?}", e); }
    };

    // Load config
    let mut nvs_buf : [u8 ; MAX_NVS_STR_SIZE] = [0; MAX_NVS_STR_SIZE];
    let nvs_value = match nvs.get_str("config", &mut nvs_buf){
        Ok(value) => { info!("Try to read NVS config"); value },
        Err(ref e) => { info!("NVS config not found {:?}", e); None }
    };
    if nvs_value == None {
        info!("NVS config not found. Set default config");
        set_default_config(&mut config_data, &mut nvs);
        dp.set_main_msg(&"Using default settings.".to_string(), MessageTypes::Status);
        thread::sleep(Duration::from_millis(1000));
    }
    else {
        info!("NVS config found {:?}", nvs_value);
        match config_data.load_config(nvs_value) {
            Ok(_) => { info!("Config load success"); },
            Err(ref e) => { 
                info!("Config load failed {:?}", e);
                dp.set_main_msg(&"Using default settings.".to_string(), MessageTypes::Status);
                set_default_config(&mut config_data, &mut nvs);
                thread::sleep(Duration::from_millis(1000));
            },
        }    
    }

    dp.initialize_menu(&config_data);
    info!("SSID: {}", config_data.wifi_ssid);

    // Initialize Wakepacket
    let mut wp = wakepacket::WakePacket::new();
    wp.set_target_mac(wakepacket::TargetDevice::Device1, &config_data.target_mac_address1);
    wp.set_target_mac(wakepacket::TargetDevice::Device2, &config_data.target_mac_address2);
    wp.set_target_mac(wakepacket::TargetDevice::Device3, &config_data.target_mac_address3);
    wp.set_target_mac(wakepacket::TargetDevice::Device4, &config_data.target_mac_address4);
    // Set Timezone
    dp.set_timezone_offset(config_data.timezone_offset);
    dp.set_initial_logo(false);

    // Initialize WiFi
    // WiFi
    dp.set_main_msg(&"Connecting WiFi..".to_string(), MessageTypes::Status);
    dp.set_second_msg(&format!("AP:{}", config_data.wifi_ssid));
    dp.set_wifi_status(WiFiStatus::Connecting);
    let mut wifi_dev = wifi::wifi_connect(peripherals.modem, &config_data.wifi_ssid, &config_data.wifi_psk);
    match &wifi_dev {
        Ok(_) => { dp.set_wifi_status(WiFiStatus::Connected);},
        Err(ref e) => { info!("{:?}", e); }
    }
      
    // Get my IP address
    let mut ip_addr : Ipv4Addr; 
    loop {
        ip_addr = wifi_dev.as_ref().unwrap().sta_netif().get_ip_info().unwrap().ip;
        if ip_addr != Ipv4Addr::new(0, 0, 0, 0) {
            break;
        }
        info!("Waiting for WiFi connection...");
        thread::sleep(Duration::from_secs(1));
    }

    // NTP Server
    let sntp_conf = SntpConf {
        servers: ["time.aws.com",
                  "time.google.com",
                  "time.cloudflare.com",
                  "ntp.nict.jp"],
        operating_mode: OperatingMode::Poll,
        sync_mode: SyncMode::Immediate,
    };
    let ntp = EspSntp::new(&sntp_conf).unwrap();

    info!("IP address: {}", ip_addr);
    dp.set_second_msg(&format!("IP: {}", ip_addr));

    // NTP Sync
    let now = SystemTime::now();
    if now.duration_since(UNIX_EPOCH).unwrap().as_millis() < 1700000000 {
        info!("NTP Sync Start..");
        dp.set_main_msg(&"NTP Syncing..".to_string(), MessageTypes::Status);
        // No wait for sync
        // while ntp.get_sync_status() != SyncStatus::Completed {
        //     thread::sleep(Duration::from_millis(10));
        // }
        // let now = SystemTime::now();
        // let dt_now : DateTime<Utc> = now.into();
        // let formatted = format!("{}", dt_now.format("%Y-%m-%d %H:%M:%S"));
        // info!("NTP Sync Completed: {}", formatted);
    } 

    // Main Loop
    let mut send_count : u32 = 0;
    let mut loop_count : u32 = 0;
    let mut start_time = SystemTime::now();
    let mut rssi : i32;
    loop {
        // Get Battery Voltage
        let battery_voltage : f32 =  adc.read(&mut adc_pin).unwrap() as f32 * 2.0 / 1000.0;
        dp.set_battery_voltage(battery_voltage);
        // Get RSSI
        rssi = wifi::get_rssi();
        dp.set_wifi_rssi(rssi);
        if rssi == 0 {
            wifi_reconnect(&mut wifi_dev.as_mut().unwrap(), &mut dp);
        }
        else {
            dp.set_wifi_status(WiFiStatus::Connected);
        }

        // If need to sync time
        match start_time.elapsed() {
            Ok(elapsed) => {
                if elapsed.as_secs() > 3600 {
                    info!("NTP Sync Start..");
                    dp.set_main_msg(&"NTP Syncing..".to_string(), MessageTypes::Status);
                    let sync_status = ntp.get_sync_status();
                    if sync_status != SyncStatus::Completed {
                        start_time = SystemTime::now();
                    } 
                }
            },
            Err(e) => {
                info!("Error: {:?}", e);
            }
        }
        // Get Key Event
        let mut target_device : wakepacket::TargetDevice = wakepacket::TargetDevice::None;
        dp.set_main_msg(&"Push Button..".to_string(), MessageTypes::Ready);
        let key_envet = keysw.get_key_event_and_clear();
        for it in key_envet {
            info!("Key Event: {:?}", it);
            match it {
                KeyEvent::UpKeyUp | KeyEvent::UpKeyDown => {
                    target_device = wakepacket::TargetDevice::Device1;
                    info!("Up key pressed. Target device1");
                    dp.set_main_msg(&"PC1".to_string(), MessageTypes::Progress);
                    break;
                },
                KeyEvent::DownKeyUp | KeyEvent::DownKeyDown => {
                    target_device = wakepacket::TargetDevice::Device2;
                    info!("Down key pressed. Target device2");
                    dp.set_main_msg(&"PC2".to_string(), MessageTypes::Progress);
                    break;
                },
                KeyEvent::LeftKeyUp | KeyEvent::LeftKeyDown => {
                    target_device = wakepacket::TargetDevice::Device3;
                    info!("Left key pressed. Target device3");
                    dp.set_main_msg(&"PC3".to_string(), MessageTypes::Progress);
                    break;
                },
                KeyEvent::RightKeyUp | KeyEvent::RightKeyDown => {
                    target_device = wakepacket::TargetDevice::Device4;
                    info!("Right key pressed. Target device4");
                    dp.set_main_msg(&"PC4".to_string(), MessageTypes::Progress);
                    break;
                },
                KeyEvent::CenterKeyUp | KeyEvent::CenterKeyDown => {
                    info!("Enter key pressed. Show menu");
                    dp.set_display_active(true);
                    let config = select_menu(&mut dp, &mut keysw);
                    if config != None {
                        if is_going_to_reset(config.as_ref().unwrap()) {
                            info!("Reset config");
                            unsafe {
                                esp_idf_sys::nvs_flash_erase();
                            }
                            dp.set_main_msg(&"Reset Config...".to_string(), MessageTypes::Status);
                            thread::sleep(Duration::from_secs(3));
                        }
                        else {
                            let toml_string = convert_config_to_toml_string(&config.unwrap());
                            info!("New config: {}", toml_string);
                            let _ = nvs.set_str("config", &toml_string);
                        }
                        dp.set_main_msg(&"Restarting...".to_string(), MessageTypes::Status);
                        thread::sleep(Duration::from_secs(1));
                        unsafe {
                            esp_idf_sys::esp_restart();
                        }
                    }
                    else {
                        dp.set_main_msg(&"No Save...".to_string(), MessageTypes::Status);
                    }
                    thread::sleep(Duration::from_secs(1));
                    keysw.clear_all_button_event();
                    loop_count = 0;
                    break;
                }
            }
        }
        if target_device != wakepacket::TargetDevice::None {
            dp.set_display_active(true);
            let mut send_retry_count : u32 = 0;
            loop {
                thread::sleep(Duration::from_secs(1));
                let status = wp.send_pkt(target_device);
                if !status.is_err() {
                    send_count += 1;
                    dp.set_send_pkt(send_count);
                    dp.set_main_msg(&"Completed.".to_string(), MessageTypes::WakeUp);
                    thread::sleep(Duration::from_secs(2));
                    keysw.clear_all_button_event();
                    break;
                }
                else {
                    send_retry_count += 1;
                    info!("Send magic packet failed");
                    if send_retry_count >= 5 {
                        dp.set_main_msg(&"Send Failed.".to_string(), MessageTypes::Error);
                        break;
                    }
                }
            }
            loop_count = 0;
        }
        else {
            loop_count += 1;
        }
        if config_data.idle_in_sleep_time == 0 {
            if config_data.display_off_time > 0 && loop_count >= config_data.display_off_time {
                loop_count = 0;
                // display off
                dp.set_display_active(false);
            }
        }
        else {
            // Sleep
            if loop_count >= config_data.idle_in_sleep_time {
                info!("Sleep Now...");
                dp.set_main_msg(&"Sleeping..".to_string(), MessageTypes::Status);
                dp.set_wifi_status(WiFiStatus::Disconnected);
                thread::sleep(Duration::from_millis(1000));
                dp.set_display_active(false);
                unsafe {
                    esp_idf_sys::esp_wifi_stop();
                }
                thread::sleep(Duration::from_millis(1000));
                loop_count = 0;
                unsafe {
                    // light sleep mode
                    if config_data.sleep_mode == SLEEP_MODE_LIGHT {
                        // gpio wakeup enable
                        esp_idf_sys::gpio_wakeup_enable(GPIO_WAKEUP_INT_PIN_4, esp_idf_sys::gpio_int_type_t_GPIO_INTR_LOW_LEVEL);
                        esp_idf_sys::gpio_wakeup_enable(GPIO_WAKEUP_INT_PIN_5, esp_idf_sys::gpio_int_type_t_GPIO_INTR_LOW_LEVEL);
                        esp_idf_sys::gpio_wakeup_enable(GPIO_WAKEUP_INT_PIN_6, esp_idf_sys::gpio_int_type_t_GPIO_INTR_LOW_LEVEL);
                        esp_idf_sys::gpio_wakeup_enable(GPIO_WAKEUP_INT_PIN_9, esp_idf_sys::gpio_int_type_t_GPIO_INTR_LOW_LEVEL);
                        esp_idf_sys::gpio_wakeup_enable(GPIO_WAKEUP_INT_PIN_10, esp_idf_sys::gpio_int_type_t_GPIO_INTR_LOW_LEVEL);
                        esp_idf_sys::esp_sleep_enable_gpio_wakeup();
                        // wakeup from rtc timer
                        // esp_idf_sys::esp_sleep_enable_timer_wakeup(config_data.wakeup_interval as u64 * 1000);
                    }
                    else {
                        info!("Deep Sleep Start");
                        esp_idf_sys::esp_deep_sleep_enable_gpio_wakeup(GPIO_WAKEUP_INT_PIN, esp_idf_sys::esp_deepsleep_gpio_wake_up_mode_t_ESP_GPIO_WAKEUP_GPIO_LOW);
                        esp_idf_sys::esp_deep_sleep_start();
                    }

                    // deep sleep mode (not here)
                    if config_data.sleep_mode == SLEEP_MODE_LIGHT {
                        let _result = esp_idf_sys::esp_light_sleep_start();
                        // gpio interrupt enable
                        esp_idf_sys::gpio_set_intr_type(GPIO_WAKEUP_INT_PIN_4, esp_idf_sys::gpio_int_type_t_GPIO_INTR_ANYEDGE);
                        esp_idf_sys::gpio_set_intr_type(GPIO_WAKEUP_INT_PIN_5, esp_idf_sys::gpio_int_type_t_GPIO_INTR_ANYEDGE);
                        esp_idf_sys::gpio_set_intr_type(GPIO_WAKEUP_INT_PIN_6, esp_idf_sys::gpio_int_type_t_GPIO_INTR_ANYEDGE);
                        esp_idf_sys::gpio_set_intr_type(GPIO_WAKEUP_INT_PIN_9, esp_idf_sys::gpio_int_type_t_GPIO_INTR_ANYEDGE);
                        esp_idf_sys::gpio_set_intr_type(GPIO_WAKEUP_INT_PIN_10, esp_idf_sys::gpio_int_type_t_GPIO_INTR_ANYEDGE);
                        continue;
                    }
                }
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn wifi_reconnect(wifi_dev: &mut EspWifi, dp: &mut DisplayPanel) -> bool{
    // display on
    dp.set_wifi_status(WiFiStatus::Connecting);
    unsafe {
        esp_idf_sys::esp_wifi_start();
    }
    match wifi_dev.connect() {
        Ok(_) => { info!("Wifi connected"); true},
        Err(ref e) => { info!("{:?}", e); false }
    }
}



fn select_menu(dp: &mut DisplayPanel, keysw: &mut KeySwitch) -> Option<Vec<(String, String)>> {
    dp.reset_menu();
    dp.set_main_msg(&"Menu".to_string(), MessageTypes::Menu);
    thread::sleep(Duration::from_millis(300));
    keysw.clear_all_button_event();
    loop {
        let key_envet = keysw.get_key_event_and_clear();
        for it in key_envet {
            info!("Key Event: {:?}", it);
            let (exit, data) = dp.key_event_input(it);
            if exit {
                return data;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn is_going_to_reset(keyval: &Vec<(String, String)>) -> bool {
    for it in keyval {
        if it.0 == "RESETCONFIG" {
            return true;
        }
    }
    false
}

fn set_default_config<T : NvsPartitionId>(config: &mut ConfigData, nvs: &mut EspNvs<T>){
    let default_config = config.set_default_config();
    let toml_cfg = convert_config_to_toml_string(&default_config);
    match nvs.set_str("config", toml_cfg.as_str()) {
        Ok(_) => { info!("Set default config"); },
        Err(ref e) => { info!("Set default config failed {:?}", e); }
    }
    config.load_config(Some(toml_cfg.as_str())).unwrap();
}

fn convert_config_to_toml_string(keyval: &Vec<(String, String)>) -> String {
    let mut toml_string = String::new();
    for it in keyval {
        toml_string.push_str(&format!("{} = \"{}\"\n", it.0, it.1));
    }
    toml_string
}
