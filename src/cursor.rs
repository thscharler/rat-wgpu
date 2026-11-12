//! 
//! Cursor styling and blink control.
//! 

use std::ops::{BitAnd, BitOr};

/// Cursor-styles.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    /// Full block cursor.
    Block,
    /// Underline cursor.
    Underscore,
    /// Bold underline cursor.
    BoldUnderscore,
    #[default]
    /// Vertical bar.
    Bar,
    /// Bold vertical bar.
    BoldBar,
    /// Vertical bar at the right end of the Cell.
    RtlBar,
    /// Bold vertical bar at the right end of the Cell.
    RtlBoldBar,
}

impl CursorStyle {
    /// Switches to the matching LTR cursor. 
    /// 
    /// When using a bar-cursor it needs to appear on the left or
    /// the right edge of the cell-box dependent on the writing direction.
    /// Other cursor types are not changed.
    pub(crate) fn to_ltr(self) -> CursorStyle {
        match self {
            CursorStyle::RtlBar => CursorStyle::Bar,
            CursorStyle::RtlBoldBar => CursorStyle::RtlBoldBar,
            v => v,
        }
    }

    /// Switches to the matching RTL cursor. 
    ///
    /// When using a bar-cursor it needs to appear on the left or
    /// the right edge of the cell-box dependent on the writing direction.
    /// Other cursor types are not changed.
    pub(crate) fn to_rtl(self) -> CursorStyle {
        match self {
            CursorStyle::Bar => CursorStyle::RtlBar,
            CursorStyle::BoldBar => CursorStyle::RtlBoldBar,
            v => v,
        }
    }
}

/// Flag for [WgpuBackend::blink](crate::WgpuBackend::blink).
///
/// This decides which blinking effect is changed.
/// e.g. you can switch off cursor blinking and leave text blinking
/// active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Blinking(u8);

impl Blinking {
    /// Blink only the Cursor.
    pub const CURSOR: Blinking = Blinking(1);
    /// Blink only blinking text.
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
