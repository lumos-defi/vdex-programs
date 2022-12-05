use std::{
    cell::RefCell,
    ops::{Div, Mul},
    rc::Rc,
};

use crate::utils::{
    assert_eq_with_dust, convert_to_big_number, create_associated_token_account, get_dex_info,
    get_keypair, get_program, get_token_balance, mint_tokens, set_add_liquidity, set_close,
    set_feed_mock_oracle, set_open, set_remove_liquidity, set_user_state, transfer, DexAsset,
    DexMarket, TEST_SOL_DECIMALS, TEST_USDC_DECIMALS,
};
use anchor_client::{
    solana_sdk::{account::Account, signature::Keypair, signer::Signer, transport::TransportError},
    Program,
};
use anchor_lang::prelude::{AccountInfo, AccountMeta, Pubkey};

use crate::utils::constant::TEST_VLP_DECIMALS;
use crate::utils::TestResult;
use dex_program::{
    dex::{Dex, MockOracle},
    user::UserState,
    utils::USDC_POW_DECIMALS,
};
use solana_program_test::ProgramTestContext;
use spl_associated_token_account::get_associated_token_address;

pub struct UserTestContext {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub program: Program,
    pub admin: Keypair,
    pub user: Keypair,
    pub user_state: Pubkey,
    pub dex: Pubkey,
    pub dex_info: RefCell<Dex>,
}

#[allow(dead_code)]
impl UserTestContext {
    pub async fn new(context: Rc<RefCell<ProgramTestContext>>, dex: Pubkey) -> UserTestContext {
        let program = get_program().await;
        let user = Keypair::new();

        let transfer_sol_amount = 10_000_000_000;
        transfer(
            &mut context.borrow_mut(),
            &user.pubkey(),
            transfer_sol_amount,
        )
        .await;

        //init user state
        let user_state =
            set_user_state::setup(&mut context.borrow_mut(), &program, &user, &dex).await;

        let dex_info = get_dex_info(&mut context.borrow_mut().banks_client, dex).await;

        let admin = get_keypair("tests/fixtures/admin.json").await;

        UserTestContext {
            context,
            program,
            admin,
            user_state,
            user,
            dex,
            dex_info,
        }
    }

    pub async fn refresh_dex_info(&mut self) {
        self.dex_info = get_dex_info(&mut self.context.borrow_mut().banks_client, self.dex).await;
    }

    pub async fn get_account(&self, account_pubkey: Pubkey) -> Account {
        let account = self
            .context
            .borrow_mut()
            .banks_client
            .get_account(account_pubkey)
            .await
            .unwrap()
            .unwrap();

        account
    }

    pub async fn get_ref_account(&self, account_pubkey: Pubkey) -> RefCell<Account> {
        let account = self
            .context
            .borrow_mut()
            .banks_client
            .get_account(account_pubkey)
            .await
            .unwrap()
            .unwrap();

        RefCell::new(account)
    }

    pub async fn get_mock_oracle_price(&self, oracle: Pubkey) -> u64 {
        let mock_oracle = self.get_mock_oracle_account_info(oracle).await;
        mock_oracle
            .price
            .div(10u64.pow(mock_oracle.expo as u32))
            .mul(USDC_POW_DECIMALS)
    }

    pub async fn get_mock_oracle_account_info(&self, oracle: Pubkey) -> &MockOracle {
        let oracle_account = self.get_account(oracle).await;
        let data_ptr = oracle_account.data.as_ptr();
        let mock_oracle = unsafe { data_ptr.add(8).cast::<MockOracle>().as_ref() }.unwrap();

        mock_oracle
    }

    pub async fn feed_asset_mock_oracle_price(&self, asset: usize, price: f64) {
        let asset_info = self.dex_info.borrow().assets[asset];
        let oracle_info = self.get_mock_oracle_account_info(asset_info.oracle).await;
        let new_market_oracle_price = convert_to_big_number(price, oracle_info.expo);
        set_feed_mock_oracle::setup(
            &mut self.context.borrow_mut(),
            &self.program,
            &self.admin,
            &asset_info.oracle,
            new_market_oracle_price,
        )
        .await
        .unwrap();
    }

    pub async fn feed_market_mock_oracle_price(&self, market: u8, price: f64) {
        let market_info = self.dex_info.borrow().markets[market as usize];
        let oracle_info = self.get_mock_oracle_account_info(market_info.oracle).await;
        let new_market_oracle_price = convert_to_big_number(price.into(), oracle_info.expo);

        set_feed_mock_oracle::setup(
            &mut self.context.borrow_mut(),
            &self.program,
            &self.admin,
            &market_info.oracle,
            new_market_oracle_price,
        )
        .await
        .unwrap();
    }

    pub fn asset_index(&self, mint_name: &str) -> usize {
        self.dex_info
            .borrow()
            .assets
            .iter()
            .position(|&a| {
                let mut name: [u8; 16] = Default::default();
                let usdc_name = mint_name.as_bytes();

                name[..usdc_name.len()].copy_from_slice(usdc_name);

                a.valid && a.symbol == name
            })
            .unwrap()
    }

    pub async fn feed_btc_price(&self, price: f64) {
        self.feed_market_mock_oracle_price(DexMarket::BTC as u8, price)
            .await;
        self.feed_asset_mock_oracle_price(self.asset_index("BTC"), price)
            .await
    }

    pub async fn feed_eth_price(&self, price: f64) {
        self.feed_market_mock_oracle_price(DexMarket::ETH as u8, price)
            .await;
        self.feed_asset_mock_oracle_price(self.asset_index("ETH"), price)
            .await
    }

    pub async fn feed_sol_price(&self, price: f64) {
        self.feed_market_mock_oracle_price(DexMarket::SOL as u8, price)
            .await;
        self.feed_asset_mock_oracle_price(self.asset_index("SOL"), price)
            .await
    }

    pub async fn generate_random_user(&self) -> Keypair {
        let user = Keypair::new();
        //transfer sol to user
        let transfer_sol_amount = 100_000_000_000;
        transfer(
            &mut self.context.borrow_mut(),
            &user.pubkey(),
            transfer_sol_amount,
        )
        .await;
        user
    }

    pub async fn get_oracle_remaining_accounts(&self) -> Vec<AccountMeta> {
        let mut remaining_accounts: Vec<AccountMeta> = Vec::new();

        //process dex asset oracle account
        for asset in &self.dex_info.borrow().assets {
            if asset.valid {
                remaining_accounts.append(&mut vec![AccountMeta::new(asset.oracle, false)])
            }
        }

        //process dex market oracle account
        for market in &self.dex_info.borrow().markets {
            if market.valid {
                remaining_accounts.append(&mut vec![AccountMeta::new(market.oracle, false)])
            }
        }

        remaining_accounts
    }

    pub async fn get_market_oracle_remaining_accounts(&self) -> Vec<AccountMeta> {
        let mut remaining_accounts: Vec<AccountMeta> = Vec::new();

        //process dex market oracle account
        for market in &self.dex_info.borrow().markets {
            if market.valid {
                remaining_accounts.append(&mut vec![AccountMeta::new(market.oracle, false)])
            }
        }

        remaining_accounts
    }

    pub async fn get_user_list_remaining_accounts(&self) -> Vec<AccountMeta> {
        let mut remaining_accounts: Vec<AccountMeta> = Vec::new();

        for i in 0..self.dex_info.borrow().user_list_remaining_pages_number as usize {
            let page = self.dex_info.borrow().user_list_remaining_pages[i];
            remaining_accounts.append(&mut vec![AccountMeta::new(page, false)])
        }

        remaining_accounts
    }

    pub async fn mint_usdc(&self, amount: f64) {
        let usdc_asset = self.dex_info.borrow().assets[DexAsset::USDC as usize];
        let mint_amount = convert_to_big_number(amount, usdc_asset.decimals);

        self.mint_asset(&usdc_asset.mint, &self.admin, mint_amount)
            .await;
    }

    pub async fn mint_btc(&self, amount: f64) {
        let btc_asset = self.dex_info.borrow().assets[DexAsset::BTC as usize];
        let mint_amount = convert_to_big_number(amount, btc_asset.decimals);

        self.mint_asset(&btc_asset.mint, &self.admin, mint_amount)
            .await;
    }

    pub async fn mint_eth(&self, amount: f64) {
        let eth_asset = self.dex_info.borrow().assets[DexAsset::ETH as usize];
        let mint_amount = convert_to_big_number(amount, eth_asset.decimals);

        self.mint_asset(&eth_asset.mint, &self.admin, mint_amount)
            .await;
    }

    pub async fn mint_sol(&self, amount: f64) {
        let sol_asset = self.dex_info.borrow().assets[DexAsset::SOL as usize];
        let mint_amount = convert_to_big_number(amount, sol_asset.decimals);

        self.mint_asset(&sol_asset.mint, &self.admin, mint_amount)
            .await;
    }

    async fn mint_asset(&self, mint: &Pubkey, mint_authority: &Keypair, amount: u64) {
        if *mint == spl_token::native_mint::id() {
            transfer(&mut self.context.borrow_mut(), &self.user.pubkey(), amount).await;
            return;
        }

        let user_mint_acc = get_associated_token_address(&self.user.pubkey(), &mint);
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        //create user asset associated token account
        match context.banks_client.get_account(user_mint_acc).await {
            Ok(None) => {
                create_associated_token_account(context, &self.user, &self.user.pubkey(), mint)
                    .await
            }
            Ok(Some(_)) => {} //if exists do nothing
            Err(_) => {}
        }

        {
            mint_tokens(
                context,
                &self.admin,
                mint,
                &user_mint_acc,
                mint_authority,
                amount,
            )
            .await
            .unwrap();
        }
    }

    pub async fn add_liquidity_with_usdc(&self, amount: f64) {
        self.mint_usdc(amount).await;
        self.add_liquidity(DexAsset::USDC as u8, amount).await;
    }

    pub async fn add_liquidity_with_btc(&self, amount: f64) {
        self.mint_btc(amount).await;
        self.add_liquidity(DexAsset::BTC as u8, amount).await;
    }

    pub async fn add_liquidity_with_eth(&self, amount: f64) {
        self.mint_eth(amount).await;
        self.assert_eth_balance(amount).await;
        self.add_liquidity(DexAsset::ETH as u8, amount).await;
    }

    pub async fn add_liquidity_with_sol(&self, amount: f64) {
        self.mint_sol(amount).await;
        self.add_liquidity(DexAsset::SOL as u8, amount).await;
    }

    async fn add_liquidity(&self, asset: u8, amount: f64) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();
        let asset_info = self.dex_info.borrow().assets[asset as usize];
        let deposit_amount = convert_to_big_number(amount, asset_info.decimals);
        let remaining_accounts = self.get_oracle_remaining_accounts().await;

        set_add_liquidity::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &asset_info.mint,
            &asset_info.vault,
            &self.dex_info.borrow().event_queue,
            &self.user_state,
            deposit_amount,
            remaining_accounts,
        )
        .await
        .unwrap();
    }

    pub async fn open(
        &self,
        in_asset: DexAsset,
        market: DexMarket,
        long: bool,
        amount: f64,
        leverage: u32,
    ) -> Result<(), TransportError> {
        let di = self.dex_info.borrow();
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();
        let ai = self.dex_info.borrow().assets[in_asset as usize];
        let open_amount = convert_to_big_number(amount, ai.decimals);

        let mi = di.markets[market as usize];

        let in_mint = ai.mint;
        let in_mint_oracle = ai.oracle;
        let in_mint_vault = ai.vault;

        let mai = if long {
            di.assets[mi.asset_index as usize]
        } else {
            di.assets[di.usdc_asset_index as usize]
        };

        let market_mint = mai.mint;
        let market_mint_oracle = mai.oracle;
        let market_mint_vault = mai.vault;

        let market_oracle = mi.oracle;

        let user_state = self.user_state;

        let remaining_accounts = self.get_user_list_remaining_accounts().await;

        set_open::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &in_mint,
            &in_mint_oracle,
            &in_mint_vault,
            &market_mint,
            &market_mint_oracle,
            &market_mint_vault,
            &market_oracle,
            &user_state,
            &di.event_queue,
            &di.user_list_entry_page,
            remaining_accounts,
            market as u8,
            long,
            open_amount,
            leverage,
        )
        .await
    }

    pub async fn assert_open(
        &self,
        in_asset: DexAsset,
        market: DexMarket,
        long: bool,
        amount: f64,
        leverage: u32,
    ) {
        self.open(in_asset, market, long, amount, leverage)
            .await
            .assert_ok();
    }

    pub async fn assert_open_fail(
        &self,
        in_asset: DexAsset,
        market: DexMarket,
        long: bool,
        amount: f64,
        leverage: u32,
    ) {
        self.open(in_asset, market, long, amount, leverage)
            .await
            .assert_err();
    }

    pub async fn close(
        &self,
        market: DexMarket,
        long: bool,
        size: f64,
    ) -> Result<(), TransportError> {
        let di = self.dex_info.borrow();
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let mi = di.markets[market as usize];
        let close_size = convert_to_big_number(size, mi.decimals);

        let mai = if long {
            di.assets[mi.asset_index as usize]
        } else {
            di.assets[di.usdc_asset_index as usize]
        };

        let mint = mai.mint;
        let vault = mai.vault;
        let program_signer = mai.program_signer;

        let oracle = mi.oracle;

        let user_state = self.user_state;
        let remaining_accounts = self.get_user_list_remaining_accounts().await;

        set_close::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &mint,
            &oracle,
            &vault,
            &program_signer,
            &user_state,
            &di.event_queue,
            &di.user_list_entry_page,
            remaining_accounts,
            market as u8,
            long,
            close_size,
        )
        .await
    }

    pub async fn assert_close(&self, market: DexMarket, long: bool, size: f64) {
        self.close(market, long, size).await.assert_ok();
    }

    pub async fn assert_close_fail(&self, market: DexMarket, long: bool, size: f64) {
        self.close(market, long, size).await.assert_err();
    }

    pub async fn remove_liquidity_withdraw_usdc(&self, vlp_amount: f64) {
        self.remove_liquidity(DexAsset::USDC as u8, vlp_amount)
            .await;
    }

    pub async fn remove_liquidity_withdraw_btc(&self, vlp_amount: f64) {
        self.remove_liquidity(DexAsset::BTC as u8, vlp_amount).await;
    }

    pub async fn remove_liquidity_withdraw_eth(&self, vlp_amount: f64) {
        self.remove_liquidity(DexAsset::ETH as u8, vlp_amount).await;
    }

    pub async fn remove_liquidity_withdraw_sol(&self, vlp_amount: f64) {
        self.remove_liquidity(DexAsset::SOL as u8, vlp_amount).await;
    }

    async fn remove_liquidity(&self, asset: u8, vlp_amount: f64) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();
        let asset_info = self.dex_info.borrow().assets[asset as usize];
        let withdraw_vlp_amount = convert_to_big_number(vlp_amount, TEST_VLP_DECIMALS);
        let remaining_accounts = self.get_oracle_remaining_accounts().await;

        set_remove_liquidity::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &asset_info.mint,
            &asset_info.vault,
            &asset_info.program_signer,
            &self.dex_info.borrow().event_queue,
            &self.user_state,
            withdraw_vlp_amount,
            remaining_accounts,
        )
        .await
        .unwrap();
    }

    pub async fn assert_usdc_balance(&self, amount: f64) {
        let user_asset_acc = self.get_user_usdc_token_pubkey().await;
        self.assert_mint_balance(&user_asset_acc, DexAsset::USDC as usize, amount)
            .await;
    }

    pub async fn assert_btc_balance(&self, amount: f64) {
        let user_asset_acc = self.get_user_btc_token_pubkey().await;
        self.assert_mint_balance(&user_asset_acc, DexAsset::BTC as usize, amount)
            .await;
    }

    pub async fn assert_eth_balance(&self, amount: f64) {
        let user_asset_acc = self.get_user_eth_token_pubkey().await;
        self.assert_mint_balance(&user_asset_acc, DexAsset::ETH as usize, amount)
            .await;
    }

    pub async fn balance(&self) -> u64 {
        self.get_account(self.user.pubkey()).await.lamports
    }

    pub async fn assert_mint_balance(&self, user_asset_acc: &Pubkey, asset: usize, amount: f64) {
        let asset_info = self.dex_info.borrow().assets[asset];
        let asset_amount =
            get_token_balance(&mut self.context.borrow_mut().banks_client, user_asset_acc).await;

        // assert_eq!(
        //     asset_amount,
        //     convert_to_big_number(amount.into(), asset_info.decimals)
        // );

        println!(
            "{} {} {}",
            amount,
            asset_amount,
            convert_to_big_number(amount.into(), asset_info.decimals)
        );

        assert!(asset_amount - convert_to_big_number(amount.into(), asset_info.decimals) <= 2);
    }

    pub async fn assert_vlp(&self, amount: f64) {
        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let ref_us = us.borrow();

        let vlp_amount = ref_us.meta.vlp.staked;

        assert_eq_with_dust(vlp_amount, convert_to_big_number(amount, TEST_VLP_DECIMALS));
    }

    // pub async fn assert_vlp_amount(&self, user_mint_acc: &Pubkey, amount: f64) {
    //     let vlp_account = self.get_account(self.dex_info.borrow().vlp_mint).await;
    //     let vlp_mint_info = Mint::try_deserialize(&mut vlp_account.data.as_ref()).unwrap();
    //     let asset_amount =
    //         get_token_balance(&mut self.context.borrow_mut().banks_client, user_mint_acc).await;

    //     assert_eq!(
    //         asset_amount,
    //         convert_to_big_number(amount.into(), vlp_mint_info.decimals)
    //     );
    // }

    // pub async fn get_user_vlp_token_pubkey(&self) -> Pubkey {
    //     let user_mint_acc =
    //         get_associated_token_address(&self.user.pubkey(), &self.dex_info.borrow().vlp_mint);
    //     user_mint_acc
    // }

    pub async fn get_user_usdc_token_pubkey(&self) -> Pubkey {
        self.get_user_asset_token_pubkey(DexAsset::USDC as usize)
            .await
    }

    pub async fn get_user_btc_token_pubkey(&self) -> Pubkey {
        self.get_user_asset_token_pubkey(DexAsset::BTC as usize)
            .await
    }

    pub async fn get_user_eth_token_pubkey(&self) -> Pubkey {
        self.get_user_asset_token_pubkey(DexAsset::ETH as usize)
            .await
    }

    pub async fn get_user_sol_token_pubkey(&self) -> Pubkey {
        self.get_user_asset_token_pubkey(DexAsset::SOL as usize)
            .await
    }

    pub async fn get_user_asset_token_pubkey(&self, asset: usize) -> Pubkey {
        let asset_info = self.dex_info.borrow().assets[asset];
        let user_mint_acc = get_associated_token_address(&self.user.pubkey(), &asset_info.mint);
        user_mint_acc
    }

    pub async fn assert_vlp_total(&self, amount: f64) {
        let di = get_dex_info(&mut self.context.borrow_mut().banks_client, self.dex).await;
        let staked_total = di.borrow().vlp_pool.staked_total;

        assert_eq_with_dust(
            staked_total,
            convert_to_big_number(amount.into(), TEST_VLP_DECIMALS),
        );
    }

    pub async fn assert_vlp_rewards(&self, amount: f64) {
        let di = get_dex_info(&mut self.context.borrow_mut().banks_client, self.dex).await;
        let reward_total = di.borrow().vlp_pool.reward_total;

        assert_eq_with_dust(
            reward_total,
            convert_to_big_number(amount.into(), TEST_SOL_DECIMALS),
        );
    }

    pub async fn assert_liquidity(&self, asset: DexAsset, amount: f64) {
        let di = get_dex_info(&mut self.context.borrow_mut().banks_client, self.dex).await;
        let ai = di.borrow().assets[asset as usize];
        let expect = convert_to_big_number(amount, ai.decimals);
        assert_eq_with_dust(expect, ai.liquidity_amount);
    }

    pub async fn assert_fee(&self, asset: DexAsset, fee: f64) {
        let di = get_dex_info(&mut self.context.borrow_mut().banks_client, self.dex).await;
        let ai = di.borrow().assets[asset as usize];

        let expect = convert_to_big_number(fee, ai.decimals);
        assert_eq_with_dust(expect, ai.fee_amount);
    }

    pub async fn assert_fee_big(&self, asset: DexAsset, big_fee: u64) {
        let di = get_dex_info(&mut self.context.borrow_mut().banks_client, self.dex).await;
        let ai = di.borrow().assets[asset as usize];

        assert_eq_with_dust(big_fee, ai.fee_amount);
    }

    pub async fn assert_borrow(&self, asset: DexAsset, fee: f64) {
        let di = get_dex_info(&mut self.context.borrow_mut().banks_client, self.dex).await;
        let ai = di.borrow().assets[asset as usize];
        let expect = convert_to_big_number(fee, ai.decimals);
        assert_eq_with_dust(expect, ai.borrowed_amount);
    }

    pub async fn assert_collateral(&self, asset: DexAsset, fee: f64) {
        let di = get_dex_info(&mut self.context.borrow_mut().banks_client, self.dex).await;
        let ai = di.borrow().assets[asset as usize];
        let expect = convert_to_big_number(fee, ai.decimals);
        assert_eq_with_dust(expect, ai.collateral_amount);
    }

    pub async fn assert_global_long(&self, market: DexMarket, price: f64, size: f64) {
        let di = get_dex_info(&mut self.context.borrow_mut().banks_client, self.dex).await;
        let mi = di.borrow().markets[market as usize];
        let expect_price = convert_to_big_number(price, TEST_USDC_DECIMALS);
        let expect_size = convert_to_big_number(size, mi.decimals);

        assert_eq_with_dust(expect_price, mi.global_long.average_price);
        assert_eq_with_dust(expect_size, mi.global_long.size);
    }

    pub async fn assert_global_short(&self, market: DexMarket, price: f64, size: f64) {
        let di = get_dex_info(&mut self.context.borrow_mut().banks_client, self.dex).await;
        let mi = di.borrow().markets[market as usize];
        let expect_price = convert_to_big_number(price, TEST_USDC_DECIMALS);
        let expect_size = convert_to_big_number(size, mi.decimals);

        assert_eq_with_dust(expect_price, mi.global_short.average_price);
        assert_eq_with_dust(expect_size, mi.global_short.size);
    }

    pub async fn assert_position(
        &self,
        market: DexMarket,
        long: bool,
        price: f64,
        size: f64,
        collateral: f64,
        borrow: f64,
        closing_size: f64,
    ) {
        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let ref_us = us.borrow();

        let di = get_dex_info(&mut self.context.borrow_mut().banks_client, self.dex).await;
        let mi = di.borrow().markets[market as usize];

        let position = ref_us
            .find_or_new_position(market as u8, false)
            .assert_unwrap();

        let decimals = if long {
            mi.decimals
        } else {
            TEST_USDC_DECIMALS
        };

        let expect_price = convert_to_big_number(price, TEST_USDC_DECIMALS);
        let expect_size = convert_to_big_number(size, mi.decimals);
        let expect_collateral = convert_to_big_number(collateral, decimals);
        let expect_borrow = convert_to_big_number(borrow, decimals);
        let expect_closing_size = convert_to_big_number(closing_size, mi.decimals);

        if long {
            assert_eq_with_dust(expect_price, position.data.long.average_price);
            assert_eq_with_dust(expect_size, position.data.long.size);
            assert_eq_with_dust(expect_collateral, position.data.long.collateral);
            assert_eq_with_dust(expect_borrow, position.data.long.borrowed_amount);
            assert_eq_with_dust(expect_closing_size, position.data.long.closing_size);
        } else {
            assert_eq_with_dust(expect_price, position.data.short.average_price);
            assert_eq_with_dust(expect_size, position.data.short.size);
            assert_eq_with_dust(expect_collateral, position.data.short.collateral);
            assert_eq_with_dust(expect_borrow, position.data.short.borrowed_amount);
            assert_eq_with_dust(expect_closing_size, position.data.short.closing_size);
        }
    }
}
