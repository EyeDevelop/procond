pub mod operating;
pub mod configuring;

use crate::procon::ProController;

#[derive(Debug, PartialEq, Eq)]
pub enum ProconMode {
    OPERATING,
    CONFIGURING,
    DORMANT,
}

impl std::fmt::Display for ProconMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mode: &str = match self {
            ProconMode::OPERATING => "operating as usual",
            ProconMode::CONFIGURING => "configuring controllers",
            ProconMode::DORMANT => "dormant, no controllers connected",
        };

        write!(f, "{}", mode)
    }
}

pub fn configure() -> Result<Vec<ProController>, std::io::Error> {
    // First, we fetch all input devices that match our
    // vendor and products.
    let devices = configuring::get_devices()?;

    // Set each controller to blinking lights.
    devices.iter()
        .for_each(|device| {
            match ProController::set_led(&device, 0) {
                Err(s) => println!("Cannot set controller LED!!"),
                _ => (),
            }
        });

    // Make pairings of controllers.
    let mut controllers = configuring::make_pairings_into_controllers(devices)?;

    // Grab all of the controllers so events are not
    // passed twice to the OS.
    configuring::grab_all_controllers(&mut controllers)?;

    Ok(controllers)
}

pub fn operate(controllers: &Vec<ProController>) -> Result<(), std::io::Error> {
    operating::emulate_single(controllers)?;

    Ok(())
}