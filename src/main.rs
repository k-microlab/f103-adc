// #![deny(unsafe_code)]
#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use cortex_m::singleton;
use cortex_m_rt::entry;
use stm32f1xx_hal::{pac, prelude::*};

use stm32f1xx_hal::adc::{Adc, SampleTime};
use stm32f1xx_hal::serial::{Config, Serial};

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

    /*// Acquire the GPIOC peripheral
    let mut gpioc = dp.GPIOC.split();

    // Configure gpio C pin 13 as a push-pull output. The `crh` register is passed to the function
    // in order to configure the port. For pins 0-7, crl should be passed instead.
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    // Configure the syst timer to trigger an update every second
    let mut timer = Timer::syst(cp.SYST, &clocks).counter_hz();
    timer.start(1.Hz()).unwrap();

    // Wait for the timer to trigger an update and change the state of the LED
    loop {
        block!(timer.wait()).unwrap();
        led.set_high();
        block!(timer.wait()).unwrap();
        led.set_low();
    }*/

    // Setup pins
    let mut gpioa = device.GPIOA.split();
    let mut gpiob = device.GPIOB.split();

    let dma = device.DMA1.split();
    let mut dma_ch1 = dma.1;
    let mut dma_ch2 = dma.2;

    let mut adc = Adc::adc1(device.ADC1, clocks);
    adc.set_sample_time(SampleTime::T_1);

    let tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
    let rx = gpioa.pa10;
    let mut sc = Config::default();
    sc.baudrate = 3000000.bps();

    let mut serial = Serial::new(device.USART1, (tx, rx), &mut afio.mapr, sc, &clocks);

    // Configure pa0 as an analog input
    let mut adc_ch0 = gpioa.pa0.into_analog(&mut gpioa.crl);
    let mut adc_ch1 = gpioa.pa1.into_analog(&mut gpioa.crl);

    // let mut timer = dp.TIM3.delay_us(&clocks);

    let mut buffer = singleton!(: [u16; 1024] = [0; 1024]).unwrap().as_mut_slice();
    let mut sending = singleton!(: [u16; 1024] = [0; 1024]).unwrap().as_mut_slice();

    defmt::info!("vref");

    let vref = adc.read_vref();

    defmt::info!("measured {:?}", vref);

    loop {
        let v0: u16 = nb::block!(adc.read(&mut adc_ch0)).unwrap();
        let v0 = v0 as i32 * 3300 / 4096 * 5000 / 3000;
        let v1: u16 = nb::block!(adc.read(&mut adc_ch1)).unwrap();
        let v1 = v1 as i32 * 3300 / 4096 * 1745 / 1000;
        let v2: i32 = (v0 - v1);
        serial.bwrite_all((v0 as i16).to_ne_bytes().as_slice()).unwrap();
        serial.bwrite_all((-(v1 as i16)).to_ne_bytes().as_slice()).unwrap();
        serial.bwrite_all((v2 as i16).to_ne_bytes().as_slice()).unwrap();
    }
}