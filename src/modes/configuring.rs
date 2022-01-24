use crate::procon::ProController;

use libc;
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::vec::Vec;

use evdev_rs::enums::{EventCode, EV_KEY};
use evdev_rs::{Device, DeviceWrapper, ReadFlag};

pub fn get_devices() -> Result<Vec<evdev_rs::Device>, std::io::Error> {
    const VENDOR_NINTENDO: u16 = 0x57e;
    const PRODUCT_LEFT_JOYCON: u16 = 0x2006;
    const PRODUCT_RIGHT_JOYCON: u16 = 0x2007;
    const PRODUCT_PROCON: u16 = 0x2009;

    std::fs::read_dir("/dev/input")?
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .map(|file_name| {
            OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(file_name.path())
        })
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .map(|file| Device::new_from_file(file))
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .filter(|device| match (device.vendor_id(), device.product_id()) {
            (VENDOR_NINTENDO, PRODUCT_LEFT_JOYCON) => true,
            (VENDOR_NINTENDO, PRODUCT_RIGHT_JOYCON) => true,
            (VENDOR_NINTENDO, PRODUCT_PROCON) => true,
            _ => false,
        })
        .map(|device| Ok(device))
        .collect::<Result<Vec<Device>, _>>()
}

pub fn make_pairings_into_controllers(mut devices: Vec<Device>) -> Result<Vec<ProController>, std::io::Error> {
    let mut devices_left = devices.len();
    let mut player_id = 1;
    let mut procons: Vec<ProController> = Vec::new();

    while devices_left > 0 {
        let mut possible_left_device: Option<usize> = None;
        let mut possible_right_device: Option<usize> = None;

        while possible_left_device.is_none() || possible_right_device.is_none() {
            devices
                .iter()
                .enumerate()
                .map(|(index, device)| (index, device.next_event(ReadFlag::NORMAL)))
                .filter(|(_, event)| event.is_ok())
                .map(|(index, event)| (index, event.unwrap()))
                .for_each(|(index, event)| match event.1.event_code {
                    EventCode::EV_KEY(EV_KEY::BTN_TL) => {
                        possible_left_device = if event.1.value == 1 {
                            Some(index)
                        } else {
                            None
                        }
                    }
                    EventCode::EV_KEY(EV_KEY::BTN_TR) => {
                        possible_right_device = if event.1.value == 1 {
                            Some(index)
                        } else {
                            None
                        }
                    }
                    _ => (),
                });
        }

        let left_device: Device = devices.swap_remove(possible_left_device.unwrap());

        let right_device: Option<Device> =
            if possible_left_device.unwrap() == possible_right_device.unwrap() {
                None
            } else if possible_left_device.unwrap() < possible_right_device.unwrap() {
                Some(devices.swap_remove(possible_right_device.unwrap() - 1))
            } else {
                Some(devices.swap_remove(possible_right_device.unwrap()))
            };

        // Sanity check, ProCons can only bind with themselves.
        const PROCON_PRODUCT: u16 = 0x2009;
        if right_device.is_some()
            && (left_device.product_id() == PROCON_PRODUCT
                || right_device.as_ref().unwrap().product_id() == PROCON_PRODUCT)
        {
            println!("Not matching ProCon with non-ProCon!");
            devices.push(left_device);
            devices.push(right_device.unwrap());
            continue;
        }

        // Update the amount of devices left.
        devices_left -= 1;
        if right_device.is_some() {
            devices_left -= 1;
        }

        if let Some(left_name) = &left_device.name() {
            if right_device.is_some() {
                if let Some(right_name) = right_device.as_ref().unwrap().name() {
                    println!("Matching {} with {}", left_name, right_name);
                }
            } else {
                println!("Matching {}", left_name);
            }
        }

        let mut procon: ProController = ProController::from(left_device, right_device)?;
        procon.set_player_id(player_id as u8)?;
        procons.push(procon);
        player_id += 1;
    }

    Ok(procons)
}

pub fn grab_all_controllers(controllers: &mut Vec<ProController>) -> Result<(), std::io::Error> {
    for controller in controllers.iter_mut() {
        controller.grab()?;
    }

    Ok(())
}

pub fn ungrab_all_controllers(controllers: &mut Vec<ProController>) -> Result<(), std::io::Error> {
    for controller in controllers.iter_mut() {
        controller.ungrab()?;
    }

    Ok(())
}