//! Drive two sets of WS2812 LED modules.
#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::ws2812::{Grb, PioWs2812, PioWs2812Program, Rgb};
use embassy_time::{Delay, Duration, Instant, Ticker};
use embedded_hal_async::delay::DelayNs;
use fixed::types::U16F16;
use smart_leds::RGB8;
use smart_leds::hsv::{Hsv, hsv2rgb};
use {defmt_rtt as _, panic_probe as _};

mod colors;

use colors::BLACK;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

/// Flash the onboard LED at this rate in Hz.
///
/// Each flash lasts for [`NS_PER_60HZ`].
const LED_HZ: u64 = 1;
/// The number of nanoseconds in one interval of a 60Hz signal.
const NS_PER_60HZ: u32 = 16_666_667;

/// The length of the LED strand on PIN3. (Uses the Grb color format.)
const STRAND_LEN: usize = 20;
/// The dimming factor for the LED strand on PIN3.
const STRAND_DIM: u8 = 1;
/// The length of the LED strip on PIN2
const STRIP_LEN: usize = 24;
/// The dimming factor for the LED strip on PIN2
const STRIP_DIM: u8 = 8;

#[embassy_executor::task]
async fn toggle_led(mut led: Output<'static>, interval: Duration) {
    info!(
        "task `toggle_led` started in {}us",
        Instant::now().as_micros()
    );
    let mut ticker = Ticker::every(interval);
    loop {
        led.set_high();
        Delay.delay_ns(NS_PER_60HZ).await;
        led.set_low();
        ticker.next().await;
    }
}

#[embassy_executor::task]
async fn print_runtime() {
    info!(
        "task `print_runtime` started in {}us",
        Instant::now().as_micros()
    );
    let mut ticker = Ticker::every(Duration::from_secs(5));
    loop {
        let now = Instant::now().as_micros();
        info!("Runtime: {}us", now);
        ticker.next().await;
    }
}

#[embassy_executor::task]
async fn pin2_led_strip(mut led_strip: PioWs2812<'static, PIO0, 0, STRIP_LEN, Grb>) {
    info!(
        "task `led_strip` spawned at {}us",
        Instant::now().as_micros()
    );
    let mut leds = [RGB8::default(); STRIP_LEN];
    let mut temp_12 = [RGB8::default(); STRIP_LEN / 2];
    let mut ticker = Ticker::every(Duration::from_millis(10));
    loop {
        for j in 0..255 {
            fill(&mut temp_12, j, 3u8.into());

            for i in 0..12 {
                leds[2 * i] = temp_12[i];
                leds[2 * i + 1] = temp_12[11 - i];
            }

            led_strip.write(&leds.map(|led| led / STRIP_DIM)).await;
            ticker.next().await;
        }
    }
}

#[embassy_executor::task]
async fn pin3_led_strand(mut led_strand: PioWs2812<'static, PIO0, 1, STRAND_LEN, Rgb>) {
    info!(
        "task `led_strand` spawned at {}us",
        Instant::now().as_micros()
    );
    let mut leds = [RGB8::default(); STRAND_LEN];
    let mut ticker = Ticker::every(Duration::from_millis(10));
    loop {
        for scale in 0x1_FFu16..0x3_FF {
            let scale = (scale as u32) << 4;
            let scale = U16F16::from_bits(scale);
            info!("scale={}", scale);
            for j in 0..255 {
                fill(&mut leds, j, scale);
                led_strand.write(&leds.map(|led| led / STRAND_DIM)).await;
                ticker.next().await;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Booted in {}us", Instant::now().as_micros());
    let p = embassy_rp::init(Default::default());

    let Pio {
        mut common,
        sm0,
        sm1,
        ..
    } = Pio::new(p.PIO0, Irqs);

    let led = Output::new(p.PIN_25, embassy_rp::gpio::Level::Low);

    info!("Spawning tasks: {}us", Instant::now().as_micros());
    unwrap!(spawner.spawn(toggle_led(led, Duration::from_hz(LED_HZ))));
    unwrap!(spawner.spawn(print_runtime()));

    let program = PioWs2812Program::new(&mut common);
    let mut led_strip = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_2, &program);
    let mut led_strand: PioWs2812<_, _, _, Rgb> =
        PioWs2812::with_color_order(&mut common, sm1, p.DMA_CH1, p.PIN_3, &program);

    info!("Started clearing LEDs: {}us", Instant::now().as_micros());
    let start = Instant::now();
    led_strip.write(&[BLACK; STRIP_LEN]).await;
    let middle = Instant::now();
    info!("Took {}us to clear 24 LEDs", (middle - start).as_micros());
    led_strand.write(&[BLACK; STRAND_LEN]).await;
    let end = Instant::now();
    info!("Took {}us to clear 20 LEDs", (end - middle).as_micros());
    info!("Finished clearing LEDs: {}us", Instant::now().as_micros());

    unwrap!(spawner.spawn(pin2_led_strip(led_strip)));
    unwrap!(spawner.spawn(pin3_led_strand(led_strand)));
}

fn fill(data: &mut [RGB8], j: u8, scale: U16F16) {
    let len = U16F16::from(data.len() as u16);
    let len = scale * len;
    let len: u16 = len.to_num();
    for (i, led) in data.iter_mut().enumerate() {
        let hue = (i as u16 * 256u16 / len) as u8;
        let hue = hue.wrapping_add(j);
        let hsv = Hsv {
            hue,
            sat: 255,
            val: 255,
        };
        *led = hsv2rgb(hsv);
    }
}
