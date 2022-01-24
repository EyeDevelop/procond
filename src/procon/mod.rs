use std::time::SystemTime;
use std::io::{ErrorKind, Write};
use std::string::String;

use evdev_rs::enums::{EventCode, EventType, EV_ABS, EV_KEY, EV_SYN};
use evdev_rs::{AbsInfo, Device, DeviceWrapper, UninitDevice, UInputDevice, GrabMode, ReadFlag, ReadStatus, InputEvent, TimeVal};

pub struct ProController {
    left: Device,
    right: Option<Device>,
    virtual_controller: UInputDevice,
    player_id: u8,
}

impl ProController {
    pub fn new(
        device_left: Device,
        device_right: Option<Device>,
        virtual_controller: UInputDevice,
        player_id: u8,
    ) -> Self {
        ProController {
            left: device_left,
            right: device_right,
            virtual_controller,
            player_id,
        }
    }

    pub fn from(device_left: Device, device_right: Option<Device>) -> Result<Self, std::io::Error> {
        let virtual_device = UninitDevice::new().unwrap();
        virtual_device.set_name("ProConD Virtual Controller");
        virtual_device.set_bustype(device_left.bustype());
        virtual_device.set_vendor_id(0x0);
        virtual_device.set_product_id(0x0);

        // Enable buttons and axis.
        // First, the A/B/X/Y buttons
        virtual_device.enable(&EventType::EV_KEY)?;
        virtual_device.enable(&EventCode::EV_KEY(EV_KEY::BTN_NORTH))?;
        virtual_device.enable(&EventCode::EV_KEY(EV_KEY::BTN_SOUTH))?;
        virtual_device.enable(&EventCode::EV_KEY(EV_KEY::BTN_EAST))?;
        virtual_device.enable(&EventCode::EV_KEY(EV_KEY::BTN_WEST))?;

        // Then, HOME/BACK/START
        virtual_device.enable(&EventCode::EV_KEY(EV_KEY::BTN_START))?;
        virtual_device.enable(&EventCode::EV_KEY(EV_KEY::BTN_SELECT))?;
        virtual_device.enable(&EventCode::EV_KEY(EV_KEY::BTN_MODE))?;

        // Then, triggers and bumpers.
        virtual_device.enable(&EventCode::EV_KEY(EV_KEY::BTN_TR))?;
        virtual_device.enable(&EventCode::EV_KEY(EV_KEY::BTN_TL))?;
        virtual_device.enable(&EventCode::EV_KEY(EV_KEY::BTN_TR2))?;
        virtual_device.enable(&EventCode::EV_KEY(EV_KEY::BTN_TL2))?;

        // Now, we do axis.
        // D-Pad.
        virtual_device.enable(&EventType::EV_ABS)?;
        virtual_device.set_abs_info(
            &EventCode::EV_ABS(EV_ABS::ABS_HAT0X),
            &AbsInfo {
                minimum: -1,
                maximum: 1,
                fuzz: 0,
                flat: 0,
                value: 0,
                resolution: 0,
            },
        );
        virtual_device.set_abs_info(
            &EventCode::EV_ABS(EV_ABS::ABS_HAT0Y),
            &AbsInfo {
                minimum: -1,
                maximum: 1,
                fuzz: 0,
                flat: 0,
                value: 0,
                resolution: 0,
            },
        );

        // Analog stick data.
        const PROCON_STICK_MAX: i32 = 32767;
        const PROCON_STICK_FUZZ: i32 = 1000;
        const PROCON_STICK_FLAT: i32 = 2000;

        // Left stick.
        virtual_device.set_abs_info(
            &EventCode::EV_ABS(EV_ABS::ABS_X),
            &AbsInfo {
                minimum: -PROCON_STICK_MAX,
                maximum: PROCON_STICK_MAX,
                fuzz: PROCON_STICK_FUZZ,
                flat: PROCON_STICK_FLAT,
                value: 0,
                resolution: 0,
            },
        );
        virtual_device.set_abs_info(
            &EventCode::EV_ABS(EV_ABS::ABS_Y),
            &AbsInfo {
                minimum: -PROCON_STICK_MAX,
                maximum: PROCON_STICK_MAX,
                fuzz: PROCON_STICK_FUZZ,
                flat: PROCON_STICK_FLAT,
                value: 0,
                resolution: 0,
            },
        );

        // Right stick.
        virtual_device.set_abs_info(
            &EventCode::EV_ABS(EV_ABS::ABS_RX),
            &AbsInfo {
                minimum: -PROCON_STICK_MAX,
                maximum: PROCON_STICK_MAX,
                fuzz: PROCON_STICK_FUZZ,
                flat: PROCON_STICK_FLAT,
                value: 0,
                resolution: 0,
            },
        );
        virtual_device.set_abs_info(
            &EventCode::EV_ABS(EV_ABS::ABS_RY),
            &AbsInfo {
                minimum: -PROCON_STICK_MAX,
                maximum: PROCON_STICK_MAX,
                fuzz: PROCON_STICK_FUZZ,
                flat: PROCON_STICK_FLAT,
                value: 0,
                resolution: 0,
            },
        );

        let virtual_controller = UInputDevice::create_from_device(&virtual_device)?;
        let controller = ProController::new(device_left, device_right, virtual_controller, 0);

        controller.set_all_leds(controller.player_id)?;
        Ok(controller)
    }

    fn get_proc_base(device: &Device) -> std::option::Option<String> {
        const VENDOR_NINTENDO: u16 = 0x57e;
        const PRODUCT_LEFT_JOYCON: u16 = 0x2006;
        const PRODUCT_RIGHT_JOYCON: u16 = 0x2007;
        const PRODUCT_PROCON: u16 = 0x2009;

        return match (device.vendor_id(), device.product_id(), device.name()) {
            (VENDOR_NINTENDO, PRODUCT_LEFT_JOYCON, Some(name)) => Some(String::from(format!(
                "/proc/procon/controller{}",
                &name["Nintendo Switch Left JoyCon [controller ".len()..name.len() - 1]
            ))),

            (VENDOR_NINTENDO, PRODUCT_RIGHT_JOYCON, Some(name)) => Some(String::from(format!(
                "/proc/procon/controller{}",
                &name["Nintendo Switch Right JoyCon [controller ".len()..name.len() - 1]
            ))),

            (VENDOR_NINTENDO, PRODUCT_PROCON, Some(name)) => Some(String::from(format!(
                "/proc/procon/controller{}",
                &name["Nintendo Switch ProCon [controller ".len()..name.len() - 1]
            ))),

            _ => None,
        };
    }

    pub fn set_led(device: &Device, player_led: u8) -> Result<(), std::io::Error> {
        let proc_base = match ProController::get_proc_base(device) {
            None => {
                return Err(std::io::Error::new(
                    ErrorKind::NotFound,
                    "Cannot find proc file for controller!",
                ))
            }
            Some(name) => name,
        };

        let mut led_file = std::fs::OpenOptions::new()
            .write(true)
            .open(format!("{}/led", proc_base))?;

        led_file.write(format!("{}", player_led).as_bytes())?;

        Ok(())
    }

    pub fn set_all_leds(&self, player_led: u8) -> Result<(), std::io::Error> {
        let devices = self.get_devices();
        ProController::set_led(devices.0, player_led)?;
        
        if let Some(second_device) = devices.1 {
            ProController::set_led(second_device, player_led)?;
        }

        Ok(())
    }

    pub fn get_devices(&self) -> (&Device, Option<&Device>) {
        (&self.left, self.right.as_ref())
    }

    fn get_devices_mut(&mut self) -> (&mut Device, Option<&mut Device>) {
        (&mut self.left, self.right.as_mut())
    }

    pub fn set_player_id(&mut self, new_id: u8) -> Result<(), std::io::Error> {
        self.player_id = new_id;
        self.set_all_leds(self.player_id)?;

        Ok(())
    }

    pub fn grab(&mut self) -> Result<(), std::io::Error> {
        let devices = self.get_devices_mut();
        devices.0.grab(GrabMode::Grab)?;

        if let Some(device) = devices.1 {
            let e = device.grab(GrabMode::Grab);
            if let Err(s) = e { println!("{}", s) }
        }

        Ok(())
    }

    pub fn ungrab(&mut self) -> Result<(), std::io::Error> {
        let devices = self.get_devices_mut();
        devices.0.grab(GrabMode::Ungrab)?;

        if let Some(device) = devices.1 {
            device.grab(GrabMode::Ungrab)?;
        }

        Ok(())
    }

    pub fn parse_single_event(&self) -> Result<(), std::io::Error> {
        let devices = self.get_devices();
        let left_event = devices.0.next_event(ReadFlag::NORMAL)?;
        let right_event: Option<(ReadStatus, InputEvent)> = if let Some(device) = devices.1 {
            Some(device.next_event(ReadFlag::NORMAL)?)
        } else {
            None
        };

        self.virtual_controller.write_event(&left_event.1)?;
        if right_event.is_some() {
            self.virtual_controller.write_event(&right_event.unwrap().1)?;
        }

        let current_time = TimeVal::try_from(SystemTime::now());
        match current_time {
            Err(_) => return Err(std::io::Error::new(ErrorKind::Other, "Cannot get current time.")),
            _ => (),
        }

        self.virtual_controller.write_event(&InputEvent::new(&current_time.unwrap(), &EventCode::EV_SYN(EV_SYN::SYN_REPORT), 0))?;

        Ok(())
    }
}
