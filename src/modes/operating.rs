use crate::procon::ProController;

pub fn emulate_single(controllers: &Vec<ProController>) -> Result<(), std::io::Error> {
    for controller in controllers.iter() {
        controller.parse_single_event()?;
    }

    Ok(())
}