use defmt::info;
use embassy_rp::gpio;
use embassy_time::{Duration, Instant, Ticker, Timer};

/// The cycle rate of the onboard LED.
const FLASH_RATE: Duration = Duration::from_hz(1);
/// The duration of each pulse.
const PULSE_DURATION: Duration = Duration::from_hz(60);

/// Flash the onboard LED for one [`PULSE_DURATION`] every [`FLASH_RATE`].
#[embassy_executor::task]
pub(crate) async fn blink_led(mut led: gpio::Output<'static>) {
    info!(
        "task `blink_led` started in {}us",
        Instant::now().as_micros()
    );
    let mut ticker = Ticker::every(FLASH_RATE);
    loop {
        led.set_high();
        Timer::after(PULSE_DURATION).await;
        led.set_low();
        ticker.next().await;
    }
}
