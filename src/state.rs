use device::Device;
use rx::RxMode;
use standby::StandbyMode;
use tx::TxMode;

pub enum State<D: Device> {
    Standby(StandbyMode<D>),
    Tx(TxMode<D>),
    Rx(RxMode<D>),
    None(D),
}

type ModeChangeResult<T, D> = Result<T, (State<D>, <D as Device>::Error)>;

pub struct StateHolder<D: Device> {
    state: Option<State<D>>,
}

impl<D: Device> StateHolder<D> {
    pub fn new(standby: StandbyMode<D>) -> Self {
        StateHolder {
            state: Some(State::Standby(standby)),
        }
    }

    pub fn with_tx<F, R>(&mut self, f: F) -> Result<R, D::Error>
    where
        F: Fn(&mut TxMode<D>) -> Result<R, D::Error>,
    {
        if let Some(state) = self.state.take() {
            match state.get_tx() {
                Ok(mut tx) => {
                    let result = f(&mut tx);
                    self.state = Some(State::Tx(tx));
                    result
                }
                Err((state, e)) => {
                    self.state = Some(state);
                    Err(e)
                }
            }
        } else {
            panic!("Missing state!");
        }
    }

    pub fn with_standby<F, R>(&mut self, f: F) -> Result<R, D::Error>
    where
        F: Fn(&mut StandbyMode<D>) -> Result<R, D::Error>,
    {
        if let Some(state) = self.state.take() {
            match state.get_standby() {
                Ok(mut standby) => {
                    let result = f(&mut standby);
                    self.state = Some(State::Standby(standby));
                    result
                }
                Err((state, e)) => {
                    self.state = Some(state);
                    Err(e)
                }
            }
        } else {
            panic!("Missing state!");
        }
    }

    pub fn with_rx<F, R>(&mut self, f: F) -> Result<R, D::Error>
    where
        F: Fn(&mut RxMode<D>) -> Result<R, D::Error>,
    {
        if let Some(state) = self.state.take() {
            match state.get_rx() {
                Ok(mut rx) => {
                    let result = f(&mut rx);
                    self.state = Some(State::Rx(rx));
                    result
                }
                Err((state, e)) => {
                    self.state = Some(state);
                    Err(e)
                }
            }
        } else {
            panic!("Missing state!");
        }
    }

    pub fn to_rx(&mut self) -> Result<(), D::Error> {
        if let Some(state) = self.state.take() {
            match state.get_rx() {
                Ok(mut rx) => {
                    self.state = Some(State::Rx(rx));
                    Ok(())
                }
                Err((state, e)) => {
                    self.state = Some(state);
                    Err(e)
                }
            }
        } else {
            panic!("Missing state!");
        }
    }
}

/// Easier switching of states
impl<D: Device> State<D> {
    pub fn get_standby(self) -> ModeChangeResult<StandbyMode<D>, D> {
        match self {
            State::None(d) => StandbyMode::power_up(d).map_err(|(d, e)| (State::None(d), e)),
            State::Tx(tx) => tx.standby().map_err(|(tx, e)| (State::Tx(tx), e)),
            State::Rx(rx) => Ok(rx.standby()),
            State::Standby(standby) => Ok(standby),
        }
    }

    pub fn get_rx(self) -> ModeChangeResult<RxMode<D>, D> {
        match self {
            State::None(_) => Self::get_rx(State::Standby(Self::get_standby(self)?)),
            State::Tx(_) => Self::get_rx(State::Standby(Self::get_standby(self)?)),
            State::Rx(rx) => Ok(rx),
            State::Standby(standby) => standby.rx().map_err(|(d, e)| (State::None(d), e)),
        }
    }

    pub fn get_tx(self) -> ModeChangeResult<TxMode<D>, D> {
        match self {
            State::None(_) => Self::get_tx(State::Standby(Self::get_standby(self)?)),
            State::Tx(tx) => Ok(tx),
            State::Rx(_) => Self::get_tx(State::Standby(Self::get_standby(self)?)),
            State::Standby(standby) => standby.tx().map_err(|(d, e)| (State::None(d), e)),
        }
    }
}
