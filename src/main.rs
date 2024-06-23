#![allow(clippy::empty_loop)]
#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_probe as _;

#[rtic::app(device = stm32f1xx_hal::pac)]
mod app {
    use core::sync::atomic::{AtomicU32, Ordering};
    use cortex_m::peripheral::NVIC;
    use cortex_m::singleton;
    use nb::block;
    use stm32f1xx_hal::{
        gpio::{gpioa::PA0, Edge, ExtiPin, Input},
        prelude::*,
    };
    use stm32f1xx_hal::adc::{Adc, SampleTime};
    use stm32f1xx_hal::device::TIM1;
    use stm32f1xx_hal::gpio::{Alternate, Analog, Floating, IOPinSpeed, Output, OutputSpeed, PA1, PA10, PA15, PA9, PC13};
    use stm32f1xx_hal::pac::{Interrupt, USART1};
    use stm32f1xx_hal::serial::{Config, Serial};
    use stm32f1xx_hal::stm32::ADC1;
    use stm32f1xx_hal::timer::{CounterHz, Event};

    static TICKS: AtomicU32 = AtomicU32::new(0);

    #[shared]
    struct Shared {
    }

    #[local]
    struct Local {
        pps: PA15<Input<Floating>>,
        led: PC13<Output>,
        adc: Adc<ADC1>,
        adc_ch0: PA0<Analog>,
        adc_ch1: PA1<Analog>,
        serial: Serial<USART1, (PA9<Alternate>, PA10)>,
        timer: CounterHz<TIM1>
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let mut afio = ctx.device.AFIO.constrain();
        let mut flash = ctx.device.FLASH.constrain();
        let mut exti = ctx.device.EXTI;
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr
            .sysclk(72.MHz())
            .use_hse(8.MHz())
            .pclk1(36.MHz())
            .pclk2(72.MHz())
            .freeze(&mut flash.acr);

        unsafe {
            NVIC::unmask(Interrupt::TIM1_CC);
            NVIC::unmask(Interrupt::TIM1_UP);
            NVIC::unmask(Interrupt::EXTI15_10);
        }

        let mut gpioa = ctx.device.GPIOA.split();
        let mut gpiob = ctx.device.GPIOB.split();
        let mut gpioc = ctx.device.GPIOC.split();

        let mut timer = ctx.device.TIM1.counter_hz(&clocks);
        timer.listen(Event::Update);
        timer.start(12800.Hz()).unwrap();

        let (pa15, pb3, pb4) = afio.mapr.disable_jtag(gpioa.pa15, gpiob.pb3, gpiob.pb4);

        let mut adc = Adc::adc1(ctx.device.ADC1, clocks);
        adc.set_sample_time(SampleTime::T_1);

        let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
        led.set_speed(&mut gpioc.crh, IOPinSpeed::Mhz50);

        let mut pps = pa15.into_floating_input(&mut gpioa.crh);
        pps.make_interrupt_source(&mut afio);
        pps.enable_interrupt(&mut exti);
        pps.trigger_on_edge(&mut exti, Edge::RisingFalling);
        defmt::println!("init");

        let tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
        let rx = gpioa.pa10;
        let mut sc = Config::default();
        sc.baudrate = 1024000.bps();

        let mut serial = Serial::new(ctx.device.USART1, (tx, rx), &mut afio.mapr, sc, &clocks);

        // Configure pa0 as an analog input
        let mut adc_ch0 = gpioa.pa0.into_analog(&mut gpioa.crl);
        let mut adc_ch1 = gpioa.pa1.into_analog(&mut gpioa.crl);

        defmt::info!("init done");

        (Shared { }, Local { pps, led, serial, adc, adc_ch0, adc_ch1, timer })
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            TICKS.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[task(binds = TIM1_UP, local = [serial, adc, adc_ch0, adc_ch1, timer])]
    fn timer(ctx: timer::Context) {
        let serial = ctx.local.serial;
        let adc = ctx.local.adc;
        let adc_ch0 = ctx.local.adc_ch0;
        let adc_ch1 = ctx.local.adc_ch1;
        let timer = ctx.local.timer;
        block!(timer.wait());

        let v0: u16 = block!(adc.read(adc_ch0)).unwrap();
        let v0 = v0 as i32 * 3300 / 4096 * 5000 / 3000;
        let v1: u16 = block!(adc.read(adc_ch1)).unwrap();
        let v1 = v1 as i32 * 3300 / 4096 * 1745 / 1000;
        let v2: i32 = v0 - v1;
        serial.bwrite_all((v0 as i16).to_ne_bytes().as_slice()).unwrap();
        serial.bwrite_all((-(v1 as i16)).to_ne_bytes().as_slice()).unwrap();
        serial.bwrite_all((v2 as i16).to_ne_bytes().as_slice()).unwrap();
    }

    #[task(binds = EXTI15_10, local = [pps, led], priority = 2)]
    fn pps_tick(ctx: pps_tick::Context) {
        let timer: TIM1 = unsafe { core::mem::transmute(()) };
        timer.cr1.write(|w| w.cen().clear_bit());
        timer.ccr1().write(|w| w.ccr().variant(0));
        ctx.local.pps.clear_interrupt_pending_bit();
        ctx.local.led.toggle();
    }
}