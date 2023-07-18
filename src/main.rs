// #![deny(unsafe_code)]
#![no_std]
#![no_main]

extern crate stm32f1xx_hal as hal;

use defmt_rtt as _;
use panic_probe as _;

use hal::prelude::*;
use hal::spi::Spi;
use embedded_hal::spi::{Mode, Phase, Polarity};
use crate::ad770x::{AD770x, Channel, ChannelConfig};

mod ad770x;

pub const MODE: Mode = Mode {
    polarity: Polarity::IdleLow,
    phase: Phase::CaptureOnFirstTransition,
};

#[cortex_m_rt::entry]
fn main() -> ! {
    // Get access to the core peripherals from the cortex-m crate
    let core = cortex_m::peripheral::Peripherals::take().unwrap();
    // Get access to the device specific peripherals from the peripheral access crate
    let device = hal::device::Peripherals::take().unwrap();

    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs
    let mut flash = device.FLASH.constrain();
    let rcc = device.RCC.constrain();

    // defmt::info!("Loading");

    // Freeze the configuration of all the clocks in the system and store the frozen frequencies in
    // `clocks`
    let clocks = rcc.cfgr
        .sysclk(48.MHz()) // Delay timer crash on 72 MHz
        .use_hse(8.MHz())
        .hclk(72.MHz())
        .pclk1(36.MHz())
        .pclk2(72.MHz())
        .freeze(&mut flash.acr);

    let systick = core.SYST;

    let mut afio = device.AFIO.constrain();

    // Setup pins
    let mut gpioa = device.GPIOA.split();
    let mut gpiob = device.GPIOB.split();

    let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);

    defmt::println!("SPI init");

    let spi = Spi::spi1(
        device.SPI1,
        (sck, miso, mosi),
        &mut afio.mapr,
        MODE,
        1.MHz(),
        clocks,
    );
    let nss = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);

    // defmt::println!("Timer init");
    let delay = device.TIM1.delay_ms(&clocks);

    defmt::println!("ADC init");
    let mut ad = AD770x::new(spi, nss);

    ad.reset();
    ad.init(Channel::AIN1, ChannelConfig::default());
    ad.init(Channel::AIN2, ChannelConfig::default());

    defmt::println!("Init done");

    let r = ad.read(Channel::AIN1);
    defmt::println!("Reading: {}", r);

    defmt::println!("DONE");

    loop {
        cortex_m::asm::wfi();
    }
}