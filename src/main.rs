mod modes;
mod procon;

use std::time::Duration;
use std::thread;

use modes::ProconMode;
use procon::ProController;

fn main() -> Result<(), std::io::Error> {
    let mut current_mode = ProconMode::CONFIGURING;
    loop {
        let mut controllers: Vec<ProController> = Vec::new();

        match current_mode {
            ProconMode::CONFIGURING => {
                println!("Configuring controllers...");
                controllers = modes::configure()?;
                if controllers.len() == 0 {
                    current_mode = ProconMode::DORMANT;
                    continue;
                }

                current_mode = ProconMode::OPERATING;
            },
            ProconMode::OPERATING => {
                modes::operate(&controllers)?
            },
            ProconMode::DORMANT => {
                println!("Dormant.");
                thread::sleep(Duration::from_secs(60));
                if modes::configuring::get_devices()?.len() > 0 {
                    current_mode = ProconMode::CONFIGURING;
                }
            },
        }
    }

    Ok(())
}

// Todo: Grab and lock device exposed by driver.
// Todo: Expose binding to put procond into "Player choose" mode, where players can be assigned.
// Todo: Create virtual device(s) depending on player mapping.
// Todo: Support 2 JoyCons working as one controller.