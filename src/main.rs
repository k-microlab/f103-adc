// #![deny(unsafe_code)]
#![no_std]
#![no_main]

extern crate stm32f1xx_hal as hal;

use defmt_rtt as _;
use panic_probe as _;

#[rtic::app(device = hal::pac, dispatchers = [SPI1])]
mod app {
    use systick_monotonic::*; // Implements the `Monotonic` trait

    use hal::prelude::*;
    use hal::adc::{Adc, SampleTime};
    use hal::gpio::{Alternate, Analog, PB11, Pin};
    use hal::pac::{ADC1, TIM2, USART3};
    use hal::serial::{Config, Serial, TxDma3};
    use hal::timer::{CounterHz, Event, Timer};

    // A monotonic timer to enable scheduling in RTIC
    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<100>; // 100 Hz / 10 ms granularity

    const FRAME_SIZE: usize = 1024;
    const TIMER_FREQ: usize = 1024 * 16;

    // Resources shared between tasks
    #[shared]
    struct Shared {
        exchange: &'static mut [u16; FRAME_SIZE]
    }

    // Local resources to specific tasks (cannot be shared)
    #[local]
    struct Local {
        timer: CounterHz<TIM2>,
        adc: Adc<ADC1>,
        channels: (Pin<'A', 0, Analog>,),
        serial: Option<TxDma3>
    }

    #[init(local = [
        exchange: [u16; FRAME_SIZE] = [0; FRAME_SIZE]
    ])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // Get access to the core peripherals from the cortex-m crate
        let core = cx.core;
        // Get access to the device specific peripherals from the peripheral access crate
        let device = cx.device;

        // Take ownership over the raw flash and rcc devices and convert them into the corresponding
        // HAL structs
        let mut flash = device.FLASH.constrain();
        let rcc = device.RCC.constrain();

        defmt::info!("Loading");

        // Freeze the configuration of all the clocks in the system and store the frozen frequencies in
        // `clocks`
        let clocks = rcc.cfgr
            .sysclk(72.MHz())
            .use_hse(8.MHz())
            .hclk(72.MHz())
            .pclk1(36.MHz())
            .pclk2(72.MHz())
            .freeze(&mut flash.acr);

        let systick = core.SYST;

        // Initialize the monotonic (SysTick rate is 72 MHz)
        let mono = Systick::new(systick, 72_000_000);

        let mut afio = device.AFIO.constrain();

        // Setup pins
        let mut gpioa = device.GPIOA.split();
        let mut gpiob = device.GPIOB.split();

        let dma = device.DMA1.split();
        let mut dma_ch1 = dma.1;
        let mut dma_ch2 = dma.2;

        let mut adc = Adc::adc1(device.ADC1, clocks);
        adc.set_sample_time(SampleTime::T_13);

        let tx = gpiob.pb10.into_alternate_push_pull(&mut gpiob.crh);
        let rx = gpiob.pb11;
        let mut sc = Config::default();
        sc.baudrate = 1024000.bps();

        let serial = Serial::new(device.USART3, (tx, rx), &mut afio.mapr, sc, &clocks);
        let (tx, rx) = serial.split();
        let tx = tx.with_dma(dma_ch2);

        let mut timer = Timer::new(device.TIM2, &clocks).counter_hz();
        timer.start((TIMER_FREQ as u32).Hz()).unwrap();
        timer.listen(Event::Update);

        // Configure pa0 as an analog input
        let mut adc_ch0 = gpioa.pa0.into_analog(&mut gpioa.crl);
        let mut adc_ch1 = gpioa.pa1.into_analog(&mut gpioa.crl);
        let mut adc_ch2 = gpioa.pa2.into_analog(&mut gpioa.crl);

        (
            // Initialization of shared resources
            Shared { exchange: cx.local.exchange },
            // Initialization of task local resources
            Local { timer, adc, channels: (adc_ch0,), serial: Some(tx) },
            // Move the monotonic timer to the RTIC run-time, this enables
            // scheduling
            init::Monotonics(mono),
        )
    }

    // Background task, runs whenever no other tasks are running
    #[idle]
    fn idle(cx: idle::Context) -> ! {
        loop {
            continue;
        }
    }

    #[task(binds = TIM2, priority = 3, local = [
        timer, adc, channels,
        buffer: [u16; FRAME_SIZE] = [0; FRAME_SIZE],
        cursor: usize = 0
    ], shared = [exchange])]
    fn timer_tick(mut cx: timer_tick::Context) {
        let timer = cx.local.timer;
        timer.clear_interrupt(Event::Update);

        let adc = cx.local.adc;
        let channels = cx.local.channels;

        let reading: u16 = nb::block!(adc.read(&mut channels.0)).unwrap();
        let reading = (reading as u32 * 3300 / 4096) as u16;

        // defmt::println!("read {}", reading);

        let cursor = cx.local.cursor;
        let buffer = cx.local.buffer;

        buffer[*cursor] = reading;
        *cursor += 1;
        if *cursor >= FRAME_SIZE {
            *cursor = 0;
            cx.shared.exchange.lock(|b| {
                core::mem::swap(buffer, b);
            });
            send::spawn().unwrap();
        }
    }

    #[task(local = [
        serial,
        buffer: [u16; FRAME_SIZE] = [0; FRAME_SIZE]
    ], shared = [exchange])]
    fn send(mut cx: send::Context) {
        let serial = cx.local.serial.take().unwrap();
        let buffer = cx.local.buffer;

        cx.shared.exchange.lock(|b| {
            core::mem::swap(buffer, b);
        });

        // defmt::println!("tx start");
        let transfer = serial.write(unsafe { core::mem::transmute::<_, &mut [u8; 2 * FRAME_SIZE]>(buffer) });
        let (_, serial) = transfer.wait();
        // defmt::println!("tx done");

        let _ = cx.local.serial.insert(serial);
    }
}