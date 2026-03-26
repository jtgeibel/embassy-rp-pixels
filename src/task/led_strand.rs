use defmt::info;
use embassy_futures::select::{Either, select};
use embassy_rp::{
    adc, peripherals,
    pio_programs::rotary_encoder::{Direction, PioEncoder},
    pio_programs::ws2812::{PioWs2812, Rgb},
};
use embassy_time::{Duration, Instant, Ticker};
use smart_leds::{
    RGB8, gamma,
    hsv::{Hsv, hsv2rgb},
};

use crate::STRAND_LEN;

const REFRESH_RATE: Duration = Duration::from_hz(100);

#[embassy_executor::task]
pub(crate) async fn pin17_led_strand(
    mut led_strand: PioWs2812<'static, peripherals::PIO0, 1, STRAND_LEN, Rgb>,
    mut adc: adc::Adc<'static, adc::Async>,
    mut adc_pin: adc::Channel<'static>,
    mut encoder: PioEncoder<'static, peripherals::PIO0, 2>,
) {
    info!(
        "task `pin3_led_strand` spawned at {}us",
        Instant::now().as_micros()
    );
    let mut leds = [RGB8::default(); STRAND_LEN];
    let mut ticker = Ticker::every(REFRESH_RATE);
    let mut shift = 127u8;
    let mut j = 0u8;
    loop {
        match select(encoder.read(), ticker.next()).await {
            Either::First(direction) => update_phase_shift(&mut shift, direction),
            Either::Second(_) => {
                let sat = adc.read(&mut adc_pin).await.unwrap();
                let sat = (sat >> 4) as u8;
                render(&mut leds, shift, j, sat);
                led_strand.write(&leds).await;

                j = j.wrapping_add(1);
            }
        }
    }
}

fn render(leds: &mut [RGB8; STRAND_LEN], shift: u8, j: u8, sat: u8) {
    let gen_color = gamma(
        (0..)
            .map(|i: u16| {
                let hue = (i * shift as u16) + j as u16;
                let hue = hue as u8;
                Hsv { hue, sat, val: 255 }
            })
            .map(hsv2rgb),
    );

    for (led, color) in leds.iter_mut().zip(gen_color) {
        *led = color;
    }
}

fn update_phase_shift(shift: &mut u8, direction: Direction) {
    let delta = match (&shift, &direction) {
        // Handle the range boundaries, where rotation direction determines step size.
        (7 | 135, Direction::CounterClockwise) => 1u8,
        (7 | 135, Direction::Clockwise) => 4,
        (119 | 247, Direction::CounterClockwise) => 4,
        (119 | 247, Direction::Clockwise) => 1,
        // Handle the ranges between boundaries.
        (0..7 | 120..135 | 248.., _) => 1,
        (8..119 | 136..247, _) => 4,
    };
    match direction {
        Direction::Clockwise => *shift = shift.wrapping_add(delta),
        Direction::CounterClockwise => *shift = shift.wrapping_sub(delta),
    }
    info!("phase shift: {}", shift);
}
