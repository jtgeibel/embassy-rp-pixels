//! Drive two sets of WS2812 LED modules.
#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Output, Pull};
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::ws2812::{Grb, PioWs2812, PioWs2812Program, Rgb};
use embassy_rp::{adc, bind_interrupts};
use embassy_time::{Delay, Duration, Instant, Ticker};
use embedded_hal_async::delay::DelayNs;
use fixed::types::U24F8;
use smart_leds::hsv::{Hsv, hsv2rgb};
use smart_leds::{RGB8, gamma};
use {defmt_rtt as _, panic_probe as _};

mod colors;

use colors::BLACK;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    ADC_IRQ_FIFO => adc::InterruptHandler;
});

/// Flash the onboard LED at this rate in Hz.
///
/// Each flash lasts for [`NS_PER_60HZ`].
const LED_HZ: u64 = 1;
/// The number of nanoseconds in one interval of a 60Hz signal.
const NS_PER_60HZ: u32 = 16_666_667;

/// The length of the LED strand on PIN3. (Uses the Grb color format.)
const STRAND_LEN: usize = 20;
/// The length of the LED strip on PIN2
const STRIP_LEN: usize = 24;
/// The hsv value component for the LED strip on PIN2
const STRIP_BRIGHTNESS: u8 = 0x1F;

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

#[embassy_executor::task]
async fn pin3_led_strand(
    mut led_strand: PioWs2812<'static, PIO0, 1, STRAND_LEN, Rgb>,
    mut adc: adc::Adc<'static, adc::Async>,
    mut adc_pin: adc::Channel<'static>,
) {
    info!(
        "task `led_strand` spawned at {}us",
        Instant::now().as_micros()
    );
    let mut leds = [RGB8::default(); STRAND_LEN];
    let mut ticker = Ticker::every(Duration::from_millis(10));
    let mut debounced_step = 0x7FF;
    loop {
        for j in 0u8..255 {
            let step = adc.read(&mut adc_pin).await.unwrap();
            if (step as i32 - debounced_step).abs() > 32 {
                debounced_step = step as i32;
            }
            let step = U24F8::from_bits((debounced_step as u32) << 4);
            if j % 8 == 0 {
                info!("step={}", step);
            }
            let j = U24F8::from(j);

            let gen_color = gamma(
                (0..)
                    .map(|i: u16| {
                        let i = U24F8::from(i);
                        let hue = (i * step).wrapping_add(j);
                        let hue = hue.to_num::<u32>() as u8;
                        Hsv {
                            hue,
                            sat: 255,
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
        // }
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
    let adc = adc::Adc::new(p.ADC, Irqs, adc::Config::default());
    let p26 = adc::Channel::new_pin(p.PIN_26, Pull::None);

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
    unwrap!(spawner.spawn(pin3_led_strand(led_strand, adc, p26)));
}
