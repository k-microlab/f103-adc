//register selection
//RS2 RS1 RS0
#[repr(u8)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Reg {
    CMM    = 0x0, //communication register 8 bit
    SETUP  = 0x1, //setup register 8 bit
    CLOCK  = 0x2, //clock register 8 bit
    DATA   = 0x3, //data register 16 bit, contains conversion result
    TEST   = 0x4, //test register 8 bit, POR 0x0
    NOP    = 0x5, //no operation
    OFFSET = 0x6, //offset register 24 bit
    GAIN   = 0x7, //gain register 24 bit
}

//channel selection for AD7706 (for AD7705 use the first two channel definitions)
//CH1 CH0
#[repr(u8)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Channel {
    AIN1 = 0x0, //AIN1; calibration register pair 0
    AIN2 = 0x1, //AIN2; calibration register pair 1
    COMM = 0x2, //common; calibration register pair 0
    AIN3 = 0x3, //AIN3; calibration register pair 2
}

//output update rate
//CLK FS1 FS0
#[repr(u8)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum UpdateRate {
    H20  = 0x0, // 20 Hz
    H25  = 0x1, // 25 Hz
    H100 = 0x2, // 100 Hz
    H200 = 0x3, // 200 Hz
    H50  = 0x4, // 50 Hz
    H60  = 0x5, // 60 Hz
    H250 = 0x6, // 250 Hz
    H500 = 0x7, // 500 Hz
}

//operating mode options
//MD1 MD0
#[repr(u8)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum OperatingMode {
    Normal               = 0x0, //normal mode
    SelfCalibration      = 0x1, //self-calibration
    ZeroScaleCalibration = 0x2, //zero-scale system calibration, POR 0x1F4000, set FSYNC high before calibration, FSYNC low after calibration
    FullScaleCalibration = 0x3, //full-scale system calibration, POR 0x5761AB, set FSYNC high before calibration, FSYNC low after calibration
}

//gain setting
#[repr(u8)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Gain {
    G1   = 0x0,
    G2   = 0x1,
    G4   = 0x2,
    G8   = 0x3,
    G16  = 0x4,
    G32  = 0x5,
    G64  = 0x6,
    G128 = 0x7,
}

#[repr(u8)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Polarity {
    Unipolar = 0x0,
    Bipolar  = 0x1,
}

#[repr(u8)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum ClockDivider {
    DIV1 = 0x1,
    DIV2 = 0x2,
}

pub struct ChannelConfig {
    clock_divider: ClockDivider,
    polarity: Polarity,
    gain: Gain,
    update_rate: UpdateRate
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            clock_divider: ClockDivider::DIV1,
            polarity: Polarity::Bipolar,
            gain: Gain::G1,
            update_rate: UpdateRate::H25,
        }
    }
}

pub struct AD770x<SPI, CS>
    where
        SPI: embedded_hal::blocking::spi::Transfer<u8> + embedded_hal::blocking::spi::Write<u8>,
        CS: embedded_hal::digital::v2::OutputPin,
        <CS as embedded_hal::digital::v2::OutputPin>::Error: core::fmt::Debug,
        <SPI as embedded_hal::blocking::spi::Transfer<u8>>::Error: core::fmt::Debug,
        <SPI as embedded_hal::blocking::spi::Write<u8>>::Error: core::fmt::Debug
{
    spi: SPI,
    cs: CS
}

impl<SPI, CS> AD770x<SPI, CS>
    where
        SPI: embedded_hal::blocking::spi::Transfer<u8> + embedded_hal::blocking::spi::Write<u8>,
        CS: embedded_hal::digital::v2::OutputPin,
        <CS as embedded_hal::digital::v2::OutputPin>::Error: core::fmt::Debug,
        <SPI as embedded_hal::blocking::spi::Transfer<u8>>::Error: core::fmt::Debug,
        <SPI as embedded_hal::blocking::spi::Write<u8>>::Error: core::fmt::Debug
{
    pub fn new(spi: SPI, mut cs: CS) -> Self {
        cs.set_high().unwrap();
        Self {
            spi,
            cs
        }
    }

    #[inline]
    fn transfer_bytes(&mut self, bytes: &mut [u8]) {
        self.cs.set_low().unwrap();
        self.spi.transfer(bytes).unwrap();
        self.cs.set_high().unwrap();
    }

    //write communication register
    //   7        6      5      4      3      2      1      0
    //0/DRDY(0) RS2(0) RS1(0) RS0(0) R/W(0) STBY(0) CH1(0) CH0(0)
    pub fn set_next_operation(&mut self, reg: Reg, channel: Channel, read_write: bool) {
        let mut r = (reg as u8) << 4 | (read_write as u8) << 3 | (channel as u8);
        self.transfer_bytes(core::slice::from_mut(&mut r));
    }

    //Clock Register
    //   7      6       5        4        3        2      1      0
    //ZERO(0) ZERO(0) ZERO(0) CLKDIS(0) CLKDIV(0) CLK(1) FS1(0) FS0(1)
    //
    //CLKDIS: master clock disable bit
    //CLKDIV: clock divider bit
    pub fn write_clock_register(&mut self, clkdis: u8, clkdiv: ClockDivider, out_update_rate: UpdateRate) {
        let mut r = clkdis << 4 | (clkdiv as u8) << 3 | (out_update_rate as u8);
        r &= !(1 << 2);
        self.transfer_bytes(core::slice::from_mut(&mut r));
    }

    //Setup Register
    //   7     6     5     4     3      2      1      0
    //MD1(0) MD0(0) G2(0) G1(0) G0(0) B/U(0) BUF(0) FSYNC(1)
    pub fn write_setup_register(&mut self, mode: OperatingMode, gain: Gain, polarity: Polarity, buffered: bool, fsync: bool) {
        let mut r = (mode as u8) << 6 | (gain as u8) << 3 | (polarity as u8) << 2 | (buffered as u8) << 1 | (fsync as u8);
        self.transfer_bytes(core::slice::from_mut(&mut r));
    }

    fn read_raw(&mut self) -> u16 {
        let mut data = [0, 0];
        self.transfer_bytes(&mut data);
        u16::from_be_bytes(data)
    }

    pub fn read(&mut self, channel: Channel) -> u16 {
        while !self.data_ready(channel) {}
        self.set_next_operation(Reg::DATA, channel, true);
        self.read_raw()
    }

    pub fn read_voltage(&mut self, channel: Channel, vref: u16) -> u16 {
        ((self.read(channel) as u32 * vref as u32) / 65535) as u16
    }

    pub fn data_ready(&mut self, channel: Channel) -> bool {
        self.set_next_operation(Reg::CMM, channel, true);
        let mut r = 0;
        self.transfer_bytes(core::slice::from_mut(&mut r));
        (r & 0x80) == 0
    }

    pub fn reset(&mut self) {
        self.transfer_bytes(&mut [0xff; 100]);
    }

    pub fn init(&mut self, channel: Channel, config: ChannelConfig) {
        self.set_next_operation(Reg::CLOCK, channel, false);
        self.write_clock_register(0, config.clock_divider, config.update_rate);

        self.set_next_operation(Reg::SETUP, channel, false);
        self.write_setup_register(OperatingMode::SelfCalibration, config.gain, config.polarity, false, false);

        while !self.data_ready(channel) {}
    }
}