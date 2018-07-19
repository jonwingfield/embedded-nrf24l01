use config::Configuration;
#[cfg(not(feature = "tiny"))]
use core::fmt;
use device::Device;
use rx::RxMode;
use tx::TxMode;

/// Represents **Standby-I** mode
///
/// This represents the state the device is in inbetween TX or RX
/// mode.
pub struct StandbyMode<D: Device> {
    device: D,
}

#[cfg(not(feature = "tiny"))]
impl<D: Device> fmt::Debug for StandbyMode<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StandbyMode")
    }
}

impl<D: Device> StandbyMode<D> {
    pub fn power_up(mut device: D) -> Result<Self, (D, D::Error)> {
        // TODO: wait 130us for it to settle
        match device.update_config(|config| config.set_pwr_up(true)) {
            Ok(()) => Ok(StandbyMode { device }),
            Err(e) => Err((device, e)),
        }
    }

    pub(crate) fn from_rx_tx(mut device: D) -> Self {
        device.ce_disable();
        StandbyMode { device }
    }

    /// Go into RX mode
    pub fn rx(self) -> Result<RxMode<D>, (D, D::Error)> {
        let mut device = self.device;

        // TODO: wait 130us for it to settle
        match device.update_config(|config| config.set_prim_rx(true)) {
            Ok(()) => {
                device.ce_enable();
                Ok(RxMode::new(device))
            }
            Err(e) => Err((device, e)),
        }
    }

    /// Go into TX mode
    pub fn tx(self) -> Result<TxMode<D>, (D, D::Error)> {
        let mut device = self.device;

        match device.update_config(|config| config.set_prim_rx(false)) {
            Ok(()) => {
                // We might want to ce_enable here. Doing that now will result in a lower delay in
                // actually sending. It also *actually* puts us into TX mode. Otherwise we're still
                // in standby. The drawback is the max TX-mode time of 4ms. Staying in this mode
                // for extended periods should be avoided.
                // TODO: wait 130us for it to settle
                Ok(TxMode::new(device))
            }
            Err(e) => Err((device, e)),
        }
    }
}

impl<D: Device> Configuration for StandbyMode<D> {
    type Inner = D;
    fn device(&mut self) -> &mut Self::Inner {
        &mut self.device
    }
}
