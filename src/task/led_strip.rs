use defmt::info;
use embassy_rp::{
    peripherals,
    pio_programs::ws2812::{Grb, PioWs2812},
};
use embassy_time::{Duration, Instant, Ticker};
use infrared::remotecontrol::Action;
use smart_leds::{
    RGB8, gamma,
    hsv::{Hsv, hsv2rgb},
};

use crate::{STRIP_BRIGHTNESS, STRIP_LEN};

const REFRESH_RATE: Duration = Duration::from_hz(100);

#[embassy_executor::task]
pub(crate) async fn pin16_led_strip(
    mut led_strip: PioWs2812<'static, peripherals::PIO0, 0, STRIP_LEN, Grb>,
) {
    info!(
        "task `led_strip` spawned at {}us",
        Instant::now().as_micros()
    );

    let mut subscriber = crate::IR_PUBSUB_CHANNEL.subscriber().unwrap();

    let mut leds = [RGB8::default(); STRIP_LEN];
    let mut temp_12 = [RGB8::default(); STRIP_LEN / 2];
    let mut ticker = Ticker::every(REFRESH_RATE);
    let mut val = STRIP_BRIGHTNESS;
    let mut sat = 255u8;
    loop {
        for j in 0..255 {
            if let Some(action) = subscriber.try_next_message_pure() {
                match action {
                    Action::Plus => val = val.saturating_add(4),
                    Action::Minus => val = val.saturating_sub(4),
                    Action::Next => sat = sat.saturating_add(4),
                    Action::Prev => sat = sat.saturating_sub(4),
                    Action::Power => {
                        val = 0;
                        sat = 255;
                    }
                    _ => (),
                }
            }
            let len = temp_12.len();
            let gen_color = gamma(
                (0..)
                    .map(|i| {
                        let hue = (i as u16 * 256u16 / len as u16) as u8;
                        let hue = hue.wrapping_add(j);
                        Hsv { hue, sat, val }
                    })
                    .map(hsv2rgb),
            );

            for (temp, color) in temp_12.iter_mut().zip(gen_color) {
                *temp = color;
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
