pub mod constant;
pub mod helper;
pub mod instruction;
pub mod set_add_liquidity;
pub mod set_ask;
pub mod set_bid;
pub mod set_cancel;
pub mod set_cancel_all;
pub mod set_close;
pub mod set_crank;
pub mod set_feed_mock_oracle;
pub mod set_fill;
pub mod set_mock_oracle;
pub mod set_open;
pub mod set_remove_liquidity;
pub mod set_user_state;

pub mod state;

pub use constant::*;
pub use helper::*;
pub use instruction::*;
pub use set_add_liquidity::*;
pub use set_ask::*;
pub use set_bid::*;
pub use set_cancel::*;
pub use set_cancel_all::*;
pub use set_close::*;
pub use set_crank::*;
pub use set_feed_mock_oracle::*;
pub use set_fill::*;
pub use set_mock_oracle::*;
pub use set_open::*;
pub use set_remove_liquidity::*;
pub use set_user_state::*;
pub use state::*;

use std::fmt::Debug;

pub trait TestResult<T, E> {
    fn assert_unwrap(self) -> T;
    fn assert_err(self);
    fn assert_ok(self);
}
impl<T, E> TestResult<T, E> for Result<T, E>
where
    E: Debug,
{
    fn assert_unwrap(self) -> T {
        assert!(self.is_ok());
        self.unwrap()
    }

    fn assert_err(self) {
        assert!(self.is_err());
    }

    fn assert_ok(self) {
        assert!(self.is_ok());
    }
}
