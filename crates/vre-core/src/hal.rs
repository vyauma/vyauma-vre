//! Hardware Abstraction Layer for Embedded Targets

use std::fmt::Debug;

pub trait GpioPin: Send + Sync + Debug {
    fn set_high(&mut self) -> Result<(), String>;
    fn set_low(&mut self) -> Result<(), String>;
    fn is_high(&self) -> Result<bool, String>;
    fn is_low(&self) -> Result<bool, String>;
}

pub trait I2cBus: Send + Sync + Debug {
    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), String>;
    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), String>;
}

pub trait SpiBus: Send + Sync + Debug {
    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), String>;
}

pub trait UartPort: Send + Sync + Debug {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, String>;
    fn write(&mut self, bytes: &[u8]) -> Result<usize, String>;
}

pub trait HardwareAbstractionLayer: Send + Sync {
    fn claim_gpio(&self, pin: u8) -> Result<Box<dyn GpioPin>, String>;
    fn claim_i2c(&self, bus: u8) -> Result<Box<dyn I2cBus>, String>;
    fn claim_spi(&self, bus: u8) -> Result<Box<dyn SpiBus>, String>;
    fn claim_uart(&self, port: u8) -> Result<Box<dyn UartPort>, String>;
}

pub struct MockHal;

impl HardwareAbstractionLayer for MockHal {
    fn claim_gpio(&self, _pin: u8) -> Result<Box<dyn GpioPin>, String> {
        Err("MockHal: GPIO not implemented".into())
    }
    fn claim_i2c(&self, _bus: u8) -> Result<Box<dyn I2cBus>, String> {
        Err("MockHal: I2C not implemented".into())
    }
    fn claim_spi(&self, _bus: u8) -> Result<Box<dyn SpiBus>, String> {
        Err("MockHal: SPI not implemented".into())
    }
    fn claim_uart(&self, _port: u8) -> Result<Box<dyn UartPort>, String> {
        Err("MockHal: UART not implemented".into())
    }
}
