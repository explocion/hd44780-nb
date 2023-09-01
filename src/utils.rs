use core::fmt;

use crate::hal::blocking::delay::DelayUs;

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct State(pub(crate) u8);

impl State {
    #[inline]
    pub fn busy(&self) -> bool {
        (self.0 >> 7) > 0
    }

    #[inline]
    pub fn addr(&self) -> u8 {
        self.0 & 0x7F
    }
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "State {{ busy: {:?}, addr: {:?} }}",
            self.busy(),
            self.addr()
        )
    }
}

pub trait DelayMicros: DelayUs<u8> {}

impl<T: DelayUs<u8>> DelayMicros for T {}
