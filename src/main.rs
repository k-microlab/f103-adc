#![allow(clippy::empty_loop)]
#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_probe as _;

#[rtic::app(device = stm32f1xx_hal::pac)]
mod app {
    use core::sync::atomic::{AtomicU32, Ordering};
    use stm32f1xx_hal::{
        gpio::{gpioa::PA0, Edge, ExtiPin, Input},
        prelude::*,
    };
    use stm32f1xx_hal::device::TIM1;
    use stm32f1xx_hal::gpio::{Floating, IOPinSpeed, OutputSpeed};
    use stm32f1xx_hal::timer::PwmChannel;

    static TICKS: AtomicU32 = AtomicU32::new(0);

    #[shared]
    struct Shared {
    }

    #[local]
    struct Local {
        button: PA0<Input<Floating>>,
        ch0: PwmChannel<TIM1, 0>,
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

        let mut gpioa = ctx.device.GPIOA.split();
        let mut led = gpioa.pa8.into_alternate_push_pull(&mut gpioa.crh);
        led.set_speed(&mut gpioa.crh, IOPinSpeed::Mhz50);
        let etr = gpioa.pa12.into_alternate_push_pull(&mut gpioa.crh);

        let timer = ctx.device.TIM1;
        // timer.counter(&clocks).set_master_mode(MMS_A::ComparePulse)
        let timer = timer.pwm_hz(led, &mut afio.mapr, (256 * 50 / 2).Hz(), &clocks);
        let max = timer.get_max_duty();
        let mut ch0 = timer.split();
        ch0.enable();
        ch0.set_duty(max / 2);

        let mut button = gpioa.pa0.into_floating_input(&mut gpioa.crl);
        button.make_interrupt_source(&mut afio);
        button.enable_interrupt(&mut exti);
        button.trigger_on_edge(&mut exti, Edge::Rising);
        defmt::println!("init");

        (Shared { }, Local { button, ch0 })
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            TICKS.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[task(binds = EXTI0, local = [button, ch0], priority = 2)]
    fn pps_tick(ctx: pps_tick::Context) {
        ctx.local.button.clear_interrupt_pending_bit();
        let timer: TIM1 = unsafe { core::mem::transmute(()) };
        timer.cr1.write(|w| w.cen().clear_bit());
        timer.ccr1().write(|w| w.ccr().variant(0));
        ctx.local.ch0.set_duty(ctx.local.ch0.get_duty());
    }
}