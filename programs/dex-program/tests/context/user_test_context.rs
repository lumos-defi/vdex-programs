use std::{
    cell::RefCell,
    ops::{Div, Mul},
    rc::Rc,
};

use crate::utils::{
    assert_eq_with_dust, btc, convert_to_big_number, create_associated_token_account,
    create_token_account, get_dex_info, get_keypair, get_program, get_token_balance, mint_tokens,
    set_add_liquidity, set_ask, set_bid, set_cancel, set_cancel_all, set_close, set_crank,
    set_di_buy, set_di_create, set_di_remove_option, set_di_set_settle_price, set_di_settle,
    set_di_update_option, set_di_withdraw_settled, set_feed_mock_oracle, set_fill, set_market_swap,
    set_open, set_remove_liquidity, set_user_state, transfer, usdc, DexAsset, DexMarket,
    TEST_SOL_DECIMALS, TEST_USDC_DECIMALS,
};
use anchor_client::{
    solana_sdk::{
        account::Account,
        instruction::Instruction,
        signature::{read_keypair_file, Keypair},
        signer::Signer,
        transaction::Transaction,
        transport::TransportError,
    },
    Program,
};
use anchor_lang::{
    error,
    prelude::{AccountInfo, AccountMeta, Pubkey},
};

use crate::utils::constant::TEST_VLP_DECIMALS;
use crate::utils::TestResult;
use dex_program::{
    collections::{OrderBook, SingleEvent, SingleEventQueue},
    dex::{Dex, MockOracle},
    dual_invest::{DIOption, DI},
    errors::{DexError, DexResult},
    order::MatchEvent,
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

        let transfer_sol_amount = 10_000_000_000_000;
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
        self.context
            .borrow_mut()
            .banks_client
            .get_account(account_pubkey)
            .await
            .unwrap()
            .unwrap()
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
        let (price, expo) = self.get_mock_oracle_account_info(oracle).await;

        price.div(10u64.pow(expo as u32)).mul(USDC_POW_DECIMALS)
    }

    pub async fn get_mock_oracle_account_info(&self, oracle: Pubkey) -> (u64, u8) {
        let oracle_account = self.get_account(oracle).await;
        let data_ptr = oracle_account.data.as_ptr();

        let mock_oracle = unsafe { data_ptr.add(8).cast::<MockOracle>().as_ref() }.unwrap();

        (mock_oracle.price, mock_oracle.expo)
    }

    pub async fn feed_asset_mock_oracle_price(&self, asset: usize, price: f64) {
        let asset_info = self.dex_info.borrow().assets[asset];
        let (_, oracle_price_expo) = self.get_mock_oracle_account_info(asset_info.oracle).await;
        let new_market_oracle_price = convert_to_big_number(price, oracle_price_expo);
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
        let (_, oracle_price_expo) = self.get_mock_oracle_account_info(market_info.oracle).await;

        let new_market_oracle_price = convert_to_big_number(price.into(), oracle_price_expo);

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
        // self.feed_asset_mock_oracle_price(self.asset_index("BTC"), price)
        //     .await
    }

    pub async fn feed_eth_price(&self, price: f64) {
        self.feed_market_mock_oracle_price(DexMarket::ETH as u8, price)
            .await;
        // self.feed_asset_mock_oracle_price(self.asset_index("ETH"), price)
        //     .await
    }

    pub async fn feed_sol_price(&self, price: f64) {
        self.feed_market_mock_oracle_price(DexMarket::SOL as u8, price)
            .await;
        // self.feed_asset_mock_oracle_price(self.asset_index("SOL"), price)
        //     .await
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

    pub async fn get_market_order_pool_remaining_accounts(&self, market: u8) -> Vec<AccountMeta> {
        let mut remaining_accounts: Vec<AccountMeta> = Vec::new();

        assert!(market < self.dex_info.borrow().markets_number);

        let mi = &self.dex_info.borrow().markets[market as usize];
        //process dex market oracle account
        for i in 0..mi.order_pool_remaining_pages_number as usize {
            remaining_accounts.append(&mut vec![AccountMeta::new(
                mi.order_pool_remaining_pages[i],
                false,
            )])
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

    pub async fn mint_balance(&self, asset: DexAsset, user_asset_acc: &Pubkey) -> f64 {
        let asset_info = self.dex_info.borrow().assets[asset as usize];

        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();
        if let Ok(None) = context.banks_client.get_account(*user_asset_acc).await {
            create_associated_token_account(
                context,
                &self.user,
                &self.user.pubkey(),
                &asset_info.mint,
            )
            .await
        }

        let asset_amount = get_token_balance(&mut context.banks_client, user_asset_acc).await;

        asset_amount as f64 / 10f64.powf(asset_info.decimals.into())
    }

    pub async fn usdc_balance(&self) -> f64 {
        let user_asset_acc = self.get_user_usdc_token_pubkey().await;
        self.mint_balance(DexAsset::USDC, &user_asset_acc).await
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

        let difference =
            asset_amount as i64 - convert_to_big_number(amount.into(), asset_info.decimals) as i64;

        println!("diff {}", difference);
        assert!(difference.abs() <= 2);
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

    pub async fn get_position_size(&self, market: DexMarket, long: bool) -> u64 {
        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let ref_us = us.borrow();

        let position = ref_us
            .find_or_new_position(market as u8, false)
            .assert_unwrap();
        let size = if long {
            position.data.long.size
        } else {
            position.data.short.size
        };

        size
    }

    pub async fn crank(&self) {
        let payer = self.generate_random_user().await;
        let di = self.dex_info.borrow();

        let mut match_queue_account = self.get_account(di.match_queue).await;
        let match_queue_account_info: AccountInfo =
            (&di.match_queue, true, &mut match_queue_account).into();

        let match_queue =
            SingleEventQueue::<MatchEvent>::mount(&match_queue_account_info, true).assert_unwrap();
        if match_queue.read_head().is_err() {
            return; // No event
        }
        let SingleEvent { data } = match_queue.read_head().assert_unwrap();

        let user = Pubkey::new_from_array(data.user);

        let (user_state, _) = Pubkey::find_program_address(
            &[&self.dex.to_bytes(), &user.to_bytes()],
            &self.program.id(),
        );

        let mut user_state_account = self.get_account(user_state).await;
        let user_state_account_info: AccountInfo =
            (&user_state, true, &mut user_state_account).into();
        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let ref_us = us.borrow();

        let order = ref_us.get_order(data.user_order_slot).assert_unwrap();

        let in_asset = di.asset_as_ref(order.asset).assert_unwrap();

        let mai = di
            .market_asset_as_ref(order.market, order.long)
            .assert_unwrap();

        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();
        let remaining_accounts = self.get_user_list_remaining_accounts().await;

        let out_mint = if order.open {
            in_asset.mint // don't care
        } else {
            mai.mint
        };

        set_crank::setup(
            context,
            &self.program,
            &payer,
            &self.dex,
            order.open,
            &user,
            &user_state,
            &in_asset.mint,
            &in_asset.vault,
            &in_asset.oracle,
            &in_asset.program_signer,
            &mai.mint,
            &mai.oracle,
            &mai.vault,
            &mai.program_signer,
            &out_mint,
            &di.match_queue,
            &di.event_queue,
            &di.user_list_entry_page,
            remaining_accounts,
        )
        .await
        .unwrap();
    }

    pub async fn fill(&self, market: DexMarket) {
        let payer = self.generate_random_user().await;

        let di = self.dex_info.borrow();
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let mi = di.markets[market as usize];
        let remaining_accounts = self
            .get_market_order_pool_remaining_accounts(market as u8)
            .await;

        set_fill::setup(
            context,
            &self.program,
            &payer,
            &self.dex,
            &mi.oracle,
            &di.match_queue,
            &mi.order_book,
            &mi.order_pool_entry_page,
            remaining_accounts,
            market as u8,
        )
        .await
        .unwrap()
    }

    pub async fn ask(
        &self,
        market: DexMarket,
        long: bool,
        price: f64,
        size: f64,
    ) -> Result<(), TransportError> {
        let di = self.dex_info.borrow();
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let mi = di.markets[market as usize];
        let remaining_accounts = self
            .get_market_order_pool_remaining_accounts(market as u8)
            .await;

        set_ask::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &mi.oracle,
            &mi.order_book,
            &mi.order_pool_entry_page,
            &self.user_state,
            remaining_accounts,
            market as u8,
            long,
            convert_to_big_number(price, TEST_USDC_DECIMALS),
            convert_to_big_number(size, mi.decimals),
        )
        .await
    }

    pub async fn bid(
        &self,
        in_asset: DexAsset,
        market: DexMarket,
        long: bool,
        price: f64,
        amount: f64,
        leverage: u32,
    ) -> Result<(), TransportError> {
        let di = self.dex_info.borrow();
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();
        let ai = self.dex_info.borrow().assets[in_asset as usize];
        let bid_amount = convert_to_big_number(amount, ai.decimals);
        let bid_price = convert_to_big_number(price, TEST_USDC_DECIMALS);

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

        let market_oracle = mi.oracle;
        let order_book = mi.order_book;
        let order_pool_entry_page = mi.order_pool_entry_page;

        let user_state = self.user_state;

        let remaining_accounts = self
            .get_market_order_pool_remaining_accounts(market as u8)
            .await;

        set_bid::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &in_mint,
            &in_mint_oracle,
            &in_mint_vault,
            &market_oracle,
            &market_mint,
            &market_mint_oracle,
            &order_book,
            &order_pool_entry_page,
            &user_state,
            remaining_accounts,
            market as u8,
            long,
            bid_price,
            bid_amount,
            leverage,
        )
        .await
    }

    pub async fn assert_ask(&self, market: DexMarket, long: bool, price: f64, size: f64) {
        self.ask(market, long, price, size).await.assert_ok();
    }

    pub async fn assert_ask_fail(&self, market: DexMarket, long: bool, price: f64, size: f64) {
        self.ask(market, long, price, size).await.assert_err();
    }

    pub async fn assert_bid(
        &self,
        in_asset: DexAsset,
        market: DexMarket,
        long: bool,
        price: f64,
        amount: f64,
        leverage: u32,
    ) {
        self.bid(in_asset, market, long, price, amount, leverage)
            .await
            .assert_ok();
    }

    pub async fn assert_bid_fail(
        &self,
        in_asset: DexAsset,
        market: DexMarket,
        long: bool,
        price: f64,
        amount: f64,
        leverage: u32,
    ) {
        self.bid(in_asset, market, long, price, amount, leverage)
            .await
            .assert_err();
    }

    pub async fn cancel(&self, user_order_slot: u8) {
        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let ref_us = us.borrow();

        let order = ref_us.get_order(user_order_slot).assert_unwrap();

        let di = self.dex_info.borrow();
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let ai = di.asset_as_ref(order.asset).assert_unwrap();

        let mi = di.markets[order.market as usize];
        let remaining_accounts = self
            .get_market_order_pool_remaining_accounts(order.market)
            .await;

        set_cancel::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.user_state,
            &mi.order_book,
            &mi.order_pool_entry_page,
            &ai.mint,
            &ai.vault,
            &ai.program_signer,
            remaining_accounts,
            user_order_slot,
        )
        .await
        .unwrap()
    }

    pub async fn fail_to_cancel(&self, user_order_slot: u8) {
        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let ref_us = us.borrow();

        let order = ref_us.get_order(user_order_slot).assert_unwrap();

        let di = self.dex_info.borrow();
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let ai = di.asset_as_ref(order.asset).assert_unwrap();

        let mi = di.markets[order.market as usize];
        let remaining_accounts = self
            .get_market_order_pool_remaining_accounts(order.market)
            .await;

        set_cancel::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.user_state,
            &mi.order_book,
            &mi.order_pool_entry_page,
            &ai.mint,
            &ai.vault,
            &ai.program_signer,
            remaining_accounts,
            user_order_slot,
        )
        .await
        .assert_err()
    }

    pub async fn cancel_call(&self) {
        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let ref_us = us.borrow();

        let di = self.dex_info.borrow();
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let mut remaining_accounts: Vec<AccountMeta> = Vec::new();

        let mut close_wsol_account = false;
        let user_wsol_acc = Keypair::new();

        for market in 0..di.markets_number as usize {
            let bid_orders = ref_us.collect_orders(market, true);
            let ask_orders = ref_us.collect_orders(market, false);
            if bid_orders.is_empty() && ask_orders.is_empty() {
                continue;
            }

            let mi = di.markets[market];

            remaining_accounts.append(&mut vec![AccountMeta::new(mi.order_book, false)]);
            remaining_accounts.append(&mut vec![AccountMeta::new(mi.order_pool_entry_page, false)]);

            for r in 0..mi.order_pool_remaining_pages_number as usize {
                remaining_accounts.append(&mut vec![AccountMeta::new(
                    mi.order_pool_remaining_pages[r],
                    false,
                )]);
            }

            // Bid orders
            for user_order_slot in bid_orders {
                let order = ref_us.get_order(user_order_slot).unwrap();

                let ai = di.assets[order.asset as usize];
                remaining_accounts.append(&mut vec![AccountMeta::new(ai.mint, false)]);
                remaining_accounts.append(&mut vec![AccountMeta::new(ai.vault, false)]);
                remaining_accounts.append(&mut vec![AccountMeta::new(ai.program_signer, false)]);

                let user_mint_acc = if ai.mint == spl_token::native_mint::id() {
                    close_wsol_account = true;

                    create_token_account(
                        context,
                        &self.user,
                        &user_wsol_acc,
                        &ai.mint,
                        &self.user.pubkey(),
                        0,
                    )
                    .await
                    .unwrap();
                    user_wsol_acc.pubkey()
                } else {
                    get_associated_token_address(&self.user.pubkey(), &ai.mint)
                };

                if let Ok(None) = context.banks_client.get_account(user_mint_acc).await {
                    create_associated_token_account(
                        context,
                        &self.user,
                        &self.user.pubkey(),
                        &ai.mint,
                    )
                    .await
                }

                remaining_accounts.append(&mut vec![AccountMeta::new(user_mint_acc, false)]);
            }
        }

        set_cancel_all::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.user_state,
            remaining_accounts,
            close_wsol_account,
            &user_wsol_acc.pubkey(),
        )
        .await
        .unwrap()
    }

    pub async fn assert_bid_order(
        &self,
        in_asset: DexAsset,
        market: DexMarket,
        long: bool,
        price: f64,
        amount: f64,
        leverage: u32,
    ) {
        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let ref_us = us.borrow();

        let ai = self.dex_info.borrow().assets[in_asset as usize];

        let bid_amount = convert_to_big_number(amount, ai.decimals);
        let bid_price = convert_to_big_number(price, TEST_USDC_DECIMALS);

        let slots = ref_us.collect_market_orders(market as u8);

        for slot in slots {
            let order = ref_us.get_order(slot).assert_unwrap();
            if order.open
                && order.asset == in_asset as u8
                && order.long == long
                && order.price == bid_price
                && order.size == bid_amount
                && order.leverage == leverage
                && order.market == market as u8
            {
                return;
            }
        }

        // Not found
        assert!(false);
    }

    pub async fn assert_no_bid_order(
        &self,
        in_asset: DexAsset,
        market: DexMarket,
        long: bool,
        price: f64,
        amount: f64,
        leverage: u32,
    ) {
        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let ref_us = us.borrow();

        let ai = self.dex_info.borrow().assets[in_asset as usize];

        let bid_amount = convert_to_big_number(amount, ai.decimals);
        let bid_price = convert_to_big_number(price, TEST_USDC_DECIMALS);

        let slots = ref_us.collect_market_orders(market as u8);

        for slot in slots {
            let order = ref_us.get_order(slot).assert_unwrap();
            if order.open
                && order.asset == in_asset as u8
                && order.long == long
                && order.price == bid_price
                && order.size == bid_amount
                && order.leverage == leverage
                && order.market == market as u8
            {
                assert!(false);
            }
        }
    }

    pub async fn assert_ask_order(&self, market: DexMarket, long: bool, price: f64, size: u64) {
        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let ref_us = us.borrow();

        let ask_price = convert_to_big_number(price, TEST_USDC_DECIMALS);

        let slots = ref_us.collect_market_orders(market as u8);

        for slot in slots {
            let order = ref_us.get_order(slot).assert_unwrap();

            if !order.open && order.long == long && order.price == ask_price && order.size == size {
                return;
            }
        }

        // Not found
        assert!(false);
    }

    pub async fn assert_no_order(&self) {
        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let ref_us = us.borrow();

        let mut orders: Vec<u8> = vec![];

        for order in ref_us.order_pool.into_iter() {
            orders.push(order.index);
        }

        assert!(orders.is_empty());
    }

    pub async fn assert_order_book_bid_max_ask_min(
        &self,
        market: DexMarket,
        bid_max: f64,
        ask_min: f64,
    ) {
        let di = self.dex_info.borrow();
        let mi = di.markets[market as usize];
        let mut order_book_account = self.get_account(mi.order_book).await;
        let order_book_account_info: AccountInfo =
            (&mi.order_book, true, &mut order_book_account).into();

        let order_book = OrderBook::mount(&order_book_account_info, true).assert_unwrap();
        assert_eq!(
            order_book.bid_max_price(),
            convert_to_big_number(bid_max, TEST_USDC_DECIMALS)
        );

        assert_eq!(
            order_book.ask_min_price(),
            convert_to_big_number(ask_min, TEST_USDC_DECIMALS)
        );
    }

    pub async fn assert_no_match_event(&self) {
        let di = self.dex_info.borrow();

        let mut match_event_account = self.get_account(di.match_queue).await;
        let match_event_account_info: AccountInfo =
            (&di.match_queue, true, &mut match_event_account).into();

        let match_queue =
            SingleEventQueue::<MatchEvent>::mount(&match_event_account_info, true).assert_unwrap();

        match_queue.read_head().assert_err();
    }

    pub async fn read_match_event(&self) -> MatchEvent {
        let di = self.dex_info.borrow();

        let mut match_event_account = self.get_account(di.match_queue).await;
        let match_event_account_info: AccountInfo =
            (&di.match_queue, true, &mut match_event_account).into();

        let match_queue =
            SingleEventQueue::<MatchEvent>::mount(&match_event_account_info, true).assert_unwrap();

        let SingleEvent { data } = match_queue.read_head().assert_unwrap();

        MatchEvent { ..*data }
    }

    pub async fn market_swap(&self, in_asset: DexAsset, out_asset: DexAsset, amount: f64) {
        let aii = self.dex_info.borrow().assets[in_asset as usize];
        let aio = self.dex_info.borrow().assets[out_asset as usize];
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        set_market_swap::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.user_state,
            &aii.mint,
            &aii.oracle,
            &aii.vault,
            &aio.mint,
            &aio.oracle,
            &aio.vault,
            &aio.program_signer,
            &self.dex_info.borrow().event_queue,
            convert_to_big_number(amount, aii.decimals),
        )
        .await
        .unwrap()
    }

    pub async fn market_swap_error(&self, in_asset: DexAsset, out_asset: DexAsset, amount: f64) {
        let aii = self.dex_info.borrow().assets[in_asset as usize];
        let aio = self.dex_info.borrow().assets[out_asset as usize];
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        set_market_swap::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.user_state,
            &aii.mint,
            &aii.oracle,
            &aii.vault,
            &aio.mint,
            &aio.oracle,
            &aio.vault,
            &aio.program_signer,
            &self.dex_info.borrow().event_queue,
            convert_to_big_number(amount, aii.decimals),
        )
        .await
        .assert_err()
    }

    pub async fn assert_di_admin(&self, admin: &Pubkey) {
        let mut di_account = self.get_account(self.dex_info.borrow().di_option).await;
        let owner_key_pair = read_keypair_file("tests/fixtures/admin.json").unwrap();

        let owner = owner_key_pair.pubkey();
        let di_option_account_info: AccountInfo = (&owner, true, &mut di_account).into();
        let di = DI::mount(&di_option_account_info, true).unwrap();

        assert_eq!(di.borrow().meta.admin, *admin);
    }

    pub async fn assert_di_fee_rate(&self, fee_rate: u16) {
        let mut di_account = self.get_account(self.dex_info.borrow().di_option).await;
        let owner_key_pair = read_keypair_file("tests/fixtures/admin.json").unwrap();

        let owner = owner_key_pair.pubkey();
        let di_option_account_info: AccountInfo = (&owner, true, &mut di_account).into();
        let di = DI::mount(&di_option_account_info, true).unwrap();

        assert_eq!(di.borrow().meta.fee_rate, fee_rate);
    }

    pub async fn assert_di_options_count(&self, count: usize) {
        let mut di_account = self.get_account(self.dex_info.borrow().di_option).await;
        let owner_key_pair = read_keypair_file("tests/fixtures/admin.json").unwrap();

        let owner = owner_key_pair.pubkey();
        let di_option_account_info: AccountInfo = (&owner, true, &mut di_account).into();
        let di = DI::mount(&di_option_account_info, true).unwrap();

        assert_eq!(di.borrow().options.into_iter().count(), count);
    }

    pub async fn di_create_option(
        &self,
        id: u64,
        is_call: bool,
        base_asset: DexAsset,
        quote_asset: DexAsset,
        premium_rate: u16,
        expiry_date: i64,
        strike_price: u64,
        minimum_open_size: u64,
    ) -> DexResult {
        let bai = self.dex_info.borrow().assets[base_asset as usize];
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        if let Ok(_) = set_di_create::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.dex_info.borrow().di_option,
            &bai.oracle,
            id,
            is_call,
            base_asset as u8,
            quote_asset as u8,
            premium_rate,
            expiry_date,
            strike_price,
            minimum_open_size,
            minimum_open_size * 20,
            0,
        )
        .await
        {
            return Ok(());
        } else {
            return Err(error!(DexError::DIOptionNotFound));
        }
    }

    pub async fn di_create_btc_call(
        &self,
        id: u64,
        premium_rate: u16,
        expiry_date: i64,
        strike_price: f64,
        minimum_size: f64,
    ) {
        self.di_create_option(
            id,
            true,
            DexAsset::BTC,
            DexAsset::USDC,
            premium_rate,
            expiry_date,
            usdc(strike_price),
            btc(minimum_size),
        )
        .await
        .assert_ok()
    }

    pub async fn di_create_btc_put(
        &self,
        id: u64,
        premium_rate: u16,
        expiry_date: i64,
        strike_price: f64,
        minimum_size: f64,
    ) {
        self.di_create_option(
            id,
            false,
            DexAsset::BTC,
            DexAsset::USDC,
            premium_rate,
            expiry_date,
            usdc(strike_price),
            usdc(minimum_size),
        )
        .await
        .assert_ok()
    }

    pub async fn di_create_sol_call(
        &self,
        id: u64,
        premium_rate: u16,
        expiry_date: i64,
        strike_price: f64,
        minimum_size: f64,
    ) {
        self.di_create_option(
            id,
            true,
            DexAsset::SOL,
            DexAsset::USDC,
            premium_rate,
            expiry_date,
            usdc(strike_price),
            btc(minimum_size),
        )
        .await
        .assert_ok()
    }

    pub async fn di_create_sol_put(
        &self,
        id: u64,
        premium_rate: u16,
        expiry_date: i64,
        strike_price: f64,
        minimum_size: f64,
    ) {
        self.di_create_option(
            id,
            false,
            DexAsset::SOL,
            DexAsset::USDC,
            premium_rate,
            expiry_date,
            usdc(strike_price),
            usdc(minimum_size),
        )
        .await
        .assert_ok()
    }

    pub async fn di_set_settle_price(&self, id: u64, price: u64) -> DexResult {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        if let Ok(_) = set_di_set_settle_price::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.dex_info.borrow().di_option,
            id,
            price,
        )
        .await
        {
            return Ok(());
        } else {
            return Err(error!(DexError::DIOptionNotFound));
        }
    }

    pub async fn di_update_option(&self, id: u64, premium_rate: u16, stop: bool) -> DexResult {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        if let Ok(_) = set_di_update_option::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.dex_info.borrow().di_option,
            id,
            premium_rate,
            stop,
        )
        .await
        {
            return Ok(());
        } else {
            return Err(error!(DexError::DIOptionNotFound));
        }
    }

    pub async fn di_read_option(&self, id: u64) -> DIOption {
        let di_account = self.get_account(self.dex_info.borrow().di_option).await;
        let di = DI::mount_buf(di_account.data).unwrap();
        let di_ref = di.borrow();

        let slot = di_ref.find_option(id).unwrap();

        slot.data
    }

    pub async fn assert_di_option(
        &self,
        id: u64,
        is_call: bool,
        base_asset: DexAsset,
        quote_asset: DexAsset,
        premium_rate: u16,
        expiry_date: i64,
        strike_price: u64,
        minimum_open_size: u64,
        stopped: bool,
    ) {
        let option = self.di_read_option(id).await;

        assert_eq!(is_call, option.is_call);
        assert_eq!(base_asset as u8, option.base_asset_index);
        assert_eq!(quote_asset as u8, option.quote_asset_index);
        assert_eq!(premium_rate, option.premium_rate);
        assert_eq!(expiry_date, option.expiry_date);
        assert_eq!(strike_price, option.strike_price);
        assert_eq!(minimum_open_size, option.minimum_open_size);
        assert_eq!(stopped, option.stopped);
    }

    pub async fn assert_di_option_volume(&self, id: u64, volume: u64) {
        let option = self.di_read_option(id).await;

        assert_eq!(volume, option.volume);
    }

    pub async fn assert_di_settle_size(&self, id: u64, size: u64) {
        let option = self.di_read_option(id).await;

        assert_eq!(size, option.settle_size);
    }

    pub async fn assert_di_settle_price(&self, id: u64, price: u64) {
        let option = self.di_read_option(id).await;

        assert_eq!(price, option.settle_price);
    }

    pub async fn di_remove(&self, id: u64, force: bool) -> DexResult {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        if let Ok(_) = set_di_remove_option::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.dex_info.borrow().di_option,
            &self.dex_info.borrow().event_queue,
            id,
            force,
        )
        .await
        {
            return Ok(());
        } else {
            return Err(error!(DexError::DIOptionNotFound));
        }
    }

    pub async fn di_buy_direct(
        &self,
        id: u64,
        base_asset_index: DexAsset,
        quote_asset_index: DexAsset,
        is_call: bool,
        premium_rate: u16,
        size: u64,
    ) -> DexResult {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let bai = self.dex_info.borrow().assets[base_asset_index as usize];
        let qai = self.dex_info.borrow().assets[quote_asset_index as usize];

        let in_mint_info = if is_call { &bai } else { &qai };
        let user_state = self.user_state;
        let remaining_accounts = self.get_user_list_remaining_accounts().await;

        if let Ok(_) = set_di_buy::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.dex_info.borrow().di_option,
            &bai.oracle,
            &in_mint_info.mint,
            &in_mint_info.vault,
            &user_state,
            &self.dex_info.borrow().user_list_entry_page,
            remaining_accounts,
            id,
            premium_rate,
            size,
        )
        .await
        {
            return Ok(());
        } else {
            return Err(error!(DexError::DIOptionNotFound));
        }
    }

    pub async fn di_buy(&self, id: u64, premium_rate: u16, size: u64) -> DexResult {
        let option = self.di_read_option(id).await;
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let bai = self.dex_info.borrow().assets[option.base_asset_index as usize];
        let qai = self.dex_info.borrow().assets[option.quote_asset_index as usize];

        let in_mint_info = if option.is_call { &bai } else { &qai };
        let user_state = self.user_state;
        let remaining_accounts = self.get_user_list_remaining_accounts().await;

        if let Ok(_) = set_di_buy::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.dex_info.borrow().di_option,
            &bai.oracle,
            &in_mint_info.mint,
            &in_mint_info.vault,
            &user_state,
            &self.dex_info.borrow().user_list_entry_page,
            remaining_accounts,
            id,
            premium_rate,
            size,
        )
        .await
        {
            return Ok(());
        } else {
            return Err(error!(DexError::DIOptionNotFound));
        }
    }

    pub async fn di_settle(
        &self,
        user: &Pubkey,
        id: u64,
        force: bool,
        settle_price: u64,
    ) -> DexResult {
        let di_account = self.get_account(self.dex_info.borrow().di_option).await;
        let di = DI::mount_buf(di_account.data).unwrap();
        let di_ref = di.borrow();

        let actual_settle_price = if let Ok(slot) = di_ref.find_option(id) {
            slot.data.settle_price
        } else {
            if !force {
                return Err(error!(DexError::DIOptionNotFound));
            }
            settle_price
        };

        let options = self.di_collect_user_options(user, id).await;
        let option = if options.len() > 0 {
            options.get(0).unwrap()
        } else {
            return Err(error!(DexError::DIOptionNotFound));
        };

        let exercised = if option.is_call {
            actual_settle_price >= option.strike_price
        } else {
            actual_settle_price <= option.strike_price
        };

        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let bai = self.dex_info.borrow().assets[option.base_asset_index as usize];
        let qai = self.dex_info.borrow().assets[option.quote_asset_index as usize];

        let remaining_accounts = self.get_user_list_remaining_accounts().await;

        let (user_state, _) = Pubkey::find_program_address(
            &[&self.dex.to_bytes(), &user.to_bytes()],
            &self.program.id(),
        );

        let (mint, mint_vault, asset_program_signer) = if option.is_call {
            if exercised {
                (qai.mint, qai.vault, qai.program_signer)
            } else {
                (bai.mint, bai.vault, bai.program_signer)
            }
        } else {
            if exercised {
                (bai.mint, bai.vault, bai.program_signer)
            } else {
                (qai.mint, qai.vault, qai.program_signer)
            }
        };

        if let Ok(_) = set_di_settle::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.dex_info.borrow().di_option,
            user,
            &user_state,
            &mint,
            &qai.oracle,
            &mint_vault,
            &asset_program_signer,
            &self.dex_info.borrow().event_queue,
            &self.dex_info.borrow().user_list_entry_page,
            remaining_accounts,
            id,
            force,
            settle_price,
            true,
        )
        .await
        {
            return Ok(());
        } else {
            return Err(error!(DexError::DIOptionNotFound));
        }
    }

    pub async fn di_settle_with_invalid_user_mint_acc(
        &self,
        user: &Pubkey,
        id: u64,
        force: bool,
        settle_price: u64,
    ) -> DexResult {
        let di_account = self.get_account(self.dex_info.borrow().di_option).await;
        let di = DI::mount_buf(di_account.data).unwrap();
        let di_ref = di.borrow();

        let actual_settle_price = if let Ok(slot) = di_ref.find_option(id) {
            slot.data.settle_price
        } else {
            if !force {
                return Err(error!(DexError::DIOptionNotFound));
            }
            settle_price
        };

        let options = self.di_collect_user_options(user, id).await;
        let option = if options.len() > 0 {
            options.get(0).unwrap()
        } else {
            return Err(error!(DexError::DIOptionNotFound));
        };

        let exercised = if option.is_call {
            actual_settle_price >= option.strike_price
        } else {
            actual_settle_price <= option.strike_price
        };

        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let bai = self.dex_info.borrow().assets[option.base_asset_index as usize];
        let qai = self.dex_info.borrow().assets[option.quote_asset_index as usize];

        let remaining_accounts = self.get_user_list_remaining_accounts().await;

        let (user_state, _) = Pubkey::find_program_address(
            &[&self.dex.to_bytes(), &user.to_bytes()],
            &self.program.id(),
        );

        let (mint, mint_vault, asset_program_signer) = if option.is_call {
            if exercised {
                (qai.mint, qai.vault, qai.program_signer)
            } else {
                (bai.mint, bai.vault, bai.program_signer)
            }
        } else {
            if exercised {
                (bai.mint, bai.vault, bai.program_signer)
            } else {
                (qai.mint, qai.vault, qai.program_signer)
            }
        };

        if let Ok(_) = set_di_settle::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.dex_info.borrow().di_option,
            user,
            &user_state,
            &mint,
            &qai.oracle,
            &mint_vault,
            &asset_program_signer,
            &self.dex_info.borrow().event_queue,
            &self.dex_info.borrow().user_list_entry_page,
            remaining_accounts,
            id,
            force,
            settle_price,
            false,
        )
        .await
        {
            return Ok(());
        } else {
            return Err(error!(DexError::DIOptionNotFound));
        }
    }

    pub async fn assert_di_user_call(
        &self,
        id: u64,
        premium_rate: u16,
        size: u64,
        borrowed_base_funds: u64,
        borrowed_quote_funds: u64,
    ) {
        let raw_option = self.di_read_option(id).await;

        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let (_, option) = us.borrow().di_get_option(id, false).assert_unwrap();

        assert_eq!(option.is_call, true);
        assert_eq!(option.size, size);
        assert_eq!(option.premium_rate, premium_rate);
        assert_eq!(option.borrowed_base_funds, borrowed_base_funds);
        assert_eq!(option.borrowed_quote_funds, borrowed_quote_funds);

        assert_eq!(option.expiry_date, raw_option.expiry_date);
        assert_eq!(option.strike_price, raw_option.strike_price);
    }

    pub async fn assert_di_user_put(
        &self,
        id: u64,
        premium_rate: u16,
        size: u64,
        borrowed_base_funds: u64,
        borrowed_quote_funds: u64,
    ) {
        let raw_option = self.di_read_option(id).await;

        let mut user_state_account = self.get_account(self.user_state).await;
        let user_state_account_info: AccountInfo =
            (&self.user_state, true, &mut user_state_account).into();

        let us = UserState::mount(&user_state_account_info, true).unwrap();
        let (_, option) = us.borrow().di_get_option(id, false).assert_unwrap();

        assert_eq!(option.is_call, false);
        assert_eq!(option.size, size);
        assert_eq!(option.premium_rate, premium_rate);
        assert_eq!(option.borrowed_base_funds, borrowed_base_funds);
        assert_eq!(option.borrowed_quote_funds, borrowed_quote_funds);

        assert_eq!(option.expiry_date, raw_option.expiry_date);
        assert_eq!(option.strike_price, raw_option.strike_price);
    }

    pub async fn di_collect_user_options(
        &self,
        user: &Pubkey,
        id: u64,
    ) -> Vec<dex_program::user::UserDIOption> {
        let (user_state, _) = Pubkey::find_program_address(
            &[&self.dex.to_bytes(), &user.to_bytes()],
            &self.program.id(),
        );
        let user_state_account = self.get_account(user_state).await;

        let us = UserState::mount_buf(user_state_account.data).unwrap();
        let options = us.borrow().collect_di_option(id);

        options
    }

    pub async fn di_collect_my_options(&self, id: u64) -> Vec<dex_program::user::UserDIOption> {
        let user_state_account = self.get_account(self.user_state).await;

        let us = UserState::mount_buf(user_state_account.data).unwrap();
        let options = us.borrow().collect_di_option(id);

        options
    }

    pub async fn assert_di_user_option_count(&self, id: u64, count: usize) {
        let options = self.di_collect_my_options(id).await;

        assert_eq!(options.len(), count);
    }

    pub async fn assert_di_option_settled(&self, id: u64, exercised: bool, withdrawable: f64) {
        let options = self.di_collect_my_options(id).await;

        assert_eq!(options.len(), 1);

        assert_eq!(options[0].exercised, exercised);

        if options[0].is_call {
            if exercised {
                let ai = self.dex_info.borrow().assets[options[0].quote_asset_index as usize];
                let amount = (withdrawable * (10u64.pow(ai.decimals as u32) as f64)) as u64;
                assert_eq!(options[0].borrowed_quote_funds, amount);
            } else {
                let ai = self.dex_info.borrow().assets[options[0].base_asset_index as usize];
                let amount = (withdrawable * (10u64.pow(ai.decimals as u32) as f64)) as u64;
                assert_eq!(options[0].borrowed_base_funds, amount);
            }
        } else {
            if exercised {
                let ai = self.dex_info.borrow().assets[options[0].base_asset_index as usize];
                let amount = (withdrawable * (10u64.pow(ai.decimals as u32) as f64)) as u64;
                assert_eq!(options[0].borrowed_base_funds, amount);
            } else {
                let ai = self.dex_info.borrow().assets[options[0].quote_asset_index as usize];
                let amount = (withdrawable * (10u64.pow(ai.decimals as u32) as f64)) as u64;
                assert_eq!(options[0].borrowed_quote_funds, amount);
            }
        }
    }

    pub async fn close_mint_account(&self, asset: DexAsset) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let ai = self.dex_info.borrow().assets[asset as usize];

        let user_mint_acc = get_associated_token_address(&self.user.pubkey(), &ai.mint);

        let close_account_ix = spl_token::instruction::close_account(
            &spl_token::id(),
            &user_mint_acc,
            &self.user.pubkey(),
            &self.user.pubkey(),
            &[&self.user.pubkey()],
        )
        .unwrap();

        let instructions: Vec<Instruction> = vec![close_account_ix];

        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.user.pubkey()),
            &[&self.user],
            context.banks_client.get_latest_blockhash().await.unwrap(),
        );

        context
            .banks_client
            .process_transaction_with_preflight(transaction)
            .await
            .assert_ok();
    }

    pub async fn create_mint_account(&self, asset: DexAsset) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let ai = self.dex_info.borrow().assets[asset as usize];
        create_associated_token_account(context, &self.user, &self.user.pubkey(), &ai.mint).await
    }

    pub async fn di_withdraw_settled(&self, created: u64) -> DexResult {
        let user_state_account = self.get_account(self.user_state).await;
        let us = UserState::mount_buf(user_state_account.data).unwrap();

        let (asset_index, _) = us.borrow().di_read_created_option(created)?;
        let ai = self.dex_info.borrow().assets[asset_index as usize];

        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();
        if let Ok(_) = set_di_withdraw_settled::setup(
            context,
            &self.program,
            &self.user,
            &self.dex,
            &self.user_state,
            &ai.mint,
            &ai.vault,
            &ai.program_signer,
            created,
        )
        .await
        {
            return Ok(());
        } else {
            return Err(error!(DexError::DIOptionNotFound));
        }
    }
}
