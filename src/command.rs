pub use payload::Payload;

pub trait Command<'a> {
    fn buf(&'a mut self) -> &'a mut [u8];

    type Response;
    fn decode_response(self) -> Self::Response;
}

pub struct ReadRxPayload {
    data: [u8; 33],
    payload_width: usize,
}

impl ReadRxPayload {
    pub fn new(payload_width: usize) -> Self {
        let mut data = [0u8; 33];
        data[0] = Self::addr();
        ReadRxPayload {
            data,
            payload_width,
        }
    }

    pub fn addr() -> u8 {
        0b0110_0001
    }
}

impl<'a> Command<'a> for ReadRxPayload {
    fn buf(&'a mut self) -> &'a mut [u8] {
        &mut self.data
    }

    type Response = Payload;
    fn decode_response(self) -> Self::Response {
        Payload::new(&self.data[1..self.payload_width + 1])
    }
}

pub struct WriteTxPayload {}

impl WriteTxPayload {
    pub fn addr() -> u8 {
        0b1010_0000
    }
}

pub struct ReadRxPayloadWidth {
    data: [u8; 2],
}

impl ReadRxPayloadWidth {
    pub fn new() -> ReadRxPayloadWidth {
        let mut data = [0u8; 2];
        data[0] = Self::addr();
        ReadRxPayloadWidth { data }
    }

    pub fn addr() -> u8 {
        0b0110_0000
    }
}

impl<'a> Command<'a> for ReadRxPayloadWidth {
    fn buf(&'a mut self) -> &'a mut [u8] {
        &mut self.data
    }

    type Response = u8;
    fn decode_response(self) -> Self::Response {
        self.data[1]
    }
}

pub struct FlushRx {}

impl FlushRx {
    pub fn addr() -> u8 {
        0b1110_0010
    }
}

pub struct FlushTx {}

impl FlushTx {
    pub fn addr() -> u8 {
        0b1110_0001
    }
}

pub struct Nop {}

impl Nop {
    pub fn addr() -> u8 {
        0b1111_1111
    }
}
