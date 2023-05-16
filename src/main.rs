// #![deny(unsafe_code)]
#![no_std]
#![no_main]

use core::fmt::Write;
use core::ops::Mul;
use defmt_rtt as _;
use panic_probe as _;

use cortex_m_rt::entry;
use stm32f1xx_hal::{pac, prelude::*};

use embedded_hal::spi::{Mode, Phase, Polarity};
use embedded_hal::blocking::spi::{Transfer as SpiTransfer, Write as SpiWrite};
use stm32f1xx_hal::spi::Spi;
use stm32f1xx_hal::adc::{Adc, SampleTime};
use mfrc522::{Initialized, Mfrc522, WithNssDelay};

pub const MODE: Mode = Mode {
    polarity: Polarity::IdleLow,
    phase: Phase::CaptureOnFirstTransition,
};

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
    let clocks = rcc.cfgr/*
        .sysclk(72.MHz())
        .hclk(72.MHz())
        .pclk1(36.MHz())
        .pclk2(72.MHz())
        */.freeze(&mut flash.acr);

    let mut afio = device.AFIO.constrain();

    let mut gpioa = device.GPIOA.split();

    let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);
    let spi = Spi::spi1(
        device.SPI1,
        (sck, miso, mosi),
        &mut afio.mapr,
        MODE,
        1024.kHz(),
        clocks,
    );

    let nss = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);
    let mut mfrc522 = Mfrc522::new(spi).with_nss(nss).init().unwrap();

    loop {
        if let Ok(atqa) = mfrc522.new_card_present() {
            if let Ok(uid) = mfrc522.select(&atqa) {
                defmt::info!("* {:?}", uid.as_bytes());

                handle_authenticate(&mut mfrc522, &uid, |m| {
                    match m.mf_read(1) {
                        Ok(data) => {
                            defmt::println!("read {:?}", data);

                            let buffer = [
                               0x0F, 0x0E, 0x0D, 0x0C,
                               0x0B, 0x0A, 0x09, 0x08,
                               0x07, 0x06, 0x05, 0x04,
                               0x03, 0x02, 0x01, 0x00,
                            ];
                            /*match m.mf_write(1, buffer) {
                               Ok(_) => {
                                   defmt::println!("write success");
                               }
                               Err(_) => {
                                   defmt::println!("error during write");
                               }
                            }*/
                        }
                        Err(_) => {
                            defmt::println!("error during read");
                        }
                    }
                });
            }
        }
    }
}



fn handle_authenticate<E, SPI, NSS, D, F>(
    mfrc522: &mut Mfrc522<SPI, NSS, D, Initialized>,
    uid: &mfrc522::Uid,
    action: F,
) where
    SPI: SpiTransfer<u8, Error = E> + SpiWrite<u8, Error = E>,
    Mfrc522<SPI, NSS, D, Initialized>: WithNssDelay,
    F: FnOnce(&mut Mfrc522<SPI, NSS, D, Initialized>) -> (),
{
    let key = [0xFF; 6];
    if mfrc522.mf_authenticate(uid, 1, &key).is_ok() {
        action(mfrc522);
    } else {
        defmt::println!("Could not authenticate");
    }

    if mfrc522.hlta().is_err() {
        defmt::println!("Could not halt");
    }
    if mfrc522.stop_crypto1().is_err() {
        defmt::println!("Could not disable crypto1");
    }
}