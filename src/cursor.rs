use std::ops::{BitAnd, BitOr};

/// Cursor-styles.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Block,
    Underscore,
    BoldUnderscore,
    #[default]
    Bar,
    BoldBar,
    RtlBar,
    RtlBoldBar,
}

impl CursorStyle {
    pub(crate) fn to_ltr(self) -> CursorStyle {
        match self {
            CursorStyle::RtlBar => CursorStyle::Bar,
            CursorStyle::RtlBoldBar => CursorStyle::RtlBoldBar,
            v => v,
        }
    }

    pub(crate) fn to_rtl(self) -> CursorStyle {
        match self {
            CursorStyle::Bar => CursorStyle::RtlBar,
            CursorStyle::BoldBar => CursorStyle::RtlBoldBar,
            v => v,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Blinking(u8);

impl Blinking {
    pub const CURSOR: Blinking = Blinking(1);
    pub const TEXT: Blinking = Blinking(2);
}

impl BitOr for Blinking {
    type Output = Blinking;

    fn bitor(self, rhs: Self) -> Self::Output {
        Blinking(self.0 | rhs.0)
    }
}

impl BitAnd for Blinking {
    type Output = bool;

    fn bitand(self, rhs: Self) -> Self::Output {
        (self.0 & rhs.0) != 0
    }
}
