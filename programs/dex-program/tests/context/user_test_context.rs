use std::{
    cell::RefCell,
    ops::{Div, Mul},
    rc::Rc,
};

use crate::utils::{
    convert_to_big_number, convert_to_big_number_i, create_associated_token_account, get_dex_info,
    get_keypair, get_program, get_token_balance, mint_tokens, set_add_liquidity,
    set_feed_mock_oracle, set_user_state, transfer, DexAsset, DexMarket,
};
use anchor_client::{
    solana_sdk::{account::Account, signature::Keypair, signer::Signer},
    Program,
};
use anchor_lang::{
    prelude::{AccountMeta, Pubkey},
    AccountDeserialize,
};

use anchor_spl::token::Mint;
use dex_program::{
    dex::{Dex, MockOracle},
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
        self.add_liquidity(DexAsset::USDC as u8, amount).await;
    }

    pub async fn add_liquidity_with_btc(&self, amount: f64) {
        self.add_liquidity(DexAsset::BTC as u8, amount).await;
    }

    pub async fn add_liquidity_with_eth(&self, amount: f64) {
        self.add_liquidity(DexAsset::ETH as u8, amount).await;
    }

    pub async fn add_liquidity_with_sol(&self, amount: f64) {
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
            &self.admin,
            &self.user,
            &self.dex,
            &asset_info.mint,
            &asset_info.vault,
            &asset_info.program_signer,
            &self.dex_info.borrow().vlp_mint,
            &self.dex_info.borrow().vlp_mint_authority,
            deposit_amount,
            remaining_accounts,
        )
        .await
        .unwrap();
    }

    pub async fn assert_usdc_amount(&self, user_mint_acc: &Pubkey, amount: f64) {
        self.assert_asset_amount(user_mint_acc, DexAsset::USDC as usize, amount)
            .await;
    }

    pub async fn assert_btc_amount(&self, user_mint_acc: &Pubkey, amount: f64) {
        self.assert_asset_amount(user_mint_acc, DexAsset::BTC as usize, amount)
            .await;
    }

    pub async fn assert_eth_amount(&self, user_mint_acc: &Pubkey, amount: f64) {
        self.assert_asset_amount(user_mint_acc, DexAsset::ETH as usize, amount)
            .await;
    }

    pub async fn assert_sol_amount(&self, user_mint_acc: &Pubkey, amount: f64) {
        self.assert_asset_amount(user_mint_acc, DexAsset::SOL as usize, amount)
            .await;
    }

    pub async fn assert_asset_amount(&self, user_mint_acc: &Pubkey, asset: usize, amount: f64) {
        let asset_info = self.dex_info.borrow().assets[asset];
        let asset_amount =
            get_token_balance(&mut self.context.borrow_mut().banks_client, user_mint_acc).await;

        assert_eq!(
            asset_amount,
            convert_to_big_number(amount.into(), asset_info.decimals)
        );
    }

    pub async fn assert_vlp_amount(&self, user_mint_acc: &Pubkey, amount: f64) {
        let vlp_account = self.get_account(self.dex_info.borrow().vlp_mint).await;
        let vlp_mint_info = Mint::try_deserialize(&mut vlp_account.data.as_ref()).unwrap();
        let asset_amount =
            get_token_balance(&mut self.context.borrow_mut().banks_client, user_mint_acc).await;

        assert_eq!(
            asset_amount,
            convert_to_big_number(amount.into(), vlp_mint_info.decimals)
        );
    }

    pub async fn get_user_vlp_token_pubkey(&self) -> Pubkey {
        let user_mint_acc =
            get_associated_token_address(&self.user.pubkey(), &self.dex_info.borrow().vlp_mint);
        user_mint_acc
    }

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
}
