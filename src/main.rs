//! Drive two sets of WS2812 LED modules.
#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program, Rgb};
use embassy_rp::{adc, bind_interrupts, dma, gpio, peripherals, pio, uart};
use embassy_time::Instant;
use smart_leds::RGB8;
use {defmt_rtt as _, panic_probe as _};

mod task;

use task::*;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<peripherals::PIO0>;
    ADC_IRQ_FIFO => adc::InterruptHandler;
    DMA_IRQ_0 => dma::InterruptHandler<peripherals::DMA_CH0>,
        dma::InterruptHandler<peripherals::DMA_CH1>,
        dma::InterruptHandler<peripherals::DMA_CH2>,
        dma::InterruptHandler<peripherals::DMA_CH3>;
    UART0_IRQ => uart::InterruptHandler<peripherals::UART0>;
});

/// The length of the LED strand on PIN3. (Uses the Grb color format.)
const STRAND_LEN: usize = 20;
/// The length of the LED strip on PIN2
const STRIP_LEN: usize = 24;
/// The hsv value component for the LED strip on PIN2
const STRIP_BRIGHTNESS: u8 = 0x7F;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Booted in {}us", Instant::now().as_micros());
    let p = embassy_rp::init(Default::default());

    let pio::Pio {
        mut common,
        sm0,
        sm1,
        ..
    } = pio::Pio::new(p.PIO0, Irqs);

    let led = gpio::Output::new(p.PIN_25, embassy_rp::gpio::Level::Low);
    let adc = adc::Adc::new(p.ADC, Irqs, adc::Config::default());
    let p26 = adc::Channel::new_pin(p.PIN_26, gpio::Pull::None);

    let uart0 = uart::Uart::new(
        p.UART0,
        p.PIN_0,
        p.PIN_1,
        Irqs,
        p.DMA_CH2,
        p.DMA_CH3,
        uart::Config::default(),
    );

    info!("Spawning tasks: {}us", Instant::now().as_micros());
    spawner.spawn(unwrap!(blink_led(led)));
    spawner.spawn(unwrap!(uart_terminal(uart0)));

    let program = PioWs2812Program::new(&mut common);
    let mut led_strip = PioWs2812::new(&mut common, sm0, p.DMA_CH0, Irqs, p.PIN_2, &program);
    let mut led_strand: PioWs2812<_, _, _, Rgb> =
        PioWs2812::with_color_order(&mut common, sm1, p.DMA_CH1, Irqs, p.PIN_3, &program);

    info!("Started clearing LEDs: {}us", Instant::now().as_micros());
    let start = Instant::now();
    led_strip.write(&[RGB8::default(); STRIP_LEN]).await;
    let middle = Instant::now();
    info!("Took {}us to clear 24 LEDs", (middle - start).as_micros());
    led_strand.write(&[RGB8::default(); STRAND_LEN]).await;
    let end = Instant::now();
    info!("Took {}us to clear 20 LEDs", (end - middle).as_micros());
    info!("Finished clearing LEDs: {}us", Instant::now().as_micros());

    spawner.spawn(unwrap!(pin2_led_strip(led_strip)));
    spawner.spawn(unwrap!(pin3_led_strand(led_strand, adc, p26)));
}
