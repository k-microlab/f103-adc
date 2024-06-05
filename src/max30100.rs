use embedded_hal::blocking::i2c::{Read, Write};
use stm32f1xx_hal::i2c;
use stm32f1xx_hal::i2c::Instance;

const DEVICE: u8 = 0x57;

mod register {
    //Part ID
    pub const REV_ID: u8 = 0xFE;
    pub const PART_ID: u8 = 0xFF;

    //Status registers
    pub const INT_STATUS: u8 = 0x00;
    pub const INT_ENABLE: u8 = 0x01;

    //FIFO
    pub const FIFO_WRITE: u8 = 0x02;
    pub const FIFO_OVERFLOW_COUNTER: u8 = 0x03;
    pub const FIFO_READ: u8 = 0x04;
    pub const FIFO_DATA: u8 = 0x05;

    //Config
    pub const MODE_CONF: u8 = 0x06;
    pub const SPO2_CONF: u8 = 0x07;
    pub const LED_CONF: u8 = 0x09;

    //Temperature
    pub const TEMP_INT: u8 = 0x16;
    pub const TEMP_FRACTION: u8 = 0x17;
}

const MODE_SHDN: u8 = 1 << 7;
const MODE_RESET: u8 = 1 << 6;
const MODE_TEMP_EN: u8 = 1 << 3;

#[repr(u8)]
enum OperatingMode {
    HROnly = 0x02,
    SPO2HR = 0x03
}

const SPO2_HI_RES_EN: u8 = 1 << 6;

#[repr(u8)]
enum SamplingRate {
    R50Hz   = 0x00,
    R100Hz  = 0x01,
    R167Hz  = 0x02,
    R200Hz  = 0x03,
    R400Hz  = 0x04,
    R600Hz  = 0x05,
    R800Hz  = 0x06,
    R1000Hz = 0x07,
}

#[repr(u8)]
enum PulseWidth {
    W200UsADC13  = 0x00,
    W400UsADC14  = 0x01,
    W800UsADC15  = 0x02,
    W1600UsADC16 = 0x03,
}

#[repr(u8)]
enum LEDCurrent {
    I0MA     = 0x00,
    I4_4MA   = 0x01,
    I7_6MA   = 0x02,
    I11MA    = 0x03,
    I14_2MA  = 0x04,
    I17_4MA  = 0x05,
    I120_8MA = 0x06,
    I124MA   = 0x07,
    I127_1MA = 0x08,
    I130_6MA = 0x09,
    I133_8MA = 0x0A,
    I137MA   = 0x0B,
    I140_2MA = 0x0C,
    I143_6MA = 0x0D,
    I146_8MA = 0x0E,
    I50MA   = 0x0F,
}

pub struct Config {
    pub mode: OperatingMode,
    pub sampling_rate: SamplingRate,
    pub pulse_width: PulseWidth,
    pub red_current: LEDCurrent,
    pub infrared_current: LEDCurrent,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: OperatingMode::SPO2HR,
            sampling_rate: SamplingRate::R100Hz,
            pulse_width: PulseWidth::W1600UsADC16,
            red_current: LEDCurrent::I127_1MA,
            infrared_current: LEDCurrent::I50MA,
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct FIFO {
    pub infrared: u16,
    pub red: u16
}

pub struct Max30100<I2C> {
    i2c: I2C,
    address: u8
}

impl<I2C> Max30100<I2C> where I2C: Read<Error=i2c::Error> + Write<Error=i2c::Error> {
    pub fn new(i2c: I2C, config: Config) -> Result<Self, i2c::Error> {
        let mut this = Self {
            i2c, address: DEVICE
        };
        this.set_mode(config.mode)?;
        this.set_sampling_rate(config.sampling_rate)?;
        this.set_led_pulse_width(config.pulse_width)?;
        this.set_led_current(config.red_current, config.infrared_current)?;

        Ok(this)
    }

    fn write_reg(&mut self, register: u8, value: u8) -> Result<(), i2c::Error> {
        self.i2c.write(self.address, &[
            register,
            value
        ])
    }

    fn read_reg(&mut self, register: u8) -> Result<u8, i2c::Error> {
        self.i2c.write(self.address, &[register])?;
        let mut value = 0u8;
        self.i2c.read(self.address, core::slice::from_mut(&mut value))?;
        Ok(value)
    }

    pub fn get_revision_id(&mut self) -> Result<u8, i2c::Error> {
        self.read_reg(register::REV_ID)
    }

    pub fn get_part_id(&mut self) -> Result<u8, i2c::Error> {
        self.read_reg(register::PART_ID)
    }

    pub fn set_mode(&mut self, mode: OperatingMode) -> Result<(), i2c::Error> {
        let mut current = self.read_reg(register::MODE_CONF)?;
        self.write_reg(register::MODE_CONF, (current & 0xF8) | mode as u8)
    }

    pub fn set_high_resolution(&mut self, high_resolution: bool) -> Result<(), i2c::Error> {
        let conf = self.read_reg(register::SPO2_CONF)?;
        if high_resolution {
            self.write_reg(register::SPO2_CONF, conf | SPO2_HI_RES_EN)?;
        } else {
            self.write_reg(register::SPO2_CONF, conf & !SPO2_HI_RES_EN)?;
        }
        Ok(())
    }

    pub fn set_sampling_rate(&mut self, sampling_rate: SamplingRate) -> Result<(), i2c::Error> {
        let conf = self.read_reg(register::SPO2_CONF)?;
        self.write_reg(register::SPO2_CONF, (conf & 0xE3) | ((sampling_rate as u8) << 2))
    }

    pub fn set_led_pulse_width(&mut self, width: PulseWidth) -> Result<(), i2c::Error> {
        let conf = self.read_reg(register::SPO2_CONF)?;
        self.write_reg(register::SPO2_CONF, (conf & 0xFC) | width as u8)
    }

    pub fn set_led_current(&mut self, red: LEDCurrent, ir: LEDCurrent) -> Result<(), i2c::Error> {
        self.write_reg(register::LED_CONF, ((red as u8) << 4) | ir as u8)
    }

    pub fn read_temperature(&mut self) -> Result<(), i2c::Error> {
        let conf = self.read_reg(register::MODE_CONF)?;
        self.write_reg(register::MODE_CONF, conf | MODE_TEMP_EN)
    }

    pub fn get_temperature(&mut self) -> Result<f32, i2c::Error> {
        let int: i8 = self.read_reg(register::TEMP_INT)? as i8;
        let frac = self.read_reg(register::TEMP_FRACTION)? as f32 * 0.0625;
        Ok(int as f32 + frac)
    }

    pub fn read_fifo(&mut self) -> Result<FIFO, i2c::Error> {
        let mut fifo = [0u16, 0];
        self.i2c.write(self.address, &[register::FIFO_DATA])?;
        self.i2c.read(self.address, bytemuck::bytes_of_mut(&mut fifo))?;
        Ok(FIFO {
            infrared: u16::from_be(fifo[0]),
            red: u16::from_be(fifo[1])
        })
    }
}