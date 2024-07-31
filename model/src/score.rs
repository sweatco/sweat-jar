use crate::UDecimal;

pub type Score = u16;

pub trait ToAPY {
    fn to_apy(self) -> UDecimal;
}

impl ToAPY for Score {
    /// 1000 scores = 1%
    fn to_apy(self) -> UDecimal {
        UDecimal::new(self.into(), 5)
    }
}
