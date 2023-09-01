#![no_std]

pub use bitvec;
pub use embedded_hal as hal;
pub use nb;
pub use ufmt;

pub mod instr;
pub mod utils;

use bitvec::prelude::*;
use hal::digital::v2::{OutputPin, PinState};
use ufmt::uWrite;

use crate::instr::*;
use crate::utils::DelayMicros;
use crate::utils::State;

pub trait DataBus: Sized {
    type Error;

    fn write_pins_now(
        &mut self,
        states: impl ExactSizeIterator<Item = PinState>,
    ) -> Result<(), Self::Error>;
    fn read_pins_now(&mut self) -> Result<[PinState; 4], Self::Error>;
}

pub struct LcdPins<RS: OutputPin, RW: OutputPin, E: OutputPin, DB: DataBus> {
    pub(crate) register_selection: RS,
    pub(crate) read_write: RW,
    enable: E,
    pub(crate) data_bus: DB,
}

pub enum LcdError<RS: OutputPin, RW: OutputPin, E: OutputPin, DB: DataBus> {
    RegisterSelectionError(RS::Error),
    ReadWriteError(RW::Error),
    EnableError(E::Error),
    DataBusError(DB::Error),
}

impl<RS: OutputPin, RW: OutputPin, E: OutputPin, DB: DataBus> LcdPins<RS, RW, E, DB> {
    #[inline]
    pub fn new(register_selection: RS, read_write: RW, enable: E, data_bus: DB) -> Self {
        Self {
            register_selection,
            read_write,
            enable,
            data_bus,
        }
    }

    pub(crate) fn pulse_enable(&mut self, delay: &mut impl DelayMicros) -> Result<(), E::Error> {
        self.enable.set_high()?;
        delay.delay_us(1);
        self.enable.set_low()?;
        delay.delay_us(1);
        Ok(())
    }

    pub fn state(
        &mut self,
        delay: &mut impl DelayMicros,
    ) -> Result<State, LcdError<RS, RW, E, DB>> {
        self.register_selection
            .set_low()
            .map_err(|e| LcdError::RegisterSelectionError(e))?;
        self.read_write
            .set_high()
            .map_err(|e| LcdError::ReadWriteError(e))?;
        self.pulse_enable(delay)
            .map_err(|e| LcdError::EnableError(e))?;
        let lower_bits = self
            .data_bus
            .read_pins_now()
            .map_err(|e| LcdError::DataBusError(e))?;
        self.pulse_enable(delay)
            .map_err(|e| LcdError::EnableError(e))?;
        let upper_bits = self
            .data_bus
            .read_pins_now()
            .map_err(|e| LcdError::DataBusError(e))?;
        let mut bits = bitarr!(u8, Lsb0; 0; 8);
        bits.iter_mut()
            .zip(lower_bits.into_iter().chain(upper_bits.into_iter()))
            .for_each(|(mut b, state)| {
                b.set(match state {
                    PinState::Low => false,
                    PinState::High => true,
                })
            });
        Ok(State(bits.load::<u8>()))
    }

    pub fn write(
        &mut self,
        delay: &mut impl DelayMicros,
        deliverable: Deliverable,
    ) -> nb::Result<(), LcdError<RS, RW, E, DB>> {
        if self.state(delay)?.busy() {
            Err(nb::Error::WouldBlock)
        } else {
            let datum = match deliverable {
                Deliverable::Instr(CompiledInstr(datum)) => {
                    self.register_selection
                        .set_low()
                        .map_err(|e| LcdError::RegisterSelectionError(e))?;
                    datum
                }
                Deliverable::Data(datum) => {
                    self.register_selection
                        .set_high()
                        .map_err(|e| LcdError::RegisterSelectionError(e))?;
                    datum
                }
            };
            self.read_write
                .set_low()
                .map_err(|e| LcdError::ReadWriteError(e))?;
            let (lower_bits, upper_bits) = datum.view_bits::<Lsb0>().split_at(4);
            self.data_bus
                .write_pins_now(lower_bits.iter().map(|b| match b.as_ref() {
                    false => PinState::Low,
                    true => PinState::High,
                }))
                .map_err(|e| LcdError::DataBusError(e))?;
            self.pulse_enable(delay)
                .map_err(|e| LcdError::EnableError(e))?;
            self.data_bus
                .write_pins_now(upper_bits.iter().map(|b| match b.as_ref() {
                    false => PinState::Low,
                    true => PinState::High,
                }))
                .map_err(|e| LcdError::DataBusError(e))?;
            self.pulse_enable(delay)
                .map_err(|e| LcdError::EnableError(e))?;
            Ok(())
        }
    }
}

impl<RS: OutputPin, RW: OutputPin, E: OutputPin, DB: DataBus> From<LcdPins<RS, RW, E, DB>>
    for (RS, RW, E, DB)
{
    #[inline]
    fn from(value: LcdPins<RS, RW, E, DB>) -> Self {
        (
            value.register_selection,
            value.read_write,
            value.enable,
            value.data_bus,
        )
    }
}

pub struct Lcd<RS: OutputPin, RW: OutputPin, E: OutputPin, DB: DataBus, D: DelayMicros> {
    pub(crate) pins: LcdPins<RS, RW, E, DB>,
    pub(crate) delay: D,
}

impl<RS: OutputPin, RW: OutputPin, E: OutputPin, DB: DataBus> LcdPins<RS, RW, E, DB> {
    #[inline]
    pub fn with_delay<D: DelayMicros>(self, delay: D) -> Lcd<RS, RW, E, DB, D> {
        Lcd { pins: self, delay }
    }
}

impl<RS: OutputPin, RW: OutputPin, E: OutputPin, DB: DataBus, D: DelayMicros>
    From<Lcd<RS, RW, E, DB, D>> for (LcdPins<RS, RW, E, DB>, D)
{
    fn from(value: Lcd<RS, RW, E, DB, D>) -> Self {
        (value.pins, value.delay)
    }
}

impl<RS: OutputPin, RW: OutputPin, E: OutputPin, DB: DataBus, D: DelayMicros>
    Lcd<RS, RW, E, DB, D>
{
    pub fn state(&mut self) -> Result<State, LcdError<RS, RW, E, DB>> {
        self.pins.state(&mut self.delay)
    }

    pub fn write(&mut self, deliverable: Deliverable) -> nb::Result<(), LcdError<RS, RW, E, DB>> {
        self.pins.write(&mut self.delay, deliverable)
    }
}

impl<RS: OutputPin, RW: OutputPin, E: OutputPin, DB: DataBus, D: DelayMicros> uWrite
    for Lcd<RS, RW, E, DB, D>
{
    type Error = nb::Error<LcdError<RS, RW, E, DB>>;

    fn write_char(&mut self, c: char) -> Result<(), Self::Error> {
        self.write(Deliverable::Data(c as u8))
    }

    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
        s.bytes()
            .try_for_each(|b| nb::block!(self.write(Deliverable::Data(b))))?;
        Ok(())
    }
}
