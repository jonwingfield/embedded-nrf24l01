use command::{FlushRx, FlushTx, Nop};
use device::Device;
use registers::{
    AddressRegister, Dynpd, EnAa, EnRxaddr, Feature, Register, RfCh, RfSetup, SetupAw, SetupRetr,
    Status,
};
use PIPES_COUNT;

/// Supported air data rates.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum DataRate {
    R250Kbps,
    R1Mbps,
    R2Mbps,
}

impl Default for DataRate {
    fn default() -> DataRate {
        DataRate::R1Mbps
    }
}

/// Supported Power Levels
#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum PAControl {
    PAMax = 0b11,
    PAMinus6 = 0b10,
    PAMinus12 = 0b01,
    PAMin = 0b00,
}

impl Default for PAControl {
    fn default() -> PAControl {
        PAControl::PAMinus6
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CrcMode {
    OneByte,
    TwoBytes,
}

pub trait Configuration {
    type Inner: Device;
    fn device(&mut self) -> &mut Self::Inner;

    fn flush_rx(&mut self) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        self.device().send_command_reg(FlushRx::addr(), &[])?;
        Ok(())
    }

    fn flush_tx(&mut self) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        self.device().send_command_reg(FlushTx::addr(), &[])?;
        Ok(())
    }

    fn get_frequency(&mut self) -> Result<u8, <<Self as Configuration>::Inner as Device>::Error> {
        let (_, register) = self.device().read_register::<RfCh>()?;
        let freq_offset = register.rf_ch();
        Ok(freq_offset)
    }

    fn set_frequency(
        &mut self,
        freq_offset: u8,
    ) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        debug_assert!(freq_offset < 126);

        let mut register = RfCh(0);
        register.set_rf_ch(freq_offset);
        self.device().write_register(register)?;

        Ok(())
    }

    /// power: `0`: -18 dBm, `3`: 0 dBm
    fn set_rf(
        &mut self,
        rate: DataRate,
        power: PAControl,
    ) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        let mut register = RfSetup(0);
        register.set_rf_pwr(power as u8);

        let (dr_low, dr_high) = match rate {
            DataRate::R250Kbps => (true, false),
            DataRate::R1Mbps => (false, false),
            DataRate::R2Mbps => (false, true),
        };
        register.set_rf_dr_low(dr_low);
        register.set_rf_dr_high(dr_high);

        self.device().write_register(register)?;
        Ok(())
    }

    fn set_crc(
        &mut self,
        mode: Option<CrcMode>,
    ) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        self.device().update_config(|config| match mode {
            None => config.set_en_crc(false),
            Some(mode) => match mode {
                CrcMode::OneByte => config.set_crco(false),
                CrcMode::TwoBytes => config.set_crco(true),
            },
        })
    }

    fn set_pipes_rx_enable(
        &mut self,
        bools: &[bool; PIPES_COUNT],
    ) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        let reg = EnRxaddr::from_bools(bools.iter().map(|b| *b));
        self.device().write_register(reg)?;
        Ok(())
    }

    // TODO: Addresses don't work like this. There's a 'base' address and a 1-byte secondary
    // address. See datasheet, page 37 for a diagram and explanation of the Multiceiver arch
    fn set_rx_addr(
        &mut self,
        pipe_no: usize,
        addr: &[u8],
    ) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        match pipe_no {
            // TODO: Instead of panicing, save P0 and overwrite it when going into TX mode so Acks
            // still work
            0 => panic!(
                "Currently, using P0 for reading is not supported because it interferes with Acks"
            ),
            1 => self.device()
                .write_register_i(::registers::RxAddrP1::addr(), addr),
            2 => self.device()
                .write_register_i(::registers::RxAddrP2::addr(), &[addr[0]]),
            3 => self.device()
                .write_register_i(::registers::RxAddrP3::addr(), &[addr[0]]),
            4 => self.device()
                .write_register_i(::registers::RxAddrP4::addr(), &[addr[0]]),
            5 => self.device()
                .write_register_i(::registers::RxAddrP5::addr(), &[addr[0]]),
            _ => panic!("No such pipe"),
        }.map(|_status| ())
    }

    fn set_tx_addr(
        &mut self,
        addr: &[u8],
    ) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        self.device()
            .write_register_i(::registers::TxAddr::addr(), addr)?;
        // Important: Write P0 or we won't get acks
        self.device()
            .write_register_i(::registers::RxAddrP0::addr(), addr)?;
        Ok(())
    }

    fn set_auto_retransmit(
        &mut self,
        delay: u8,
        count: u8,
    ) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        let mut register = SetupRetr(0);
        register.set_ard(delay);
        register.set_arc(count);
        self.device().write_register(register)?;
        Ok(())
    }

    fn get_auto_ack(
        &mut self,
    ) -> Result<[bool; PIPES_COUNT], <<Self as Configuration>::Inner as Device>::Error> {
        // Read
        let (_, register) = self.device().read_register::<EnAa>()?;
        Ok(register.to_bools())
    }

    fn set_auto_ack(
        &mut self,
        bools: &[bool; PIPES_COUNT],
    ) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        // Convert back
        let register = EnAa::from_bools(bools.iter().map(|b| *b));
        // Write back
        self.device().write_register(register)?;
        Ok(())
    }

    fn get_address_width(
        &mut self,
    ) -> Result<u8, <<Self as Configuration>::Inner as Device>::Error> {
        let (_, register) = self.device().read_register::<SetupAw>()?;
        Ok(2 + register.aw())
    }

    fn get_interrupts(
        &mut self,
    ) -> Result<(bool, bool, bool), <<Self as Configuration>::Inner as Device>::Error> {
        let status = self.device().send_command_reg(Nop::addr(), &[])?;
        Ok((status.rx_dr(), status.tx_ds(), status.max_rt()))
    }

    fn clear_interrupts(
        &mut self,
    ) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        let mut clear = Status(0);
        clear.set_rx_dr(true);
        clear.set_tx_ds(true);
        clear.set_max_rt(true);
        self.device().write_register(clear)?;
        Ok(())
    }

    /// ## `bools`
    /// * `None`: Dynamic payload length
    /// * `Some(len)`: Static payload length `len`
    fn set_pipes_rx_lengths(
        &mut self,
        lengths: &[Option<u8>; PIPES_COUNT],
    ) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        // Enable dynamic payload lengths
        let bools = lengths.iter().map(|length| length.is_none());
        let dynpd = Dynpd::from_bools(bools);
        if dynpd.0 != 0 {
            self.device().update_register::<Feature, _, _>(|feature| {
                feature.set_en_dpl(true);
            })?;
        }
        self.device().write_register(dynpd)?;

        self.device()
            .write_register_i(::registers::RxPwP0::addr(), &[lengths[0].unwrap_or(0)])?;
        self.device()
            .write_register_i(::registers::RxPwP1::addr(), &[lengths[1].unwrap_or(0)])?;
        self.device()
            .write_register_i(::registers::RxPwP2::addr(), &[lengths[2].unwrap_or(0)])?;
        self.device()
            .write_register_i(::registers::RxPwP3::addr(), &[lengths[3].unwrap_or(0)])?;
        self.device()
            .write_register_i(::registers::RxPwP4::addr(), &[lengths[4].unwrap_or(0)])?;
        self.device()
            .write_register_i(::registers::RxPwP5::addr(), &[lengths[5].unwrap_or(0)])?;

        Ok(())
    }
}
