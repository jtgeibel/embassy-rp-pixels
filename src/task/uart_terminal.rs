use defmt::info;
use embassy_rp::uart;
use embassy_time::Instant;

type Uart = uart::Uart<'static, uart::Async>;

static HEX_DIGITS: &[u8] = b"0123456789ABCDEF";

#[embassy_executor::task]
pub(crate) async fn uart_terminal(mut uart: Uart) {
    info!(
        "task `uart_terminal` started in {}us",
        Instant::now().as_micros()
    );
    loop {
        let mut buffer = [0u8];
        let _ = uart.read(&mut buffer).await;

        match buffer[0] {
            b'u' => echo_uptime(&mut uart).await,
            c @ (b'\n' | b'\r' | 0x03) => echo_newline(&mut uart, c).await,
            c => info!("uart rx: {:?}", c),
        }
    }
}

async fn echo_uptime(uart: &mut Uart) {
    let now = Instant::now();
    info!("uart rx: uptime: {}us", now.as_micros());
    let _ = uart.write(b"Uptime: ").await;
    for byte in now.as_secs().to_be_bytes() {
        let low = byte & 0x0F;
        let high = byte >> 4;
        let _ = uart.write(&[HEX_DIGITS[high as usize]]).await;
        let _ = uart.write(&[HEX_DIGITS[low as usize]]).await;
    }
    let _ = uart.write(b" seconds\n").await;
}

async fn echo_newline(uart: &mut Uart, c: u8) {
    info!("uart: rx: {}, echoing newline", c);
    uart.write(b"\n").await.unwrap()
}
