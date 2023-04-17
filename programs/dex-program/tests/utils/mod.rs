pub mod constant;
pub mod helper;
pub mod instruction;
pub mod set_add_liquidity;
pub mod set_ask;
pub mod set_bid;
pub mod set_cancel;
pub mod set_cancel_all;
pub mod set_claim_rewards;
pub mod set_close;
pub mod set_compound;
pub mod set_crank;
pub mod set_di_buy;
pub mod set_di_create;
pub mod set_di_remove_option;
pub mod set_di_set_settle_price;
pub mod set_di_settle;
pub mod set_di_update_option;
pub mod set_di_withdraw_settled;
pub mod set_feed_mock_oracle;
pub mod set_fill;
pub mod set_market_swap;
pub mod set_mock_oracle;
pub mod set_open;
pub mod set_redeem_vdx;
pub mod set_remove_liquidity;
pub mod set_stake_vdx;
pub mod set_update_price;
pub mod set_user_state;
pub mod set_withdraw_asset;
pub mod state;

pub use constant::*;
pub use helper::*;
pub use instruction::*;
pub use set_add_liquidity::*;
pub use set_ask::*;
pub use set_bid::*;
pub use set_cancel::*;
pub use set_cancel_all::*;
pub use set_claim_rewards::*;
pub use set_close::*;
pub use set_compound::*;
pub use set_crank::*;
pub use set_di_buy::*;
pub use set_di_create::*;
pub use set_di_remove_option::*;
pub use set_di_set_settle_price::*;
pub use set_di_settle::*;
pub use set_di_update_option::*;
pub use set_di_withdraw_settled::*;
pub use set_feed_mock_oracle::*;
pub use set_fill::*;
pub use set_market_swap::*;
pub use set_mock_oracle::*;
pub use set_open::*;
pub use set_redeem_vdx::*;
pub use set_remove_liquidity::*;
pub use set_stake_vdx::*;
pub use set_update_price::*;
pub use set_user_state::*;
pub use set_withdraw_asset::*;
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
