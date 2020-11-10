use anyhow::{ensure, Result};
use thiserror::Error;

pub type OffsetControllerU32 = OffsetControllerT<u32>;

#[derive(Error, Debug)]
pub enum OffsetControllerError
{
    #[error("Offset already returned to controller")]
    AlreadyReturned,
}

#[derive(Debug, Clone)]
pub struct OffsetControllerT<T>
    where T: std::fmt::Debug + std::fmt::Display
{
    offsets: Vec<T>,
    multiplier: T,
}

impl<T> OffsetControllerT<T>
    where T: num::Integer + num::Zero + std::fmt::Debug + std::fmt::Display + Copy + Clone,
    std::ops::Range<T>: std::iter::Iterator,
    Vec<T>: std::iter::FromIterator<<std::ops::Range<T> as std::iter::Iterator>::Item>,
{
    pub fn new(amount: T, multiplier: T) -> Self {
        let offsets = (T::zero()..amount).collect();

        Self {
            offsets,
            multiplier,
        }
    }

    pub fn fetch_offset(&mut self) -> Option<T> {
        let offset = self.offsets.pop()?;
        Some(offset * self.multiplier)
    }

    pub fn return_offset(&mut self, value: T) -> Result<()> {
        let offset = value / self.multiplier;
        ensure!(
            !self.offsets.contains(&offset),
            OffsetControllerError::AlreadyReturned
        );
        self.offsets.push(offset);
        Ok(())
    }
    
    pub fn multiplier(&self) -> T {
        self.multiplier
    }
}