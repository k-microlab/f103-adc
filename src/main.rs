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
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

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

    let tx = gpiob.pb10.into_alternate_push_pull(&mut gpiob.crh);
    let rx = gpiob.pb11;
    let mut sc = Config::default();
    sc.baudrate = 256000.bps();

    let mut serial = Serial::new(device.USART3, (tx, rx), &mut afio.mapr, sc, &clocks);

    // Configure pa0 as an analog input
    let mut adc_ch0 = gpioa.pa0.into_analog(&mut gpioa.crl);

    // let mut timer = dp.TIM3.delay_us(&clocks);

    let mut buffer = singleton!(: [u16; 1024] = [0; 1024]).unwrap().as_mut_slice();
    let mut sending = singleton!(: [u16; 1024] = [0; 1024]).unwrap().as_mut_slice();

    defmt::info!("vref");

    let vref = adc.read_vref();

    defmt::info!("measure");

    loop {
        let val: u16 = nb::block!(adc.read(&mut adc_ch0)).unwrap();
        serial.bwrite_all(val.to_ne_bytes().as_slice()).unwrap();
    }

    let (mut s_tx, _) = serial.split();

    loop {
        let v: u16 = nb::block!(adc.read(&mut adc_ch0)).unwrap();
        if v > 500 / vref {
            let adc_dma = adc.with_dma(adc_ch0, dma_ch1);

            let (buf, adc_dma) = adc_dma.read(buffer).wait();

            for c in &mut *buf {
                *c = v;
            }

            let dma_tx = s_tx.with_dma(dma_ch2);

            let (buf, tx_dma) = dma_tx.write(bytemuck::cast_slice_mut::<u16, u8>(buf)).wait();

            let (tx, tch) = tx_dma.release();

            // Consumes the AdcDma struct, restores adc configuration to previous state and returns the
            // Adc struct in normal mode.
            let (a, ach, dch) = adc_dma.split();

            adc = a;
            adc_ch0 = ach;
            dma_ch1 = dch;
            dma_ch2 = tch;
            buffer = bytemuck::cast_slice_mut::<u8, u16>(buf);
            s_tx = tx;
            // timer.delay_us(297)
        }
    }
}