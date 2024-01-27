use anyhow::Result;
use log::*;
use wake_on_lan;

const NUM_OF_MAX_TARGET: usize = 4;

#[derive (Debug, Clone, Copy, PartialEq)]
pub enum TargetDevice {
    None,
    Device1,
    Device2,
    Device3,
    Device4,
}

#[derive (Debug, Clone, Copy)]
pub struct WakePacket {
    pub target_mac: [[u8; 6]; NUM_OF_MAX_TARGET],
}

impl WakePacket {
    pub fn new() -> Self {
        WakePacket {
            target_mac: [[0; 6]; NUM_OF_MAX_TARGET],
        }
    }

    pub fn set_target_mac(&mut self, target: TargetDevice, mac: &str) {
        let mut mac_address = [0; 6];
        mac.split(":").collect::<Vec<&str>>().iter().enumerate().for_each(|(i, v)| {
            mac_address[i] = u8::from_str_radix(v, 16).unwrap();
        });
        info!("MAC address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", 
            mac_address[0], mac_address[1], mac_address[2],
            mac_address[3], mac_address[4], mac_address[5]);
    
        match target {
            TargetDevice::Device1 => self.target_mac[0] = mac_address.clone(),
            TargetDevice::Device2 => self.target_mac[1] = mac_address.clone(),
            TargetDevice::Device3 => self.target_mac[2] = mac_address.clone(),
            TargetDevice::Device4 => self.target_mac[3] = mac_address.clone(),
            TargetDevice::None => {},
        }
    }

    pub fn send_pkt(&self, target: TargetDevice) -> Result<(), &str>{
        let mac_address : [u8; 6];
        match target {
            TargetDevice::Device1 => mac_address = self.target_mac[0],
            TargetDevice::Device2 => mac_address = self.target_mac[1],
            TargetDevice::Device3 => mac_address = self.target_mac[2],
            TargetDevice::Device4 => mac_address = self.target_mac[3],
            TargetDevice::None => return Err("Target device is not set"),
        }
        let magic_packet = wake_on_lan::MagicPacket::new(&mac_address);
        info!("Send magic packet to {:?} {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            target, 
            mac_address[0], mac_address[1], mac_address[2],
            mac_address[3], mac_address[4], mac_address[5]);
        match magic_packet.send() {
            Ok(_) => { info!("Magic packet sent successfully"); Ok(())},
            Err(e) => { info!("Magic packet sent failed: {:?}", e); Err("Magic packet sent failed")},
        }
    }
}