use anchor_lang::{error_code, Result};

#[error_code]
pub enum DexError {
    #[msg("Account not initialized")]
    NotInitialized,

    #[msg("Invalid mint")]
    InvalidMint,

    #[msg("Invalid oracle")]
    InvalidOracle,

    #[msg("Invalid oracle source")]
    InvalidOracleSource,

    #[msg("Invalid program signer")]
    InvalidProgramSigner,

    #[msg("Duplicate asset")]
    DuplicateAsset,

    #[msg("Insufficient asset slots")]
    InsufficientAssetSlots,

    #[msg("Invalid significant decimals")]
    InvalidSignificantDecimals,

    #[msg("Duplicate market name")]
    DuplicateMarketName,

    #[msg("Insufficient market index")]
    InsufficientMarketSlots,

    #[msg("Already in use")]
    AlreadyInUse,

    #[msg("Invalid index")]
    InvalidIndex,

    #[msg("Failed mount account")]
    FailedMountAccount,

    #[msg("Empty queue")]
    EventQueueEmpty,

    #[msg("Full queue")]
    EventQueueFull,

    #[msg("Invalid event queue")]
    InvalidEventQueue,

    #[msg("Invalid event")]
    InvalidEvent,

    #[msg("Failed serialize event ")]
    FailedSerializeEvent,

    #[msg("Failed send event header")]
    FailedSendEventHeader,

    #[msg("Failed send event")]
    FailedSendEvent,

    #[msg("Failed initialize user list")]
    FailedInitializeUserList,

    #[msg("Failed mount user list")]
    FailedMountUserList,

    #[msg("Invalid user list entry page")]
    InvalidUserListEntryPage,

    #[msg(" Failed init match queue")]
    FailedInitializeMatchQueue,

    #[msg("Small list slot in use")]
    SmallListSlotInUse,

    #[msg("Invalid list header")]
    InvalidListHeader,

    #[msg("Safe math error")]
    SafeMathError,

    #[msg("Failed mount user state")]
    FailedMountUserState,

    #[msg("Invalid vault")]
    InvalidVault,

    #[msg("Invalid Amount")]
    InvalidAmount,

    #[msg("Invalid asset index")]
    InvalidAssetIndex,

    #[msg("Invalid market index")]
    InvalidMarketIndex,

    #[msg("Invalid remaining accounts")]
    InvalidRemainingAccounts,

    #[msg("Failed new user list slot")]
    FailedNewUserListSlot,

    #[msg("Failed locate user list slot")]
    FailedLocateUserListSlot,

    #[msg("Failed load oracle account")]
    FailedLoadOracle,

    #[msg("Failed mount event queue")]
    FailedMountEventQueue,

    #[msg("Failed mount match queue")]
    FailedMountMatchQueue,

    #[msg("Failed append to event queue")]
    FailedAppendEvent,

    #[msg("Need no liquidation")]
    RequireNoLiquidation,

    #[msg("Collateral too small")]
    CollateralTooSmall,

    #[msg("Invalid position time")]
    InvalidPositionTime,

    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,

    #[msg("Insufficient SOL liquidity")]
    InsufficientSolLiquidity,

    #[msg("Insufficient borrow")]
    InsufficientBorrow,

    #[msg("Position not exist")]
    PositionNotExist,

    #[msg("Asset not exist")]
    AssetNotExist,

    #[msg("Pice greater than market price")]
    PriceGTMarketPrice,

    #[msg("Pice less than market price")]
    PriceLTMarketPrice,

    #[msg("Pice equal to market price")]
    PriceEQMarketPrice,

    #[msg("Invalid RBTree header")]
    InvalidRBTHeader,

    #[msg("No free RBTree node")]
    NoFreeRBTNode,

    #[msg("Exceed order size")]
    ExceedOrderSize,

    #[msg("No match order")]
    NoMatchOrder,

    #[msg("Invalid RBTree node")]
    InvalidRBTNode,

    #[msg("Page linked list error")]
    PageLinkedListError,

    #[msg("Zero size order")]
    ZeroSizeOrder,

    #[msg("Failed init order pool")]
    FailedInitOrderPool,

    #[msg("Failed mount order pool")]
    FailedMountOrderPool,

    #[msg("No free slot in order pool")]
    NoFreeSlotInOrderPool,

    #[msg("Invalid order slot")]
    InvalidOrderSlot,

    #[msg("Invalid match queue")]
    InvalidMatchQueue,

    #[msg("Fail to append match event")]
    FailedAppendMatchEvent,

    #[msg("Order slot mismatch")]
    OrderSlotMismatch,

    #[msg("Zero close size")]
    ZeroCloseSize,

    #[msg("AUM below zero")]
    AUMBelowZero,

    #[msg("VLP supply is zero")]
    VLPSupplyZero,

    #[msg("Invalid reward asset")]
    InvalidRewardAsset,

    #[msg("Invalid user")]
    InvalidUser,

    #[msg("Insufficient reward asset")]
    InsufficientRewardAsset,

    #[msg("Invalid user mint account")]
    InvalidUserMintAccount,

    #[msg("Invalid leverage")]
    InvalidLeverage,

    #[msg("No position size for ask order")]
    NoSizeForAskOrder,

    #[msg("DI option not exist")]
    DIOptionNotExist,

    #[msg("DI option not expired")]
    DIOptionNotExpired,

    #[msg("DI option expired")]
    DIOptionExpired,

    #[msg("DI option not found")]
    DIOptionNotFound,

    #[msg("Invalid DI option account")]
    InvalidDIOptionAccount,

    #[msg("Invalid strike price")]
    InvalidStrikePrice,

    #[msg("Premium rate is zero")]
    ZeroPremiumRate,

    #[msg("Invalid expiry date")]
    InvalidExpiryDate,

    #[msg("DI option not all settled")]
    DIOptionNotAllSettled,

    #[msg("Invalid DI admin")]
    InvalidDIAdmin,

    #[msg("Invalid DI premium")]
    DIInvalidPremium,

    #[msg("DI size too small")]
    DIInvalidSize,

    #[msg("DI option no settle price")]
    DIOptionNoSettlePrice,

    #[msg("DI option dup ID")]
    DIOptionDupID,

    #[msg("DI dup option")]
    DIOptionDup,

    #[msg("DI option not settled")]
    DIOptionNotSettled,

    #[msg("Invalid admin or delegate")]
    InvalidAdminOrDelegate,

    #[msg("Invalid price feed")]
    InvalidPriceFeed,

    #[msg("Failed append user page")]
    FailedAppendUserPage,

    #[msg("Insufficient user page slots")]
    InsufficientUserPageSlots,
}

pub type DexResult<T = ()> = Result<T>;
