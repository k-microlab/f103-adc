// #![deny(unsafe_code)]
#![no_std]
#![no_main]

extern crate stm32f1xx_hal as hal;

use panic_abort as _;

use hal::prelude::*;
use hal::spi::Spi;
use embedded_sdmmc::{SdCard, TimeSource, Timestamp, VolumeIdx, VolumeManager};
use embedded_hal::spi::{Mode, Phase, Polarity};
use embedded_sdmmc::sdcard::AcquireOpts;

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

    // defmt::println!("SPI init");

    let spi = Spi::spi1(
        device.SPI1,
        (sck, miso, mosi),
        &mut afio.mapr,
        MODE,
        400.kHz(),
        clocks,
    );
    let nss = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);

    // defmt::println!("Timer init");
    let delay = device.TIM1.delay_ms(&clocks);

    defmt::println!("SdCard init");
    let mut sd = SdCard::new_with_options(spi, nss, delay, AcquireOpts {
        use_crc: false,
    });

    let size = sd.num_bytes().unwrap();

    defmt::println!("Card size: {} GB", size / 1024 / 1024);

    let mut vm = VolumeManager::new(sd, Time);
    let mut volume = vm.get_volume(VolumeIdx(0)).unwrap();

    let root = vm.open_root_dir(&volume).unwrap();
    let mut buffer = [0u8; 256];

    vm.iterate_dir(&volume, &root, |entry, lfn| {
        let lfn = lfn.map(|chars| {
            let mut tmp = [0u8; 4];
            let mut len = 0;
            for c in chars {
                for b in c.encode_utf8(&mut tmp).as_bytes() {
                    buffer[len] = *b;
                    len += 1;
                }
            }
            unsafe { core::str::from_utf8_unchecked(&buffer[..len]) }
        });
        if entry.attributes.is_directory() {
            // defmt::println!("Dir: {:?}", lfn);
        } else {
            // defmt::println!("File: {:?}", lfn);
        }
        // defmt::println!("Entry: {}", core::str::from_utf8(entry.name.base_name()).unwrap());
    }).unwrap();

    loop {
        cortex_m::asm::wfi();
    }
}

pub struct Time;

impl TimeSource for Time {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}