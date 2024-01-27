use config::{File, FileFormat, Config as NvsConfig};
use std::collections::HashMap;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
    #[default("")]
    target_mac_address1: &'static str,
    #[default("")]
    target_mac_address2: &'static str,
    #[default("")]
    target_mac_address3: &'static str,
    #[default("")]
    target_mac_address4: &'static str,
    #[default("0")]
    timezone_offset: &'static str,  // Timezone offset from UTC -12 to +14
    #[default("30")]
    idle_in_sleep_time: &'static str,   // 0: disable sleep, 1-: sleep time in seconds when no key input
    #[default("light")]
    sleep_mode: &'static str,   // light or deep
    #[default("30")]
    display_off_time: &'static str, // 0: always on, 1-: display off time in seconds
}

const MENU_SSID: &str = "SSID";
const MENU_PSK: &str = "PSK";
const MENU_PC1: &str = "PC1";
const MENU_PC2: &str = "PC2";
const MENU_PC3: &str = "PC3";
const MENU_PC4: &str = "PC4";
const MENU_TIMEZONE: &str = "TIMEZONE";
const MENU_IDLESLEEP: &str = "IDLESLEEP";
const MENU_SLEEPMODE: &str = "SLEEPMODE";
const MENU_DISPLAYOFFTIME: &str = "DISPLAYOFFTIME";

#[derive(Debug)]
pub struct ConfigData {
    pub wifi_ssid: String,
    pub wifi_psk: String,
    pub target_mac_address1: String,
    pub target_mac_address2: String,
    pub target_mac_address3: String,
    pub target_mac_address4: String,
    pub timezone_offset: i32,
    pub idle_in_sleep_time: u32,
    pub sleep_mode: String,
    pub display_off_time: u32,
}

impl ConfigData {
    pub fn new() -> ConfigData {
        ConfigData {
            wifi_ssid: String::new(),
            wifi_psk: String::new(),
            target_mac_address1: String::new(),
            target_mac_address2: String::new(),
            target_mac_address3: String::new(),
            target_mac_address4: String::new(),
            timezone_offset: 0,
            idle_in_sleep_time: 30,
            sleep_mode: String::from("light"),
            display_off_time: 30,
        }
    }
    pub fn load_config(&mut self, nvs_value: Option<&str>) -> anyhow::Result<()> {
        if nvs_value == None {
            return Err(anyhow::Error::msg("nvs_value is None"));
        }
        let settings = NvsConfig::builder()
        .add_source(File::from_str(&nvs_value.unwrap(), FileFormat::Toml))
        .build()?;
        let settings_map = settings.try_deserialize::<HashMap<String, String>>()?;
        self.wifi_ssid = settings_map.get(MENU_SSID).ok_or(anyhow::Error::msg("wifi_ssid not found"))?.to_string();
        self.wifi_psk = settings_map.get(MENU_PSK).ok_or(anyhow::Error::msg("wifi_psk not found"))?.to_string();
        self.target_mac_address1 = settings_map.get(MENU_PC1).ok_or(anyhow::Error::msg("target_mac_address1 not found"))?.to_string();
        self.target_mac_address2 = settings_map.get(MENU_PC2).ok_or(anyhow::Error::msg("target_mac_address2 not found"))?.to_string();
        self.target_mac_address3 = settings_map.get(MENU_PC3).ok_or(anyhow::Error::msg("target_mac_address3 not found"))?.to_string();
        self.target_mac_address4 = settings_map.get(MENU_PC4).ok_or(anyhow::Error::msg("target_mac_address4 not found"))?.to_string();
        self.timezone_offset = settings_map.get(MENU_TIMEZONE).ok_or(anyhow::Error::msg("timezone_offset not found"))?.parse::<i32>()?;
        self.idle_in_sleep_time = settings_map.get(MENU_IDLESLEEP).ok_or(anyhow::Error::msg("idle_in_sleep_time not found"))?.parse::<u32>()?;
        self.sleep_mode = settings_map.get(MENU_SLEEPMODE).ok_or(anyhow::Error::msg("sleep_mode not found"))?.to_string();
        self.display_off_time = settings_map.get(MENU_DISPLAYOFFTIME).ok_or(anyhow::Error::msg("display_off_time not found"))?.parse::<u32>()?;
        Ok(())
    }
    
    pub fn set_default_config(&self) -> Vec::<(String, String)> {
        let mut default_config = Vec::<(String, String)>::new();
        default_config.push((MENU_SSID.to_string(), CONFIG.wifi_ssid.to_string()));
        default_config.push((MENU_PSK.to_string(),  CONFIG.wifi_psk.to_string()));
        default_config.push((MENU_PC1.to_string(), CONFIG.target_mac_address1.to_string()));
        default_config.push((MENU_PC2.to_string(), CONFIG.target_mac_address2.to_string()));
        default_config.push((MENU_PC3.to_string(), CONFIG.target_mac_address3.to_string()));
        default_config.push((MENU_PC4.to_string(), CONFIG.target_mac_address4.to_string()));
        default_config.push((MENU_TIMEZONE.to_string(), CONFIG.timezone_offset.to_string()));
        default_config.push((MENU_IDLESLEEP.to_string(), CONFIG.idle_in_sleep_time.to_string()));
        default_config.push((MENU_SLEEPMODE.to_string(), CONFIG.sleep_mode.to_string()));
        default_config.push((MENU_DISPLAYOFFTIME.to_string(), CONFIG.display_off_time.to_string()));
        default_config
    }    
}

