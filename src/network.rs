use core;
use payload::Payload;
use transciever::Transceiver;

const MAX_ROUTES: usize = 8;

type Address = u8;

pub struct Network {
    this_address: Address,
    routes: [Option<RoutingTableEntry>; MAX_ROUTES],
}

type Result<S, T> = core::result::Result<S, RoutingError<T>>;

#[derive(Debug)]
pub enum RoutingError<T: Transceiver> {
    NoRouteToHost,
    TransceiverError(T::Error),
}

#[derive(Debug, Copy, Clone)]
struct RoutingTableEntry {
    dest: Address,
    next_hop: Address,
}

#[derive(Debug, Copy, Clone)]
struct Message<'a> {
    source: Address,
    dest: Address,
    hops: u8,
    buf: &'a [u8],
}

impl<'a> Message<'a> {
    fn new(source: Address, dest: Address, buf: &[u8]) -> Message {
        Message {
            source,
            dest,
            hops: 0,
            // TODO: truncate to 28 or panic
            buf,
        }
    }

    fn from_receive(data: &[u8]) -> Message {
        let len = data[3] as usize;
        Message {
            source: data[0],
            dest: data[1],
            // TODO: check hops
            hops: data[2] + 1,
            buf: &data[3..3 + len],
        }
    }

    fn get_bytes(&self) -> [u8; 24] {
        let mut data = [0u8; 24];
        data[0] = self.source;
        data[1] = self.dest;
        data[2] = self.hops;
        data[3..3 + self.buf.len()].copy_from_slice(&self.buf);
        data
    }
}

/// Very simple static Network for RF24. Eventually to be broken out with Transceiver into its own
/// crate
impl Network {
    pub fn new(this_address: Address) -> Network {
        Network {
            this_address,
            routes: [None; MAX_ROUTES],
        }
    }

    pub fn add_route_to(&mut self, dest: Address, next_hop: Address) {
        for entry in self.routes.iter_mut() {
            // TODO: use heapless IndexMap and/or check bounds
            if let None = entry {
                *entry = Some(RoutingTableEntry { dest, next_hop });
                break;
            }
        }
    }

    fn get_route_to(&self, dest: Address) -> Option<RoutingTableEntry> {
        self.routes
            .iter()
            .filter_map(|&route| route)
            .find(|&route| route.dest == dest)
    }

    pub fn send_to_wait<T: Transceiver>(
        &self,
        buf: &[u8],
        dest: Address,
        transciever: &mut T,
    ) -> Result<bool, T> {
        let message = Message::new(self.this_address, dest, buf);

        self.route(&message, transciever)
    }

    fn route<T: Transceiver>(&self, message: &Message, transciever: &mut T) -> Result<bool, T> {
        if let Some(route) = self.get_route_to(message.dest) {
            transciever
                .send_to(route.next_hop, &message.get_bytes())
                .map_err(|e| RoutingError::TransceiverError(e))
        } else {
            Err(RoutingError::NoRouteToHost)
        }
    }

    pub fn receive<T: Transceiver>(&self, transciever: &mut T) -> Result<Option<Payload>, T> {
        let result = transciever
            .receive()
            .map_err(|e| RoutingError::TransceiverError(e))?;

        if let Some(data) = result {
            let message = Message::from_receive(&data);
            if message.dest == self.this_address {
                return Ok(Some(Payload::new(&message.buf)));
            } else {
                return self.route(&message, transciever).map(|_result| None); // TODO: indicate failure here
            }
        } else {
            Ok(None)
        }
    }
}
