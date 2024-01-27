use log::*;
use std::{thread, time::Duration, sync::Arc, sync::Mutex};
use esp_idf_hal::i2c;
use ssd1306::{I2CDisplayInterface, prelude::*, Ssd1306};
use embedded_graphics::{
    mono_font::{ascii::{FONT_10X20, FONT_5X8, FONT_6X10}, MonoTextStyle, MonoTextStyleBuilder},
    image::{Image},
    pixelcolor::BinaryColor,
    text::{Text},
    geometry::Point,
    prelude::*,
};
use tinybmp::Bmp;
use chrono::{DateTime, Utc, Local, FixedOffset};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::{MenuTable, MenuType, InputTypeChar};
use crate::keyevent::{KeyEvent};
use crate::ConfigData;

const MAIN_MENU_WIFI : usize = 0;
const MAIN_MENU_HWADDR : usize = 1;
const MAIN_MENU_SYSTEM : usize = 2;

pub enum WiFiStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(PartialEq)]
pub enum MessageTypes {
    None,
    Ready,
    Progress,
    WakeUp,
    Status,
    Error,
    Menu,
}

type DISPLAYIF<'d> = i2c::I2cDriver<'static>;

struct DisplayText {
    display_active: bool,
    main_msg: String,
    second_msg: String,
    msg_type: MessageTypes, 
    wifi_status: WiFiStatus,
    wifi_rssi: i32,
    num_send_pkt: u32,
    initial_logo: bool,
    timezone_offset: i32,
    battery_voltage: f32,
    menu_table: MenuTable,
}

pub struct DisplayPanel {
    txt: Arc<Mutex<DisplayText>>
}

impl DisplayPanel {
    pub fn new() -> DisplayPanel {
        DisplayPanel { txt: Arc::new(Mutex::new(
            DisplayText {   display_active: false,
                            second_msg: "".to_string(),
                            main_msg: "".to_string(),
                            msg_type: MessageTypes::None,
                            wifi_status: WiFiStatus::Disconnected,
                            wifi_rssi: 0,
                            num_send_pkt: 0,
                            initial_logo: false,
                            timezone_offset: 0,
                            battery_voltage: 0.0,
                            menu_table: MenuTable::new(),
                     })) }
    }

    pub fn start(&mut self, i2c : DISPLAYIF )
    {
        let txt = self.txt.clone();
        let _th = thread::spawn(move || {
            info!("Start Display Thread.");
            let interface = I2CDisplayInterface::new(i2c);        
            let mut display = Ssd1306::new(interface, 
                DisplaySize128x64,
                ssd1306::prelude::DisplayRotation::Rotate0)
                .into_buffered_graphics_mode();        
            display.init().unwrap();
            let style_large = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
            let style_large_inv = MonoTextStyleBuilder::new()
                .font(&FONT_10X20)
                .text_color(BinaryColor::Off)
                .background_color(BinaryColor::On)
                .build();
            let style_small = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
            let style_middle = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
            let style_middle_inv = MonoTextStyleBuilder::new()
                .font(&FONT_6X10)
                .text_color(BinaryColor::Off)
                .background_color(BinaryColor::On)
                .build();
            
            // Clear display
            display.clear(BinaryColor::Off).unwrap();
            display.flush().unwrap();
            // Logo BMP
            let logobmp = Bmp::from_slice(include_bytes!("./img/wakepcbox.bmp")).unwrap();
            let logo_img: Image<Bmp<BinaryColor>> = Image::new(&logobmp, Point::new(0,0));

            // Wifi BMP
            let wifibmp0 = Bmp::from_slice(include_bytes!("./img/wifi-0.bmp")).unwrap();
            let wifi_img0: Image<Bmp<BinaryColor>> = Image::new(&wifibmp0, Point::new(108,20));
            let wifibmp1 = Bmp::from_slice(include_bytes!("./img/wifi-1.bmp")).unwrap();
            let wifi_img1: Image<Bmp<BinaryColor>> = Image::new(&wifibmp1, Point::new(108,20));
            let wifibmp2 = Bmp::from_slice(include_bytes!("./img/wifi-2.bmp")).unwrap();
            let wifi_img2: Image<Bmp<BinaryColor>> = Image::new(&wifibmp2, Point::new(108,20));
            let wifibmp3 = Bmp::from_slice(include_bytes!("./img/wifi-3.bmp")).unwrap();
            let wifi_img3: Image<Bmp<BinaryColor>> = Image::new(&wifibmp3, Point::new(108,20));
            let wifibmp4 = Bmp::from_slice(include_bytes!("./img/wifi-4.bmp")).unwrap();
            let wifi_img4: Image<Bmp<BinaryColor>> = Image::new(&wifibmp4, Point::new(108,20));

            // PC sleep BMP
            let pcsleepbmp1 = Bmp::from_slice(include_bytes!("./img/pcsleep1.bmp")).unwrap();
            let pcsleep_img1: Image<Bmp<BinaryColor>> = Image::new(&pcsleepbmp1, Point::new(0,0));
            let pcsleepbmp2 = Bmp::from_slice(include_bytes!("./img/pcsleep2.bmp")).unwrap();
            let pcsleep_img2: Image<Bmp<BinaryColor>> = Image::new(&pcsleepbmp2, Point::new(0,0));
            let wakeuppcbmp = Bmp::from_slice(include_bytes!("./img/wakeup.bmp")).unwrap();
            let wakeuppc_img: Image<Bmp<BinaryColor>> = Image::new(&wakeuppcbmp, Point::new(0,0));
            // Push button BMP
            let pushbmp = Bmp::from_slice(include_bytes!("./img/pushbutton.bmp")).unwrap();
            let push_img: Image<Bmp<BinaryColor>> = Image::new(&pushbmp, Point::new(24,15));

            // Battery BMP
            let bat_x = 112;
            let bat_y = 42;
            let bat0 = Bmp::from_slice(include_bytes!("./img/battery-0.bmp")).unwrap();
            let bat0_img: Image<Bmp<BinaryColor>> = Image::new(&bat0, Point::new(bat_x, bat_y));
            let bat20 = Bmp::from_slice(include_bytes!("./img/battery-20.bmp")).unwrap();
            let bat20_img: Image<Bmp<BinaryColor>> = Image::new(&bat20, Point::new(bat_x, bat_y));
            let bat40 = Bmp::from_slice(include_bytes!("./img/battery-40.bmp")).unwrap();
            let bat40_img: Image<Bmp<BinaryColor>> = Image::new(&bat40, Point::new(bat_x, bat_y));
            let bat60 = Bmp::from_slice(include_bytes!("./img/battery-60.bmp")).unwrap();
            let bat60_img: Image<Bmp<BinaryColor>> = Image::new(&bat60, Point::new(bat_x, bat_y));
            let bat80 = Bmp::from_slice(include_bytes!("./img/battery-80.bmp")).unwrap();
            let bat80_img: Image<Bmp<BinaryColor>> = Image::new(&bat80, Point::new(bat_x, bat_y));
            let bat100 = Bmp::from_slice(include_bytes!("./img/battery-100.bmp")).unwrap();
            let bat100_img: Image<Bmp<BinaryColor>> = Image::new(&bat100, Point::new(bat_x, bat_y));
            let usbpwr = Bmp::from_slice(include_bytes!("./img/usb-power.bmp")).unwrap();
            let usbpwr_img: Image<Bmp<BinaryColor>> = Image::new(&usbpwr, Point::new(bat_x, bat_y));

            let mut loopcount = 0;
            let mut battery_level = 0;
            loop {
                let lck = txt.lock().unwrap();
                loopcount += 1;
                if loopcount > 15 {
                    loopcount = 0;
                }
                display.clear(BinaryColor::Off).unwrap();
                if !lck.display_active {
                    drop(lck);
                    match display.flush() {
                        Ok(_) => {},
                        Err(_) => {},
                    }
                    thread::sleep(Duration::from_millis(1000));
                    continue;
                }
                // Panel Top message
                if lck.initial_logo { 
                    drop(lck);
                    logo_img.draw(&mut display).unwrap();
                    display.flush().unwrap();
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
                match lck.msg_type {
                    MessageTypes::Ready => {
                        let now = SystemTime::now();
                        if now.duration_since(UNIX_EPOCH).unwrap().as_millis() > 1700000000 {
                            let dt_utc : DateTime<Utc> = now.into();
                            let fixed_offset = FixedOffset::east_opt(lck.timezone_offset * 3600).unwrap();
                            let dt_local = DateTime::<Local>::from_naive_utc_and_offset(dt_utc.naive_utc(), fixed_offset);
                            Text::new(&format!("{}", dt_local.format("%Y-%m-%d %H:%M:%S")), Point::new(1, 10), style_middle).draw(&mut display).unwrap();
                        }
                        push_img.draw(&mut display).unwrap();
                        Text::new(&lck.main_msg, Point::new(1, 60), style_middle).draw(&mut display).unwrap();
                    },
                    MessageTypes::Progress => {
                        Text::new(&lck.main_msg, Point::new(66, 15), style_large).draw(&mut display).unwrap();
                        match loopcount {
                            0..=3 => {
                                pcsleep_img1.draw(&mut display).unwrap();
                            },
                            4..=7 => {
                                pcsleep_img2.draw(&mut display).unwrap();
                            },
                            8..=11 => {
                                pcsleep_img1.draw(&mut display).unwrap();
                            },
                            12..=15 => {
                                pcsleep_img2.draw(&mut display).unwrap();
                            },
                            _ => {
                            },
                        };
                    },
                    MessageTypes::WakeUp => {
                        wakeuppc_img.draw(&mut display).unwrap();
                    },
                    MessageTypes::Status => {
                        Text::new(&lck.main_msg, Point::new(1, 30), style_middle).draw(&mut display).unwrap();
                        Text::new(&lck.second_msg, Point::new(1, 40), style_middle).draw(&mut display).unwrap();
                    },
                    MessageTypes::Menu => {
                        match lck.menu_table.get_current_level() {
                            0 => {
                                let tbl = lck.menu_table.get_menu_item_list();
                                let row = lck.menu_table.get_current_select();
                                let mut n = 0;
                                for _m in tbl {
                                    if row == n {
                                        Text::new(&lck.menu_table.get_menu_item_title(n), Point::new(8, 20 + (n * 20) as i32), style_large_inv).draw(&mut display).unwrap();
                                    }
                                    else {
                                        Text::new(&lck.menu_table.get_menu_item_title(n), Point::new(8, 20 + (n * 20) as i32), style_large).draw(&mut display).unwrap();
                                    }
                                    n += 1;
                                }
                                Text::new(">", Point::new(0, 20 + (row * 20) as i32), style_large).draw(&mut display).unwrap();        
                            },
                            1 => {
                                let sel = lck.menu_table.get_current_select();
                                let tbl = lck.menu_table.get_menu_list(sel);
                                let sel_sub = lck.menu_table.get_current_select_sub(sel);
                                let mut n = 0;
                                for _m in tbl {
                                    if sel_sub == n {
                                        Text::new(&lck.menu_table.get_menu_title(sel, n), Point::new(8, 10 + (n * 10) as i32), style_middle_inv).draw(&mut display).unwrap();
                                    }
                                    else {
                                        Text::new(&lck.menu_table.get_menu_title(sel, n), Point::new(8, 10 + (n * 10) as i32), style_middle).draw(&mut display).unwrap();
                                    }
                                    n += 1;
                                }
                                Text::new(">", Point::new(0, 10 + (sel_sub * 10) as i32), style_middle).draw(&mut display).unwrap();        
                            },
                            2 => {
                                let sel = lck.menu_table.get_current_select();
                                let sel_sub = lck.menu_table.get_current_select_sub(sel);
                                Text::new(&lck.menu_table.get_value(sel, sel_sub), Point::new(0, 30), style_middle_inv).draw(&mut display).unwrap();
                                let cursor = lck.menu_table.get_current_cursor(sel, sel_sub);
                                match lck.menu_table.get_value_type(sel, sel_sub) {
                                    InputTypeChar::SelectType => {
                                        Text::new("UP/DOWN KEY", Point::new(0, 40), style_middle).draw(&mut display).unwrap();
                                    },
                                    InputTypeChar::ActionType => {
                                        Text::new("PUSH CENTER KEY", Point::new(0, 40), style_middle).draw(&mut display).unwrap();
                                    },
                                    _ => {
                                        Text::new("^", Point::new((cursor * 6) as i32, 40), style_middle).draw(&mut display).unwrap();
                                    },   
                                }
                            },
                            _ => {},
                        }
                        if lck.menu_table.get_confirming_flag() == true {
                            Text::new("Confirm?", Point::new(0, 50), style_middle).draw(&mut display).unwrap();
                            if lck.menu_table.get_confirmed_flag() == true {
                                Text::new("Yes", Point::new(0, 60), style_middle_inv).draw(&mut display).unwrap();
                            }
                            else {
                                Text::new("No", Point::new(0, 60), style_middle_inv).draw(&mut display).unwrap();
                            }
                        }
                    },
                    MessageTypes::Error => {
                        Text::new(&lck.main_msg, Point::new(1, 40), style_large).draw(&mut display).unwrap();
                    },
                    _ => {},
                }
                if lck.msg_type != MessageTypes::Menu {
                    // Wifi status
                    match lck.wifi_status {
                        WiFiStatus::Disconnected => {
                        },
                        WiFiStatus::Connecting => {
                            match loopcount {
                                0..=2 => {
                                    wifi_img0.draw(&mut display).unwrap();
                                },
                                3..=5 => {
                                    wifi_img1.draw(&mut display).unwrap();
                                },
                                6..=8 => {
                                    wifi_img2.draw(&mut display).unwrap();
                                },
                                9..=11 => {
                                    wifi_img3.draw(&mut display).unwrap();
                                },
                                12..=15 => {
                                    wifi_img4.draw(&mut display).unwrap();
                                },
                                _ => {},
                            }
                        },
                        WiFiStatus::Connected => {
                            match lck.wifi_rssi {
                                -100..=-80 => {
                                    wifi_img0.draw(&mut display).unwrap();
                                },
                                -79..=-75 => {
                                    wifi_img1.draw(&mut display).unwrap();
                                },
                                -74..=-70 => {
                                    wifi_img2.draw(&mut display).unwrap();
                                },
                                -69..=-65 => {
                                    wifi_img3.draw(&mut display).unwrap();
                                },
                                -64..=-30 => {
                                    wifi_img4.draw(&mut display).unwrap();
                                },
                                _ => {
                                },
                            }
                            if lck.wifi_rssi != 0 {
                                Text::new(&format!("{:+02}dBm", lck.wifi_rssi), Point::new(81, 52), style_small).draw(&mut display).unwrap();
                            }
                            else {
                                Text::new("NO SIG", Point::new(81, 52), style_small).draw(&mut display).unwrap();
                            }
                        },
                    }    

                    // Battery status
                    let  battery_voltage = lck.battery_voltage;
                    Text::new(&format!("{:.1}V",  battery_voltage), Point::new(86, 60), style_small).draw(&mut display).unwrap();
                    match battery_level {
                        0 => {
                            if  battery_voltage > 3.75 {
                                battery_level = 20;
                            }
                        },
                        20 => {
                            if  battery_voltage > 3.85 {
                                battery_level = 40;
                            }
                            else if  battery_voltage < 3.7 {
                                battery_level = 0;
                            }
                        },
                        40 => {
                            if  battery_voltage > 3.95 {
                                battery_level = 60;
                            }
                            else if  battery_voltage < 3.8 {
                                battery_level = 20;
                            }
                        },
                        60 => {
                            if  battery_voltage > 4.0 {
                                battery_level = 80;
                            }
                            else if  battery_voltage < 3.9 {
                                battery_level = 40;
                            }
                        },
                        80 => {
                            if  battery_voltage > 4.05 {
                                battery_level = 100;
                            }
                            else if  battery_voltage < 3.95 {
                                battery_level = 60;
                            }
                        }
                        100 => {
                            if  battery_voltage > 4.55 {
                                battery_level = 200;
                            }
                            else if  battery_voltage < 4.0 {
                                battery_level = 80;
                            }
                        },
                        200 => {
                            if  battery_voltage < 4.5 {
                                battery_level = 100;
                            }
                        },
                        _ => {
                            battery_level = 0;
                        }
                    }
                    match battery_level {
                        0 => {
                            bat0_img.draw(&mut display).unwrap();
                        },
                        20 => {
                            bat20_img.draw(&mut display).unwrap();
                        },
                        40 => {
                            bat40_img.draw(&mut display).unwrap();
                        },
                        60 => {
                            bat60_img.draw(&mut display).unwrap();
                        },
                        80 => {
                            bat80_img.draw(&mut display).unwrap();
                        },
                        100 => {
                            bat100_img.draw(&mut display).unwrap();
                        },
                        200 => {
                            usbpwr_img.draw(&mut display).unwrap();
                        },
                        _ => {}
                    }
                }
                match display.flush(){                  
                    Ok(_) => {},
                    Err(_) => {},
                }
                drop(lck);                
                thread::sleep(Duration::from_millis(100));
            }
        });
    }

    pub fn set_second_msg(&mut self, msg: &String)
    {
        let mut lcktxt = self.txt.lock().unwrap();
        lcktxt.second_msg = msg.to_string();
    }

    pub fn set_wifi_status(&mut self, status: WiFiStatus){
        let mut lcktxt = self.txt.lock().unwrap();
        lcktxt.wifi_status = status;
    }

    pub fn set_send_pkt(&mut self, count: u32){
        let mut lcktxt = self.txt.lock().unwrap();
        lcktxt.num_send_pkt = count;
    }

    pub fn set_timezone_offset(&mut self, offset: i32){
        let mut lcktxt = self.txt.lock().unwrap();
        lcktxt.timezone_offset = offset;
    }

    pub fn set_initial_logo(&mut self, show: bool){
        let mut lcktxt = self.txt.lock().unwrap();
        lcktxt.initial_logo = show;
    }

    pub fn set_main_msg(&mut self, msg: &String, msg_type: MessageTypes)
    {
        let mut lcktxt = self.txt.lock().unwrap();
        lcktxt.main_msg = msg.to_string();
        lcktxt.msg_type = msg_type;
        lcktxt.initial_logo = false;
    }

    pub fn set_battery_voltage(&mut self, voltage: f32)
    {
        let mut lcktxt = self.txt.lock().unwrap();
        lcktxt. battery_voltage = voltage;
    }

    pub fn set_display_active(&mut self, active: bool)
    {
        let mut lcktxt = self.txt.lock().unwrap();
        lcktxt.display_active = active;
    }

    pub fn set_wifi_rssi(&mut self, rssi: i32)
    {
        let mut lcktxt = self.txt.lock().unwrap();
        lcktxt.wifi_rssi = rssi;
    }

    pub fn initialize_menu(&mut self, config_data: &ConfigData){
        // Initialize Menu
        let mut lcktxt = self.txt.lock().unwrap();
        lcktxt.menu_table.add_menu_item("WiFi");
        lcktxt.menu_table.add_menu_item("HW Address");
        lcktxt.menu_table.add_menu_item("System");
        lcktxt.menu_table.add_menu(MAIN_MENU_WIFI, "SSID", "SSID", MenuType::SubMenu, &config_data.wifi_ssid.clone(), InputTypeChar::StringType, Vec::<String>::new());
        lcktxt.menu_table.add_menu(MAIN_MENU_WIFI, "PSK", "PSK", MenuType::SubMenu, &config_data.wifi_psk.clone(), InputTypeChar::StringType, Vec::<String>::new());
        lcktxt.menu_table.add_menu(MAIN_MENU_HWADDR, "PC1", "PC1", MenuType::SubMenu, &config_data.target_mac_address1.clone(), InputTypeChar::HWAddressType, Vec::<String>::new());
        lcktxt.menu_table.add_menu(MAIN_MENU_HWADDR, "PC2", "PC2", MenuType::SubMenu, &config_data.target_mac_address2.clone(), InputTypeChar::HWAddressType, Vec::<String>::new());
        lcktxt.menu_table.add_menu(MAIN_MENU_HWADDR, "PC3", "PC3", MenuType::SubMenu, &config_data.target_mac_address3.clone(), InputTypeChar::HWAddressType, Vec::<String>::new());
        lcktxt.menu_table.add_menu(MAIN_MENU_HWADDR, "PC4", "PC4", MenuType::SubMenu, &config_data.target_mac_address4.clone(), InputTypeChar::HWAddressType, Vec::<String>::new());
        lcktxt.menu_table.add_menu(MAIN_MENU_SYSTEM, "TIME ZONE", "TIMEZONE", MenuType::SubMenu, &format!("{}{:02}", if config_data.timezone_offset >= 0 {'+'} else {'-'}, config_data.timezone_offset.abs()), InputTypeChar::TimezoneType, Vec::<String>::new());
        lcktxt.menu_table.add_menu(MAIN_MENU_SYSTEM, "IDLE TIME", "IDLESLEEP", MenuType::SubMenu, &format!("{}", config_data.idle_in_sleep_time), InputTypeChar::NumberType, Vec::<String>::new());
        lcktxt.menu_table.add_menu(MAIN_MENU_SYSTEM, "SLEEP MODE", "SLEEPMODE", MenuType::SubMenu, &format!("{}", config_data.sleep_mode), InputTypeChar::SelectType, vec!["light", "deep"].iter().map(|s| s.to_string()).collect());
        lcktxt.menu_table.add_menu(MAIN_MENU_SYSTEM, "DISPLAY OFF TIME", "DISPLAYOFFTIME", MenuType::SubMenu, &format!("{}", config_data.display_off_time), InputTypeChar::NumberType, Vec::<String>::new());
        lcktxt.menu_table.add_menu(MAIN_MENU_SYSTEM, "RESET CONFIG", "RESETCONFIG", MenuType::SubMenu, "BACK TO DEFAULT", InputTypeChar::ActionType, Vec::<String>::new());
   }

    pub fn reset_menu(&mut self)
    {
        let mut lcktxt = self.txt.lock().unwrap();
        lcktxt.menu_table.reset_menu();
    }

    pub fn key_event_input(&mut self, key: KeyEvent) -> (bool, Option<Vec<(String, String)>>)
    {
        let mut lcktxt = self.txt.lock().unwrap();
        let mut current_level = lcktxt.menu_table.get_current_level();
        let mut current_select = lcktxt.menu_table.get_current_select();
        match key {
            KeyEvent::UpKeyUp => {
                info!("Up key pressed.");
                match current_level {
                    0 => {
                        if current_select > 0 {
                            current_select -= 1;
                            lcktxt.menu_table.set_current_select(current_select);
                        }
                        else {
                            current_select = lcktxt.menu_table.get_menu_item_list().len() - 1;
                            lcktxt.menu_table.set_current_select(current_select);
                        }
                    },
                    1 => {
                        let sel_item = lcktxt.menu_table.get_current_select();
                        let mut sub_sel  = lcktxt.menu_table.get_current_select_sub(sel_item);
                        if sub_sel > 0 {
                            sub_sel -= 1;
                            lcktxt.menu_table.set_current_select_sub(sel_item, sub_sel);
                        }
                        else {
                            sub_sel = lcktxt.menu_table.get_menu_list(current_level).len() - 1;
                            lcktxt.menu_table.set_current_select_sub(sel_item, sub_sel);
                        }
                    },
                    // decrement character value in menu
                    2 => {
                        let sel = lcktxt.menu_table.get_current_select();
                        let sel_sub = lcktxt.menu_table.get_current_select_sub(sel);
                        let mut value = lcktxt.menu_table.get_value(sel, sel_sub);
                        let type_char = lcktxt.menu_table.get_value_type(sel, sel_sub);
                        if type_char == InputTypeChar::SelectType {
                            let mut sel_idx = lcktxt.menu_table.get_select_item_index(sel, sel_sub, &value);
                            if sel_idx > 0 {
                                sel_idx -= 1;
                                lcktxt.menu_table.set_current_select_item(sel, sel_sub, sel_idx);
                            }
                            else {
                                sel_idx = lcktxt.menu_table.get_select_item_count(sel, sel_sub) - 1;
                                lcktxt.menu_table.set_current_select_item(sel, sel_sub, sel_idx);
                            }
                            let new_value = lcktxt.menu_table.get_select_item(sel, sel_sub, sel_idx);
                            lcktxt.menu_table.set_value(sel, sel_sub, &new_value);
                        }
                        else {
                            let cursor = lcktxt.menu_table.get_current_cursor(sel, sel_sub);
                            let mut ch = value.chars().nth(cursor).unwrap();
                            ch = lcktxt.menu_table.inc_dec_char(false, ch, lcktxt.menu_table.get_value_type(sel, sel_sub));
                            value.replace_range(cursor..cursor+1, &ch.to_string());
                            lcktxt.menu_table.set_value(sel, sel_sub, &value);
                        }
                    },
                    _ => {},
                }
            },
            KeyEvent::DownKeyUp => {
                info!("Down key pressed.");
                match current_level {
                    0 => {
                        if current_select < lcktxt.menu_table.get_menu_item_list().len() - 1 {
                            current_select += 1;
                            lcktxt.menu_table.set_current_select(current_select);
                        }
                        else {
                            current_select = 0;
                            lcktxt.menu_table.set_current_select(current_select);
                        }
                    },
                    1 => {
                        let sel_item = lcktxt.menu_table.get_current_select();
                        let mut sub_sel  = lcktxt.menu_table.get_current_select_sub(sel_item);
                        if sub_sel < lcktxt.menu_table.get_menu_list(sel_item).len() - 1 {
                            sub_sel += 1;
                            lcktxt.menu_table.set_current_select_sub(sel_item, sub_sel);
                        }
                        else {
                            sub_sel = 0;
                            lcktxt.menu_table.set_current_select_sub(sel_item, sub_sel);
                        }
                    },
                    // increment character value in menu
                    2 => {
                        let sel = lcktxt.menu_table.get_current_select();
                        let sel_sub = lcktxt.menu_table.get_current_select_sub(sel);
                        let mut value = lcktxt.menu_table.get_value(sel, sel_sub);
                        let type_char = lcktxt.menu_table.get_value_type(sel, sel_sub);
                        if type_char == InputTypeChar::SelectType {
                            let mut sel_idx = lcktxt.menu_table.get_select_item_index(sel, sel_sub, &value);
                            if sel_idx < lcktxt.menu_table.get_select_item_count(sel, sel_sub) - 1 {
                                sel_idx += 1;
                                lcktxt.menu_table.set_current_select_item(sel, sel_sub, sel_idx);
                            }
                            else {
                                sel_idx = 0;
                                lcktxt.menu_table.set_current_select_item(sel, sel_sub, sel_idx);
                            }
                            let new_value = lcktxt.menu_table.get_select_item(sel, sel_sub, sel_idx);
                            lcktxt.menu_table.set_value(sel, sel_sub, &new_value);
                        }
                        else {
                            let cursor = lcktxt.menu_table.get_current_cursor(sel, sel_sub);
                            let mut ch = value.chars().nth(cursor).unwrap();
                            ch = lcktxt.menu_table.inc_dec_char(true, ch, lcktxt.menu_table.get_value_type(sel, sel_sub));
                            value.replace_range(cursor..cursor+1, &ch.to_string());
                            lcktxt.menu_table.set_value(sel, sel_sub, &value);
                        }
                    },
                    _ => {},
                }
            },
            KeyEvent::LeftKeyUp => {
                info!("Left key pressed.");
                match current_level {
                    0 => {
                        if lcktxt.menu_table.get_commit_flag() {
                           let config : Vec<(String, String)> = lcktxt.menu_table.get_all_values();
                            return (true, Some(config));
                        }
                        else {
                            return (true, None);
                        }
                    },
                    1 => {
                        current_level -= 1;
                        lcktxt.menu_table.set_current_level(current_level);
                    },
                    2 => {
                        if lcktxt.menu_table.get_confirming_flag() {
                            if lcktxt.menu_table.get_confirmed_flag() {
                                lcktxt.menu_table.set_confirmed_flag(false);
                            }
                            else {
                                lcktxt.menu_table.set_confirmed_flag(true);
                            }
                        }
                        else {
                            let sel = lcktxt.menu_table.get_current_select();
                            let sel_sub = lcktxt.menu_table.get_current_select_sub(sel);
                            let cursor = lcktxt.menu_table.get_current_cursor(sel, sel_sub);
                            if cursor > 0 {
                                lcktxt.menu_table.set_current_cursor(sel, sel_sub, cursor - 1);
                            }
                            else {
                                // cancel setting value
                                lcktxt.menu_table.cancel_value(sel, sel_sub);
                                lcktxt.menu_table.set_action_flag(sel, sel_sub, false);
                                current_level -= 1;
                                lcktxt.menu_table.set_current_level(current_level);
                            }
                        }
                    },
                    _ => {},
                }
            },
            KeyEvent::RightKeyUp => {
                info!("Right key pressed.");
                match current_level {
                    0 | 1 => {
                    },
                    2 => {
                        if lcktxt.menu_table.get_confirming_flag() {
                            if lcktxt.menu_table.get_confirmed_flag() {
                                lcktxt.menu_table.set_confirmed_flag(false);
                            }
                            else {
                                lcktxt.menu_table.set_confirmed_flag(true);
                            }
                        }
                        else {
                            let sel = lcktxt.menu_table.get_current_select();
                            let sel_sub = lcktxt.menu_table.get_current_select_sub(sel);
                            let mut value = lcktxt.menu_table.get_value(sel, sel_sub);
                            let cursor = lcktxt.menu_table.get_current_cursor(sel, sel_sub);
                            if cursor < value.len() - 1 {
                                lcktxt.menu_table.set_current_cursor(sel, sel_sub, cursor + 1);
                            }
                            else {
                                // add new character in end of string
                                value.push(' ');
                                lcktxt.menu_table.set_value(sel, sel_sub, &value);
                                lcktxt.menu_table.set_current_cursor(sel, sel_sub, cursor + 1);
                            }
                        }
                    },
                    _ => {},
                }
            },
            KeyEvent::CenterKeyUp => {
                info!("Enter key pressed.");
                match current_level {
                    0 | 1 => {
                        current_level += 1;
                        lcktxt.menu_table.set_current_level(current_level);
                    },
                    2 => {
                        if lcktxt.menu_table.get_confirming_flag() {
                            lcktxt.menu_table.set_confirming_flag(false);
                            if lcktxt.menu_table.get_confirmed_flag() {
                                // commit confirmed value
                                let sel = lcktxt.menu_table.get_current_select();
                                let sel_sub = lcktxt.menu_table.get_current_select_sub(sel);
                                let mut value = lcktxt.menu_table.get_value(sel, sel_sub);
                                value = value.trim().to_string();
                                lcktxt.menu_table.set_value(sel, sel_sub, &value);
                                lcktxt.menu_table.commit_value(sel, sel_sub);
                                if lcktxt.menu_table.get_value_type(sel, sel_sub) == InputTypeChar::ActionType {
                                    lcktxt.menu_table.set_action_flag(sel, sel_sub, true);
                                    if lcktxt.menu_table.get_commit_flag() {
                                        let config : Vec<(String, String)> = lcktxt.menu_table.get_all_values();
                                         return (true, Some(config));
                                     }
                                     else {
                                         return (true, None);
                                     }
                                }
                                current_level -= 1;
                                lcktxt.menu_table.set_current_level(current_level);
                            }
                            else {
                                // cancel value
                            }
                        }
                        else {
                            // set confirming flag
                            lcktxt.menu_table.set_confirming_flag(true);
                            lcktxt.menu_table.set_confirmed_flag(false);
                        }
                    }
                    _ => {},
                }
            },
            _ => {},
        }
        (false, None)
    }
}
