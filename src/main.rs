// #![deny(unsafe_code)]
#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use cortex_m_rt::entry;
use embedded_hal::spi;
use embedded_nrf24l01::{Configuration, CrcMode, DataRate, NRF24L01};
use stm32f1xx_hal::{pac, prelude::*};

use stm32f1xx_hal::serial::{Config, Serial};
use stm32f1xx_hal::spi::Spi;

const ADDR: &[u8; 5] = b"1Node";
const HEX: [u8; 16] = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'A', b'B', b'C', b'D', b'E', b'F'];

#[entry]
fn main() -> ! {
    // Get access to the core peripherals from the cortex-m crate
    let core = cortex_m::Peripherals::take().unwrap();
    // Get access to the device specific peripherals from the peripheral access crate
    let device = pac::Peripherals::take().unwrap();

    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs
    let mut flash = device.FLASH.constrain();
    let rcc = device.RCC.constrain();

    defmt::info!("Loading");

    // Freeze the configuration of all the clocks in the system and store the frozen frequencies in
    // `clocks`
    let clocks = rcc.cfgr
        .sysclk(72.MHz())
        .hclk(72.MHz())
        .pclk1(36.MHz())
        .pclk2(72.MHz())
        .adcclk(12.MHz())
        .freeze(&mut flash.acr);

    defmt::info!("Clocks done");

    let mut afio = device.AFIO.constrain();

    // Setup pins
    let mut gpioa = device.GPIOA.split();
    let mut gpiob = device.GPIOB.split();

    let tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
    let rx = gpioa.pa10;
    let mut sc = Config::default();
    sc.baudrate = 115200.bps();

    let mut serial = Serial::new(device.USART1, (tx, rx), &mut afio.mapr, sc, &clocks);
    defmt::info!("Serial done");

    let ce = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);
    let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let mi = gpioa.pa6.into_pull_up_input(&mut gpioa.crl);
    let mo = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);
    let csn = gpiob.pb0.into_push_pull_output(&mut gpiob.crl);
    let spi = Spi::spi1(device.SPI1, (sck, mi, mo), &mut afio.mapr, spi::MODE_0, 10.MHz(), clocks);
    defmt::info!("Spi done");

    let mut nrf24 = NRF24L01::new(ce, csn, spi).unwrap();
    nrf24.set_frequency(76).unwrap();
    nrf24.set_auto_retransmit(5, 15).unwrap();
    nrf24.set_rf(&DataRate::R1Mbps, 3).unwrap();
    nrf24
        .set_pipes_rx_enable(&[true, false, false, false, false, false])
        .unwrap();
    nrf24
        .set_auto_ack(&[true, true, true, true, true, true])
        .unwrap();
    nrf24.set_pipes_rx_lengths(&[Some(32); 6]).unwrap();
    nrf24.set_crc(CrcMode::OneByte).unwrap();
    nrf24.set_rx_addr(1, ADDR).unwrap();
    nrf24.set_tx_addr(ADDR).unwrap();
    nrf24.flush_rx().unwrap();
    nrf24.flush_tx().unwrap();
    defmt::info!("NRF done, freq = {:?} ({})", nrf24.get_frequency().unwrap(), 76);

    let mut delay = device.TIM1.delay_us(&clocks);

    let mut tx = nrf24.tx().unwrap();

    loop {
        tx.send(b"Hello, World!").unwrap();
        tx.wait_empty().unwrap();
    }
}