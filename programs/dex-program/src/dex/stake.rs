use anchor_lang::prelude::*;

use crate::{
    errors::{DexError, DexResult},
    utils::{SafeMath, REWARD_SHARE_POW_DECIMALS},
};

#[zero_copy]
pub struct StakingPool {
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub program_signer: Pubkey,
    pub reward_mint: Pubkey,
    pub reward_total: u64,
    pub staked_total: u64,
    pub accumulate_reward_per_share: u64,
    pub es_vdx_total: u64,
    pub accumulate_es_vdx_per_share: u64,
    pub reward_asset_index: u8,
    pub decimals: u8,
    pub nonce: u8,
    pub padding: [u8; 69],
}

impl StakingPool {
    pub fn init(
        &mut self,
        mint: Pubkey,
        vault: Pubkey,
        program_signer: Pubkey,
        reward_mint: Pubkey,
        nonce: u8,
        decimals: u8,
        reward_asset_index: u8,
    ) {
        self.mint = mint;
        self.vault = vault;
        self.program_signer = program_signer;
        self.reward_mint = reward_mint;
        self.decimals = decimals;
        self.nonce = nonce;
        self.decimals = decimals;
        self.reward_asset_index = reward_asset_index;

        self.reward_total = 0;
        self.staked_total = 0;
        self.accumulate_reward_per_share = 0;
    }

    pub fn add_rewards(&mut self, reward: u64) -> DexResult {
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

    pub fn add_es_vdx(&mut self, amount: u64) -> DexResult {
        self.es_vdx_total = self.es_vdx_total.safe_add(amount)?;
        if self.staked_total == 0 {
            return Ok(());
        }

        let delta = amount
            .safe_mul(REWARD_SHARE_POW_DECIMALS)?
            .safe_div(self.staked_total as u128)? as u64;

        self.accumulate_es_vdx_per_share = self.accumulate_es_vdx_per_share.safe_add(delta)?;

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

    #[inline]
    pub fn withdraw_es_vdx(&mut self, pending: u64) -> DexResult {
        self.es_vdx_total = self.es_vdx_total.safe_sub(pending)?;
        Ok(())
    }
}

pub struct UserStake {
    pub staked: u64,
    pub reward_debt: u64,
    pub reward_accumulated: u64,
    pub es_vdx_debt: u64,
    pub es_vdx_accumulated: u64,
    pub padding: [u8; 64],
}

impl UserStake {
    pub fn init(&mut self) {
        self.staked = 0;
        self.reward_debt = 0;
        self.reward_accumulated = 0;
        self.es_vdx_debt = 0;
        self.es_vdx_accumulated = 0;
    }

    pub fn enter_staking(&mut self, pool: &mut StakingPool, amount: u64) -> DexResult {
        require!(amount > 0, DexError::InvalidAmount);

        if self.staked > 0 {
            let pending_reward = (self
                .staked
                .safe_mul(pool.accumulate_reward_per_share)?
                .safe_div(REWARD_SHARE_POW_DECIMALS as u128)?
                as u64)
                .safe_sub(self.reward_debt)?;

            self.reward_accumulated = self.reward_accumulated.safe_add(pending_reward)?;
            pool.withdraw_reward(pending_reward)?;

            let pending_es_vdx = (self
                .staked
                .safe_mul(pool.accumulate_es_vdx_per_share)?
                .safe_div(REWARD_SHARE_POW_DECIMALS as u128)?
                as u64)
                .safe_sub(self.es_vdx_debt)?;

            self.es_vdx_accumulated = self.es_vdx_accumulated.safe_add(pending_es_vdx)?;
            pool.withdraw_es_vdx(pending_es_vdx)?;
        }

        pool.increase_staking(amount)?;
        self.staked = self.staked.safe_add(amount)?;

        self.reward_debt = self
            .staked
            .safe_mul(pool.accumulate_reward_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64;

        self.es_vdx_debt = self
            .staked
            .safe_mul(pool.accumulate_es_vdx_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64;

        Ok(())
    }

    pub fn leave_staking(&mut self, pool: &mut StakingPool, amount: u64) -> DexResult {
        require!(amount > 0, DexError::InvalidAmount);

        let pending_reward = (self
            .staked
            .safe_mul(pool.accumulate_reward_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64)
            .safe_sub(self.reward_debt)?;

        if pending_reward > 0 {
            self.reward_accumulated = self.reward_accumulated.safe_add(pending_reward)?;
            pool.withdraw_reward(pending_reward)?;
        }

        let pending_es_vdx = (self
            .staked
            .safe_mul(pool.accumulate_es_vdx_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64)
            .safe_sub(self.es_vdx_debt)?;

        if pending_es_vdx > 0 {
            self.es_vdx_accumulated = self.es_vdx_accumulated.safe_add(pending_es_vdx)?;
            pool.withdraw_es_vdx(pending_es_vdx)?;
        }

        pool.decrease_staking(amount)?;
        self.staked = self.staked.safe_sub(amount)?;

        self.reward_debt = self
            .staked
            .safe_mul(pool.accumulate_reward_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64;

        self.es_vdx_debt = self
            .staked
            .safe_mul(pool.accumulate_es_vdx_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64;

        Ok(())
    }

    pub fn withdraw_reward(&mut self, pool: &mut StakingPool) -> DexResult<u64> {
        let pending = (self
            .staked
            .safe_mul(pool.accumulate_reward_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64)
            .safe_sub(self.reward_debt)?;

        pool.withdraw_reward(pending)?;
        let withdrawable = self.reward_accumulated.safe_add(pending)?;

        self.reward_accumulated = 0;
        self.reward_debt = self
            .staked
            .safe_mul(pool.accumulate_reward_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64;

        Ok(withdrawable)
    }

    pub fn withdraw_es_vdx(&mut self, pool: &mut StakingPool) -> DexResult<u64> {
        let pending = (self
            .staked
            .safe_mul(pool.accumulate_es_vdx_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64)
            .safe_sub(self.es_vdx_debt)?;

        pool.withdraw_es_vdx(pending)?;
        let withdrawable = self.es_vdx_accumulated.safe_add(pending)?;

        self.es_vdx_accumulated = 0;
        self.es_vdx_debt = self
            .staked
            .safe_mul(pool.accumulate_es_vdx_per_share)?
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

    pub fn pending_es_vdx(&self, pool: &StakingPool) -> DexResult<u64> {
        let pending = (self
            .staked
            .safe_mul(pool.accumulate_es_vdx_per_share)?
            .safe_div(REWARD_SHARE_POW_DECIMALS as u128)? as u64)
            .safe_sub(self.es_vdx_debt)?;

        self.es_vdx_accumulated.safe_add(pending)
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
        pool.add_rewards(reward).assert_ok();

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
        pool.add_rewards(reward).assert_ok();

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
        pool.add_rewards(reward).assert_ok();

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
        pool.add_rewards(additional_reward).assert_ok();

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
        pool.add_rewards(reward).assert_ok();

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
        pool.add_rewards(additional_reward).assert_ok();

        for i in 0..user_total as usize - 1 {
            assert_eq!(
                users[i].pending_reward(&pool).assert_unwrap(),
                reward / user_total as u64 + additional_reward / (user_total as u64 - 1)
            );
        }

        // Check rewards
        let pending_reward = users[user_total as usize - 1]
            .pending_reward(&pool)
            .assert_unwrap();

        assert_eq!(pending_reward, reward / user_total as u64);
        assert_eq!(
            pool.reward_total,
            reward + additional_reward - pending_reward
        );

        // Withdraw rewards
        let withdrawable_reward = users[user_total as usize - 1]
            .withdraw_reward(&mut pool)
            .assert_unwrap();
        assert_eq!(pending_reward, withdrawable_reward);
        assert_eq!(
            pool.reward_total,
            reward + additional_reward - withdrawable_reward
        );

        let pending_reward = users[user_total as usize - 1]
            .pending_reward(&pool)
            .assert_unwrap();
        assert_eq!(pending_reward, 0);
    }

    #[test]
    //
    // 1. Alice add the first liquidity, she will have all the rewards( add liquidity fee );
    // 2. Bob add the second liquidity, he will share the generated rewards with Alice.
    //
    fn test_add_liquidity_process() {
        let mut pool = StakingPool::default();
        let mut alice = UserStake::default();
        let mut bob = UserStake::default();

        // 1. Initially no pending rewards
        assert_eq!(alice.pending_reward(&pool).assert_unwrap(), 0);
        assert_eq!(bob.pending_reward(&pool).assert_unwrap(), 0);

        // 2. Alice add liquidity and enter staking
        // No rewards available
        pool.add_rewards(0).assert_ok();
        alice.enter_staking(&mut pool, usdc(1000.)).assert_ok();
        assert_eq!(alice.pending_reward(&pool).assert_unwrap(), 0);

        // 3. Bob add liquidity and enter staking.
        // Rewards were generated when alice adding liquidity
        pool.add_rewards(eth(0.1)).assert_ok();

        bob.enter_staking(&mut pool, usdc(1000.)).assert_ok();

        assert_eq!(alice.pending_reward(&pool).assert_unwrap(), eth(0.1));
        assert_eq!(bob.pending_reward(&pool).assert_unwrap(), 0);

        // 4. Update rewards later, check the pending rewards
        pool.add_rewards(eth(0.1)).assert_ok();
        assert_eq!(alice.pending_reward(&pool).assert_unwrap(), eth(0.1 + 0.05));
        assert_eq!(bob.pending_reward(&pool).assert_unwrap(), eth(0.05));
    }
}
