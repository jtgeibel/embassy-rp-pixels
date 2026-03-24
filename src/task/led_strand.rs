use defmt::info;
use embassy_rp::{
    adc, peripherals,
    pio_programs::ws2812::{PioWs2812, Rgb},
};
use embassy_time::{Duration, Instant, Ticker};
use fixed::types::U24F8;
use smart_leds::{
    RGB8, gamma,
    hsv::{Hsv, hsv2rgb},
};

use crate::STRAND_LEN;

#[embassy_executor::task]
pub(crate) async fn pin17_led_strand(
    mut led_strand: PioWs2812<'static, peripherals::PIO0, 1, STRAND_LEN, Rgb>,
    mut adc: adc::Adc<'static, adc::Async>,
    mut adc_pin: adc::Channel<'static>,
) {
    info!(
        "task `pin3_led_strand` spawned at {}us",
        Instant::now().as_micros()
    );
    let mut leds = [RGB8::default(); STRAND_LEN];
    let mut ticker = Ticker::every(Duration::from_millis(10));
    let mut debounced_shift = 0x7FF;
    loop {
        for j in 0u8..255 {
            let shift = adc.read(&mut adc_pin).await.unwrap();
            let mut changed = false;
            if (shift as i32 - debounced_shift).abs() > 48 {
                debounced_shift = shift as i32;
                changed = true;
            }
            let shift = U24F8::from_bits((debounced_shift as u32) << 4);
            if changed {
                info!("shift={}", shift);
            }
            let j = U24F8::from(j);

            let gen_color = gamma(
                (0..)
                    .map(|i: u16| {
                        let i = U24F8::from(i);
                        let hue = (i * shift).wrapping_add(j);
                        let hue = hue.to_num::<u32>() as u8;
                        Hsv {
                            hue,
                            sat: 255 - 64,
                            val: 255,
                        }
                    })
                    .map(hsv2rgb),
            );

            for (led, color) in leds.iter_mut().zip(gen_color) {
                *led = color;
            }
            led_strand.write(&leds).await;
            ticker.next().await;
        }
    }
}
