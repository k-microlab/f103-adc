#![allow(clippy::empty_loop)]
#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_probe as _;

#[rtic::app(device = stm32f1xx_hal::pac)]
mod app {
    use core::sync::atomic::{AtomicU32, Ordering};
    use cortex_m::peripheral::NVIC;
    use stm32f1xx_hal::{
        gpio::{gpioa::PA0, Edge, ExtiPin, Input},
        prelude::*,
    };
    use stm32f1xx_hal::device::TIM1;
    use stm32f1xx_hal::gpio::{Floating, IOPinSpeed, Output, OutputSpeed, PA15, PC13};
    use stm32f1xx_hal::pac::Interrupt;
    use stm32f1xx_hal::timer::PwmChannel;

    static TICKS: AtomicU32 = AtomicU32::new(0);

    #[shared]
    struct Shared {
    }

    #[local]
    struct Local {
        button: PA15<Input<Floating>>,
        led: PC13<Output>,
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
            NVIC::unmask(Interrupt::EXTI15_10);
        }

        let mut gpioa = ctx.device.GPIOA.split();
        let mut gpiob = ctx.device.GPIOB.split();
        let mut gpioc = ctx.device.GPIOC.split();

        let (pa15, pb3, pb4) = afio.mapr.disable_jtag(gpioa.pa15, gpiob.pb3, gpiob.pb4);

        let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
        led.set_speed(&mut gpioc.crh, IOPinSpeed::Mhz50);

        let mut button = pa15.into_floating_input(&mut gpioa.crh);
        button.make_interrupt_source(&mut afio);
        button.enable_interrupt(&mut exti);
        button.trigger_on_edge(&mut exti, Edge::RisingFalling);
        defmt::println!("init");

        (Shared { }, Local { button, led })
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {

        }
    }

    #[task(binds = EXTI15_10, local = [button, led], priority = 2)]
    fn pps_tick(ctx: pps_tick::Context) {
        ctx.local.button.clear_interrupt_pending_bit();
        let timer: TIM1 = unsafe { core::mem::transmute(()) };
        timer.cr1.write(|w| w.cen().clear_bit());
        timer.ccr1().write(|w| w.ccr().variant(0));
        ctx.local.led.toggle();
    }
}