use num_enum::TryFromPrimitive;

#[derive(Copy, Clone, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum PositionDirection {
    LONG = 0,
    SHORT = 1,
}

impl PositionDirection {
    pub fn opposite(self) -> Self {
        match self {
            PositionDirection::LONG => PositionDirection::SHORT,
            PositionDirection::SHORT => PositionDirection::LONG,
        }
    }
}

#[derive(Copy, Clone, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum PositionMove {
    OPEN = 0,
    CLOSE = 1,
}

#[derive(Copy, Clone, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum PositionMode {
    HEDGE = 0,
    ONEWAY = 1,
}
