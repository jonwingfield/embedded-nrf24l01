// Copyright 2018, Astro <astro@spaceboyz.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE>. This file
// may not be copied, modified, or distributed except according to
// those terms.
#![no_std]
extern crate embedded_hal;
#[macro_use]
extern crate bitfield;

#[cfg(not(feature = "tiny"))]
use core::fmt;
use core::fmt::Debug;
use embedded_hal::blocking::spi::Transfer as SpiTransfer;
use embedded_hal::digital::OutputPin;

mod config;
pub use config::{Configuration, CrcMode, DataRate, PAControl};
pub mod setup;

mod registers;
use registers::{Config, Register, SetupAw, Status};
mod command;
use command::Command;
mod payload;
pub use payload::Payload;
mod error;
pub use error::Error;

mod device;
pub use device::Device;
mod standby;
pub use standby::StandbyMode;
mod rx;
pub use rx::RxMode;
mod tx;
pub use tx::TxMode;

mod network;

pub const PIPES_COUNT: usize = 6;
pub const MIN_ADDR_BYTES: usize = 3;
pub const MAX_ADDR_BYTES: usize = 5;

/// Driver for the nRF24L01+
pub struct NRF24L01<CE: OutputPin, CSN: OutputPin, SPI: SpiTransfer<u8>> {
    ce: CE,
    csn: CSN,
    spi: SPI,
    config: Config,
}

#[cfg(not(feature = "tiny"))]
impl<CE: OutputPin, CSN: OutputPin, SPI: SpiTransfer<u8, Error = SPIE>, SPIE: Debug> fmt::Debug
    for NRF24L01<CE, CSN, SPI>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NRF24L01")
    }
}

impl<CE: OutputPin, CSN: OutputPin, SPI: SpiTransfer<u8, Error = SPIE>, SPIE: Debug>
    NRF24L01<CE, CSN, SPI>
{
    /// Construct a new driver instance.
    pub fn new(mut ce: CE, mut csn: CSN, spi: SPI) -> Result<StandbyMode<Self>, Error<SPIE>> {
        ce.set_low();
        csn.set_high();

        // Reset value
        // TODO: should we be reading config from the device?
        let mut config = Config(0b0000_1000);
        config.set_mask_rx_dr(true);
        config.set_mask_tx_ds(true);
        config.set_mask_max_rt(true);
        let mut device = NRF24L01 {
            ce,
            csn,
            spi,
            config,
        };
        if !device.is_connected()? {
            panic!("No device");
        }
        // debug_assert!(device.is_connected().unwrap());

        // TODO: activate features?

        StandbyMode::power_up(device).map_err(|(_, e)| e)
    }

    pub fn is_connected(&mut self) -> Result<bool, Error<SPIE>> {
        let (_, setup_aw) = self.read_register::<SetupAw>()?;
        let valid = setup_aw.aw() >= 3 && setup_aw.aw() <= 5;
        Ok(valid)
    }
}

impl<CE: OutputPin, CSN: OutputPin, SPI: SpiTransfer<u8, Error = SPIE>, SPIE: Debug> Device
    for NRF24L01<CE, CSN, SPI>
{
    type Error = Error<SPIE>;

    fn ce_enable(&mut self) {
        self.ce.set_high();
    }

    fn ce_disable(&mut self) {
        self.ce.set_low();
    }

    fn send_command<'a, C: Command<'a>>(
        &mut self,
        command: &'a mut C,
    ) -> Result<Status, Self::Error> {
        let mut buf = command.buf();
        // SPI transaction
        self.csn.set_low();
        let transfer_result = self.spi.transfer(&mut buf).map(|_| {});
        self.csn.set_high();
        // Propagate Err only after csn.set_high():
        transfer_result?;

        // Parse response
        let status = Status(buf[0]);

        Ok(status)
    }

    fn send_command_reg(&mut self, addr: u8, data: &[u8]) -> Result<Status, Self::Error> {
        // Allocate storage
        let mut buf = [0; 33];
        buf[0] = addr;
        let len = data.len();
        buf[1..len + 1].copy_from_slice(&data);
        if len > 0 && buf[1] != data[0] {
            panic!("");
        }

        // SPI transaction
        self.csn.set_low();
        let transfer_result = self.spi.transfer(&mut buf[0..len + 1]).map(|_| {});
        self.csn.set_high();
        // Propagate Err only after csn.set_high():
        transfer_result?;
        // Parse response
        let status = Status(buf[0]);

        Ok(status)
    }

    fn write_register<R: Register>(&mut self, reg: R) -> Result<Status, Self::Error> {
        self.send_command_reg(0b10_0000 | R::addr(), &[reg.data()])
    }

    fn write_register_i(&mut self, addr: u8, data: &[u8]) -> Result<Status, Self::Error> {
        self.send_command_reg(0b10_0000 | addr, data)
    }

    fn read_register<R: Register>(&mut self) -> Result<(Status, R), Self::Error> {
        self.read_register_internal(R::addr())
            .map(|(status, reg)| (status, R::decode(&[reg])))
    }

    fn read_register_internal(&mut self, addr: u8) -> Result<(Status, u8), Self::Error> {
        // Allocate storage
        let mut buf = [addr, 0u8];

        // SPI transaction
        self.csn.set_low();
        let transfer_result = self.spi.transfer(&mut buf).map(|_| {});
        self.csn.set_high();
        // Propagate Err only after csn.set_high():
        transfer_result?;

        // Parse response
        let status = Status(buf[0]);

        Ok((status, buf[1]))
    }

    fn update_config<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(&mut Config) -> R,
    {
        // Mutate
        let old_config = self.config.clone();
        let result = f(&mut self.config);

        if self.config != old_config {
            let config = self.config.clone();
            self.write_register(config)?;
        }
        Ok(result)
    }
}
