#![no_std]
#![no_main]

use panic_probe as _;
use defmt_rtt as _;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use embedded_graphics::{geometry::Point, image::Image, pixelcolor::Rgb565, prelude::*};
use panic_semihosting as _;
use ssd1331::{DisplayRotation, Ssd1331};
use stm32f1xx_hal::{
    prelude::*,
    spi::{Mode, Phase, Polarity, Spi},
    stm32,
};
use tinybmp::Bmp;

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);

    // SPI1
    let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);
    let mut delay = cp.SYST.delay(&clocks);
    let mut rst = gpiob.pb0.into_push_pull_output(&mut gpiob.crl);
    let dc = gpiob.pb1.into_push_pull_output(&mut gpiob.crl);

    let spi = Spi::spi1(
        dp.SPI1,
        (sck, miso, mosi),
        &mut afio.mapr,
        Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        },
        8.mhz(),
        clocks
    );

    let mut disp = Ssd1331::new(spi, dc, DisplayRotation::Rotate0);

    disp.reset(&mut rst, &mut delay).unwrap();
    disp.init().unwrap();
    disp.flush().unwrap();

    let (w, h) = disp.dimensions();

    let bmp =
        Bmp::from_slice(include_bytes!("./rust-pride.bmp")).expect("Failed to load BMP image");

    let im: Image<Bmp<Rgb565>> = Image::new(&bmp, Point::zero());

    // Position image in the center of the display
    let moved = im.translate(Point::new(
        (w as u32 - bmp.size().width) as i32 / 2,
        (h as u32 - bmp.size().height) as i32 / 2,
    ));

    moved.draw(&mut disp).unwrap();

    disp.flush().unwrap();

    loop {}
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("{:#?}", ef);
}

/// Size information for the common 96x16 variants
#[derive(Debug, Copy, Clone)]
pub struct DisplaySize96x64;
impl DisplaySize for DisplaySize96x64 {
    const WIDTH: u8 = 96;
    const HEIGHT: u8 = 64;
    type Buffer = [u8; Self::WIDTH as usize * Self::HEIGHT as usize / 8];

    fn configure(&self, iface: &mut impl WriteOnlyDataCommand) -> Result<(), DisplayError> {
        Command::ComPinConfig(false, false).send(iface)
    }
}