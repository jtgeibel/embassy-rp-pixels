use embassy_rp::gpio;
use embassy_time::Instant;
use infrared::{
    Receiver,
    protocol::{Nec, nec::NecCommand},
    remotecontrol::{Button, nec::SpecialForMp3},
};

use defmt::info;

#[embassy_executor::task]
pub(crate) async fn infrared_remote(
    mut ir_receiver: Receiver<Nec, gpio::Input<'static>, u64, Button<SpecialForMp3, NecCommand>>,
) {
    info!(
        "task `infrared_remote` started in {}us",
        Instant::now().as_micros()
    );

    loop {
        // FIXME: Must be run in release mode for this to work. Move into an interrupt handler.
        ir_receiver.pin_mut().wait_for_any_edge().await;

        let now = Instant::now().as_micros();
        if let Ok(Some(cmd)) = ir_receiver.event_instant(now) {
            if let Some(action) = cmd.action() {
                info!("ir command: {}", action.to_str())
            } else {
                info!("ir command: action is None")
            }
        }
    }
}
