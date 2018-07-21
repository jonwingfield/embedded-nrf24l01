use config::Configuration;
use device::Device;
use payload::Payload;
use state::StateHolder;

pub trait Transceiver {
    type Error;

    fn send_to(&mut self, dest: u8, buf: &[u8]) -> Result<bool, Self::Error>;
    fn receive(&mut self) -> Result<Option<Payload>, Self::Error>;
}

impl<D: Device> Transceiver for StateHolder<D> {
    type Error = D::Error;

    fn send_to(&mut self, dest: u8, buf: &[u8]) -> Result<bool, Self::Error> {
        // set the destination address to netw + dest
        self.with_standby(|ref mut standby| standby.set_tx_addr(&[b'n', b'e', b't', b'w', dest]))?;
        // send the message
        let result = self.with_tx(|ref mut tx| Ok(tx.send_sync(buf).unwrap_or(false)))?;
        // go back to rx mode
        self.to_rx()?;
        Ok(result)
    }

    fn receive(&mut self) -> Result<Option<Payload>, Self::Error> {
        self.with_rx(|ref mut rx| {
            if !rx.is_empty()? {
                rx.read().map(|payload| Some(payload))
            } else {
                Ok(None)
            }
        })
    }
}
