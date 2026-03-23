use defmt::info;
use embassy_rp::{
    peripherals,
    pio_programs::ws2812::{Grb, PioWs2812},
};
use embassy_time::{Duration, Instant, Ticker};
use smart_leds::{
    RGB8, //gamma,
    hsv::{Hsv, hsv2rgb},
};

use crate::{STRIP_BRIGHTNESS, STRIP_LEN};

#[embassy_executor::task]
pub(crate) async fn pin2_led_strip(
    mut led_strip: PioWs2812<'static, peripherals::PIO0, 0, STRIP_LEN, Grb>,
) {
    info!(
        "task `led_strip` spawned at {}us",
        Instant::now().as_micros()
    );
    let mut leds = [RGB8::default(); STRIP_LEN];
    let mut temp_12 = [RGB8::default(); STRIP_LEN / 2];
    let mut ticker = Ticker::every(Duration::from_millis(10));
    loop {
        for j in 0..255 {
            let len = temp_12.len();
            for (i, led) in temp_12.iter_mut().enumerate() {
                let hue = (i as u16 * 256u16 / len as u16) as u8;
                let hue = hue.wrapping_add(j);
                let hsv = Hsv {
                    hue,
                    sat: 255,
                    val: STRIP_BRIGHTNESS,
                };
                *led = hsv2rgb(hsv);
            }

            for i in 0..12 {
                leds[2 * i] = temp_12[i];
                leds[2 * i + 1] = temp_12[11 - i];
            }

            led_strip.write(&leds).await;
            ticker.next().await;
        }
    }
}
