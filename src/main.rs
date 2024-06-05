// #![deny(unsafe_code)]
#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use cortex_m_rt::entry;
use embedded_hal::spi;
use embedded_nrf24l01::{Configuration, CrcMode, DataRate, NRF24L01};
use stm32f1xx_hal::{pac, prelude::*};
use stm32f1xx_hal::gpio::PinState;
use stm32f1xx_hal::i2c::{BlockingI2c, DutyCycle, Mode};

use stm32f1xx_hal::serial::{Config, Serial};
use stm32f1xx_hal::spi::Spi;

mod max30100;

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

    let mut delay = core.SYST.delay(&clocks);

    let mut afio = device.AFIO.constrain();

    // Setup pins
    let mut gpioa = device.GPIOA.split();
    let mut gpiob = device.GPIOB.split();

    let tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
    let rx = gpioa.pa10;
    let mut sc = Config::default();
    sc.baudrate = 1024000.bps();

    let mut serial = Serial::new(device.USART1, (tx, rx), &mut afio.mapr, sc, &clocks);
    defmt::info!("Serial done");

    let rd = gpiob.pb11.into_push_pull_output_with_state(&mut gpiob.crh, PinState::High);
    let ird = gpiob.pb10.into_push_pull_output_with_state(&mut gpiob.crh, PinState::High);
    let scl = gpiob.pb8.into_alternate_open_drain(&mut gpiob.crh);
    let sda = gpiob.pb9.into_alternate_open_drain(&mut gpiob.crh);

    let i2c = BlockingI2c::i2c1(
        device.I2C1,
        (scl, sda),
        &mut afio.mapr,
        Mode::Fast {
            frequency: 400.kHz(),
            duty_cycle: DutyCycle::Ratio2to1,
        },
        clocks,
        1000,
        10,
        1000,
        1000,
    );

    defmt::info!("I2C done");

    let mut max = max30100::Max30100::new(i2c, max30100::Config::default()).expect("max");
    defmt::info!("config init");

    loop {
        // max.read_temperature().expect("read");
        // delay.delay_ms(100u32);
        // let temp = max.get_temperature().expect("get");
        // defmt::info!("temp: {}", temp);
        let fifo = max.read_fifo().unwrap();
        // defmt::info!("FIFO: ir = {} / r = {}", fifo.infrared, fifo.red);
        serial.bwrite_all(&u16::to_ne_bytes(fifo.infrared)).unwrap();
        serial.bwrite_all(&u16::to_ne_bytes(fifo.red)).unwrap();
    }
}