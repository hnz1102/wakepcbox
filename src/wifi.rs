// Wi-Fi connection and RSSI measurement
// SPDX-License-Identifier: MIT
// Copyright (c) 2024 Hiroshi Nakajima

#![allow(dead_code)]

use std::time::Duration;
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};

use esp_idf_hal::peripheral;
use esp_idf_svc::{eventloop::EspSystemEventLoop, handle::RawHandle, wifi::EspWifi};
use esp_idf_sys;

use embedded_svc::wifi::{ClientConfiguration, Configuration};
use anyhow::bail;
use anyhow::Result;
use std::str::FromStr;
use log::info;

fn wait_for_dhcp_ip(
    wifi: &EspWifi<'_>,
    timeout_secs: u64,
    tag: &str,
) -> Option<std::net::Ipv4Addr> {
    for _ in 0..timeout_secs {
        let ip = wifi.sta_netif()
            .get_ip_info()
            .map(|i| i.ip)
            .unwrap_or(std::net::Ipv4Addr::UNSPECIFIED);
        if ip != std::net::Ipv4Addr::UNSPECIFIED {
            info!("[{}] DHCP IP: {}", tag, ip);
            return Some(ip);
        }
        thread::sleep(Duration::from_secs(1));
    }
    None
}

// ─── WPS state (shared between C event handler and Rust polling loop) ─────────

static WPS_SUCCESS: AtomicBool = AtomicBool::new(false);
static WPS_FAILED:  AtomicBool = AtomicBool::new(false);
static WPS_GOT_CREDS: AtomicBool = AtomicBool::new(false);

static mut WPS_SSID_BUF: [u8; 33] = [0u8; 33];
static mut WPS_PASS_BUF: [u8; 65] = [0u8; 65];

unsafe extern "C" fn wps_event_handler(
    _arg: *mut core::ffi::c_void,
    _base: esp_idf_sys::esp_event_base_t,
    event_id: i32,
    event_data: *mut core::ffi::c_void,
) {
    use esp_idf_sys::*;
    let id = event_id as u32;
    if id == wifi_event_t_WIFI_EVENT_STA_WPS_ER_SUCCESS {
        if !event_data.is_null() {
            let evt = &*(event_data as *const wifi_event_sta_wps_er_success_t);
            if evt.ap_cred_cnt > 0 {
                let ssid = &evt.ap_cred[0].ssid;
                let pass = &evt.ap_cred[0].passphrase;
                let slen = ssid.iter().position(|&b| b == 0).unwrap_or(ssid.len());
                let plen = pass.iter().position(|&b| b == 0).unwrap_or(pass.len());
                let dst_s = core::ptr::addr_of_mut!(WPS_SSID_BUF) as *mut u8;
                core::ptr::copy_nonoverlapping(ssid.as_ptr(), dst_s, slen);
                dst_s.add(slen).write(0u8);
                let dst_p = core::ptr::addr_of_mut!(WPS_PASS_BUF) as *mut u8;
                core::ptr::copy_nonoverlapping(pass.as_ptr(), dst_p, plen);
                dst_p.add(plen).write(0u8);
                WPS_GOT_CREDS.store(true, Ordering::Release);
            }
        }
        WPS_SUCCESS.store(true, Ordering::Release);
    } else if id == wifi_event_t_WIFI_EVENT_STA_WPS_ER_FAILED
           || id == wifi_event_t_WIFI_EVENT_STA_WPS_ER_TIMEOUT {
        WPS_FAILED.store(true, Ordering::Release);
    }
}

pub fn wifi_connect(
    modem: impl peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    ssid: &str,
    pass: &str,
) -> Result<Box<EspWifi<'static>>> {

    if ssid.is_empty() || pass.is_empty() {
        bail!("SSID or password is empty");
    }
    let sys_event_loop = EspSystemEventLoop::take().unwrap();
    let mut wifi = Box::new(EspWifi::new(modem, sys_event_loop.clone(), None).unwrap());

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: heapless::String::<32>::from_str(ssid).unwrap(),
        password: heapless::String::<64>::from_str(pass).unwrap(),
        ..Default::default()
    })).unwrap();

    wifi.start().unwrap();
    wifi.connect()?;
    let mut timeout = 0;
    loop {
        if wifi.is_connected().unwrap() {
            break;
        }
        thread::sleep(Duration::from_secs(1));
        timeout += 1;
        if timeout > 30 {
            break;
        }
    }

    let _ = wait_for_dhcp_ip(&wifi, 20, "WiFi");
    Ok(wifi)
}

/// Connect using WPS PBC (Push Button Configuration).
///
/// Returns `(EspWifi, ssid, passphrase)` so the caller can persist credentials.
/// `set_status` is a closure called with a display message during the WPS wait.
pub fn wifi_connect_wps(
    modem: impl peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    mut set_status: impl FnMut(&str, &str),
) -> Result<(Box<EspWifi<'static>>, String, String)> {
    info!("[WPS] Starting WPS PBC — press the WPS button on your router within 120 s");
    set_status("WPS MODE", "Push WPS button");

    let sys_event_loop = EspSystemEventLoop::take().unwrap();
    let mut wifi = Box::new(EspWifi::new(modem, sys_event_loop, None).unwrap());

    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default())).unwrap();
    wifi.start().unwrap();

    WPS_SUCCESS.store(false, Ordering::Release);
    WPS_FAILED.store(false, Ordering::Release);
    WPS_GOT_CREDS.store(false, Ordering::Release);

    unsafe {
        esp_idf_sys::esp_event_handler_register(
            esp_idf_sys::WIFI_EVENT,
            esp_idf_sys::ESP_EVENT_ANY_ID,
            Some(wps_event_handler),
            core::ptr::null_mut(),
        );
        let cfg = esp_idf_sys::esp_wps_config_t {
            wps_type: esp_idf_sys::wps_type_WPS_TYPE_PBC,
            ..Default::default()
        };
        let ret = esp_idf_sys::esp_wifi_wps_enable(&cfg);
        if ret != esp_idf_sys::ESP_OK as i32 {
            esp_idf_sys::esp_event_handler_unregister(
                esp_idf_sys::WIFI_EVENT,
                esp_idf_sys::ESP_EVENT_ANY_ID,
                Some(wps_event_handler),
            );
            bail!("[WPS] esp_wifi_wps_enable failed: 0x{:x}", ret);
        }
        let ret = esp_idf_sys::esp_wifi_wps_start(0);
        if ret != esp_idf_sys::ESP_OK as i32 {
            esp_idf_sys::esp_wifi_wps_disable();
            esp_idf_sys::esp_event_handler_unregister(
                esp_idf_sys::WIFI_EVENT,
                esp_idf_sys::ESP_EVENT_ANY_ID,
                Some(wps_event_handler),
            );
            bail!("[WPS] esp_wifi_wps_start failed: 0x{:x}", ret);
        }
    }

    // Poll up to 120 s
    for elapsed in 0..120u32 {
        thread::sleep(Duration::from_secs(1));

        if elapsed % 10 == 0 && elapsed > 0 {
            let remaining = 120 - elapsed;
            info!("[WPS] Waiting… {}s elapsed, {}s remaining", elapsed, remaining);
            set_status("WPS MODE", &format!("{}s remaining", remaining));
        }

        if WPS_FAILED.load(Ordering::Acquire) {
            unsafe {
                esp_idf_sys::esp_wifi_wps_disable();
                esp_idf_sys::esp_event_handler_unregister(
                    esp_idf_sys::WIFI_EVENT,
                    esp_idf_sys::ESP_EVENT_ANY_ID,
                    Some(wps_event_handler),
                );
            }
            bail!("[WPS] WPS failed or timed out by AP at {}s", elapsed);
        }

        if WPS_SUCCESS.load(Ordering::Acquire) {
            let (ssid, pass) = if WPS_GOT_CREDS.load(Ordering::Acquire) {
                unsafe {
                    let mut ssid_local = [0u8; 33];
                    let mut pass_local = [0u8; 65];
                    core::ptr::copy_nonoverlapping(
                        core::ptr::addr_of!(WPS_SSID_BUF) as *const u8,
                        ssid_local.as_mut_ptr(), 33,
                    );
                    core::ptr::copy_nonoverlapping(
                        core::ptr::addr_of!(WPS_PASS_BUF) as *const u8,
                        pass_local.as_mut_ptr(), 65,
                    );
                    let slen = ssid_local.iter().position(|&b| b == 0).unwrap_or(33);
                    let plen = pass_local.iter().position(|&b| b == 0).unwrap_or(65);
                    (
                        String::from_utf8_lossy(&ssid_local[..slen]).into_owned(),
                        String::from_utf8_lossy(&pass_local[..plen]).into_owned(),
                    )
                }
            } else {
                unsafe {
                    let mut cfg: esp_idf_sys::wifi_config_t = core::mem::zeroed();
                    esp_idf_sys::esp_wifi_get_config(
                        esp_idf_sys::wifi_interface_t_WIFI_IF_STA,
                        &mut cfg,
                    );
                    let sta = &cfg.sta;
                    let ssid_raw = &sta.ssid;
                    let pass_raw = &sta.password;
                    let ssid_len = ssid_raw.iter().position(|&b| b == 0).unwrap_or(ssid_raw.len());
                    let pass_len = pass_raw.iter().position(|&b| b == 0).unwrap_or(pass_raw.len());
                    (
                        String::from_utf8_lossy(&ssid_raw[..ssid_len]).into_owned(),
                        String::from_utf8_lossy(&pass_raw[..pass_len]).into_owned(),
                    )
                }
            };
            unsafe {
                esp_idf_sys::esp_wifi_wps_disable();
                esp_idf_sys::esp_event_handler_unregister(
                    esp_idf_sys::WIFI_EVENT,
                    esp_idf_sys::ESP_EVENT_ANY_ID,
                    Some(wps_event_handler),
                );
            }
            info!("[WPS] Credentials received: SSID={}", ssid);
            set_status("WPS: Connecting", &format!("SSID:{}", ssid));

            wifi.set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: heapless::String::<32>::from_str(&ssid)
                    .map_err(|_| anyhow::anyhow!("WPS SSID too long"))?,
                password: heapless::String::<64>::from_str(&pass)
                    .map_err(|_| anyhow::anyhow!("WPS passphrase too long"))?,
                ..Default::default()
            })).unwrap();
            wifi.connect()?;

            let mut t = 0;
            loop {
                if wifi.is_connected().unwrap() {
                    info!("[WPS] Waiting for connection stabilization...");
                    thread::sleep(Duration::from_secs(3));
                    unsafe {
                        let ret = esp_idf_sys::esp_wifi_set_ps(
                            esp_idf_sys::wifi_ps_type_t_WIFI_PS_NONE,
                        );
                        if ret == esp_idf_sys::ESP_OK as i32 {
                            info!("[WPS] WiFi power management disabled (WIFI_PS_NONE)");
                        }
                    }
                    break;
                }
                thread::sleep(Duration::from_secs(1));
                t += 1;
                if t >= 15 { break; }
            }

            // WPS reports connected before DHCP converges; retry if needed.
            if wait_for_dhcp_ip(&wifi, 15, "WPS").is_none() {
                info!("[WPS] DHCP still 0.0.0.0 — restarting DHCP client");
                unsafe {
                    let handle = wifi.sta_netif().handle();
                    let _ = esp_idf_sys::esp_netif_dhcpc_stop(handle);
                    let rc = esp_idf_sys::esp_netif_dhcpc_start(handle);
                    if rc != esp_idf_sys::ESP_OK as i32 {
                        info!("[WPS] esp_netif_dhcpc_start failed: 0x{:x}", rc);
                    }
                }
                let _ = wait_for_dhcp_ip(&wifi, 20, "WPS");
            }

            return Ok((wifi, ssid, pass));
        }
    }

    unsafe {
        esp_idf_sys::esp_wifi_wps_disable();
        esp_idf_sys::esp_event_handler_unregister(
            esp_idf_sys::WIFI_EVENT,
            esp_idf_sys::ESP_EVENT_ANY_ID,
            Some(wps_event_handler),
        );
    }
    bail!("[WPS] WPS timed out after 120 s");
}

pub fn get_rssi() -> i32 {
    unsafe {
        let mut rssi: i32 = 0;
        esp_idf_sys::esp_wifi_sta_get_rssi(&mut rssi);
        rssi
    }
}
