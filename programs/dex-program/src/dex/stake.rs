use anchor_lang::prelude::*;

use crate::{
    errors::{DexError, DexResult},
    utils::{SafeMath, REWARD_SHARE_POW_DECIMALS},
};

pub struct StakingPool {
    pub vault: Pubkey,
    pub program_signer: Pubkey,
    pub reward_total: u64,
    pub staked_total: u64,
    pub accumulate_reward_per_share: u64,
    pub reward_asset_index: u8,
    pub decimals: u8,
    pub nonce: u8,
    pub padding: [u8; 6],
}

impl StakingPool {
    pub fn init(
        &mut self,
        vault: Pubkey,
        program_signer: Pubkey,
        nonce: u8,
        decimals: u8,
        reward_asset_index: u8,
    ) {
        self.vault = vault;
        self.program_signer = program_signer;
        self.decimals = decimals;
        self.nonce = nonce;
        self.decimals = decimals;
        self.reward_asset_index = reward_asset_index;

        self.reward_total = 0;
        self.staked_total = 0;
        self.accumulate_reward_per_share = 0;
    }

    pub fn add_reward(&mut self, reward: u64) -> DexResult {
        self.reward_total = self.reward_total.safe_add(reward)?;
        if self.staked_total == 0 {
            return Ok(());
        }

        let delta = reward
            .safe_mul(REWARD_SHARE_POW_DECIMALS)?
            .safe_div(self.staked_total as u128)? as u64;

        self.accumulate_reward_per_share = self.accumulate_reward_per_share.safe_add(delta)?;

        Ok(())
    }

    #[inline]
    pub fn increase_staking(&mut self, amount: u64) -> DexResult {
        self.staked_total = self.staked_total.safe_add(amount)?;
        Ok(())
    }

    #[inline]
    pub fn decrease_staking(&mut self, amount: u64) -> DexResult {
        self.staked_total = self.staked_total.safe_sub(amount)?;
        Ok(())
    }

    #[inline]
    pub fn withdraw_reward(&mut self, pending: u64) -> DexResult {
        self.reward_total = self.reward_total.safe_sub(pending)?;
        Ok(())
    }
}

// #[derive(Clone, Copy)]
pub struct UserStake {
    pub staked: u64,
    pub reward_debt: u64,
    pub reward_accumulated: u64,
}

impl UserStake {
    pub fn init(&mut self) {
        self.staked = 0;
        self.reward_debt = 0;
        self.reward_accumulated = 0;
    }

    pub fn enter_staking(&mut self, pool: &mut StakingPool, amount: u64) -> DexResult {
        require!(amount > 0, DexError::InvalidAmount);

        if self.staked > 0 {
            let pending = (self
                .staked
                .safe_mul(pool.accumulate_reward_per_share)?
                .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64)
                .safe_sub(self.reward_debt)?;

            self.reward_accumulated = self.reward_accumulated.safe_add(pending)?;
            pool.withdraw_reward(pending)?;
        }

        pool.increase_staking(amount)?;
        self.staked = self.staked.safe_add(amount)?;

        self.reward_debt = self
            .staked
            .safe_mul(pool.accumulate_reward_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64;

        Ok(())
    }

    pub fn leave_staking(&mut self, pool: &mut StakingPool, amount: u64) -> DexResult {
        require!(amount > 0, DexError::InvalidAmount);
        let pending = (self
            .staked
            .safe_mul(pool.accumulate_reward_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64)
            .safe_sub(self.reward_debt)?;

        if pending > 0 {
            self.reward_accumulated = self.reward_accumulated.safe_add(pending)?;
            pool.withdraw_reward(pending)?;
        }

        pool.decrease_staking(amount)?;
        self.staked = self.staked.safe_sub(amount)?;

        self.reward_debt = self
            .staked
            .safe_mul(pool.accumulate_reward_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64;

        Ok(())
    }

    pub fn withdraw_reward(&mut self, pool: &StakingPool) -> DexResult<u64> {
        let pending = (self
            .staked
            .safe_mul(pool.accumulate_reward_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64)
            .safe_sub(self.reward_debt)?;

        let withdrawable = self.reward_accumulated.safe_add(pending)?;

        self.reward_accumulated = 0;
        self.reward_debt = self
            .staked
            .safe_mul(pool.accumulate_reward_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64;

        Ok(withdrawable)
    }

    pub fn pending_reward(&self, pool: &StakingPool) -> DexResult<u64> {
        let pending = (self
            .staked
            .safe_mul(pool.accumulate_reward_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64)
            .safe_sub(self.reward_debt)?;

        self.reward_accumulated.safe_add(pending)
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use super::*;
    use crate::utils::test::*;

    impl Default for StakingPool {
        fn default() -> StakingPool {
            unsafe { std::mem::zeroed() }
        }
    }

    impl Default for UserStake {
        fn default() -> UserStake {
            unsafe { std::mem::zeroed() }
        }
    }

    fn mock_staking_users(count: u32) -> Vec<UserStake> {
        let mut users: Vec<UserStake> = vec![];

        for _ in 0..count {
            users.push(UserStake::default());
        }

        users
    }

    #[test]
    //
    // 100 users have the same staking amount, will equally share the rewards.
    //
    fn test_equal_share() {
        let user_total = 100;
        let mut pool = StakingPool::default();
        let mut users = mock_staking_users(user_total);

        for user in &mut users {
            user.enter_staking(&mut pool, usdc(1000.)).assert_ok();
        }

        let reward = eth(10.);
        pool.add_reward(reward).assert_ok();

        for user in &users {
            assert_eq!(
                user.pending_reward(&pool).assert_unwrap(),
                reward / user_total as u64
            );
        }
    }

    #[test]
    //
    // 100 users have the different staking amount, will share the rewards proportionally.
    //
    fn test_proportional_share() {
        let user_total = 100;
        let mut pool = StakingPool::default();
        let mut users = mock_staking_users(user_total);

        for i in 0..user_total as usize {
            users[i]
                .enter_staking(&mut pool, usdc(1.0 * (i + 1) as f64))
                .assert_ok();
        }

        let mut reward = 0;
        let mut total_share = 0;
        for i in 0..user_total as usize {
            reward += eth(1.0 * (i + 1) as f64);
            total_share += (i + 1) as u64;
        }
        pool.add_reward(reward).assert_ok();

        let per_share = reward / total_share;
        assert_eq!(per_share, eth(1.0));

        for i in 0..user_total as usize {
            assert_eq!(
                users[i].pending_reward(&pool).assert_unwrap(),
                per_share * ((i + 1) as u64)
            );
        }
    }

    #[test]
    //
    // 100 users have the same staking amount and share the rewards. Then an new user enters in...
    //
    fn test_user_enter_staking() {
        let user_total = 100;
        let mut pool = StakingPool::default();
        let mut users = mock_staking_users(user_total);

        for user in &mut users {
            user.enter_staking(&mut pool, usdc(1000.)).assert_ok();
        }

        let reward = eth(10.);
        pool.add_reward(reward).assert_ok();

        for user in &users {
            assert_eq!(
                user.pending_reward(&pool).assert_unwrap(),
                reward / user_total as u64
            );
        }

        // An new user enters in
        let mut alice = UserStake::default();
        alice.enter_staking(&mut pool, usdc(1000.)).assert_ok();

        // Should not affect previous reward share
        for user in &users {
            assert_eq!(
                user.pending_reward(&pool).assert_unwrap(),
                reward / user_total as u64
            );
        }
        assert_eq!(alice.pending_reward(&pool).assert_unwrap(), 0);

        // Pool got additional rewards
        let additional_reward = eth(10.1);
        pool.add_reward(additional_reward).assert_ok();

        for user in &users {
            assert_eq!(
                user.pending_reward(&pool).assert_unwrap(),
                reward / user_total as u64 + additional_reward / (user_total as u64 + 1)
            );
        }

        assert_eq!(
            alice.pending_reward(&pool).assert_unwrap(),
            additional_reward / (user_total as u64 + 1)
        );
    }

    #[test]
    //
    // 100 users have the same staking amount and share the rewards. Then one of them leaves...
    //
    fn test_user_leave_staking() {
        let user_total = 100;
        let mut pool = StakingPool::default();
        let mut users = mock_staking_users(user_total);

        for user in &mut users {
            user.enter_staking(&mut pool, usdc(1000.)).assert_ok();
        }

        let reward = eth(10.);
        pool.add_reward(reward).assert_ok();

        for user in &users {
            assert_eq!(
                user.pending_reward(&pool).assert_unwrap(),
                reward / user_total as u64
            );
        }

        // One user leaves
        users[user_total as usize - 1]
            .leave_staking(&mut pool, usdc(1000.))
            .assert_ok();

        // Should not affect user's reward including the leaving one
        for user in &users {
            assert_eq!(
                user.pending_reward(&pool).assert_unwrap(),
                reward / user_total as u64
            );
        }

        // Pool got additional rewards
        let additional_reward = eth(9.9);
        pool.add_reward(additional_reward).assert_ok();

        for i in 0..user_total as usize - 1 {
            assert_eq!(
                users[i].pending_reward(&pool).assert_unwrap(),
                reward / user_total as u64 + additional_reward / (user_total as u64 - 1)
            );
        }

        assert_eq!(
            users[user_total as usize - 1]
                .pending_reward(&pool)
                .assert_unwrap(),
            reward / user_total as u64
        );
    }
}