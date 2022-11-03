use anchor_lang::{error_code, Result};

#[error_code]
pub enum DexError {
    #[msg("Not initialized")]
    NotInitialized,

    #[msg("Invalid mint")]
    InvalidMint,

    #[msg("Invalid oracle source")]
    InvalidOracleSource,

    #[msg("Invalid program signer")]
    InvalidProgramSigner,

    #[msg("Duplicate asset")]
    DuplicateAsset,

    #[msg("Insufficient asset index")]
    InsufficientAssetIndex,

    #[msg("Invalid significant decimals")]
    InvalidSignificantDecimals,

    #[msg("Duplicate market name")]
    DuplicateMarketName,

    #[msg("Insufficient market index")]
    InsufficientMarketIndex,

    #[msg("Already in use")]
    AlreadyInUse,

    #[msg("Invalid index")]
    InvalidIndex,

    #[msg("Mount Account failed")]
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

    #[msg(" Failed init match queue")]
    FailedInitMatchQueue,

    #[msg("Small list slot in use")]
    SmallListSlotInUse,

    #[msg("Invalid list header")]
    InvalidListHeader,

    #[msg("Safe math error")]
    SafeMathError,

    #[msg("Failed mount user state")]
    FailedMountUserState,

    #[msg("Invalid PDA")]
    InvalidPDA,

    #[msg("Invalid vault")]
    InvalidVault,

    #[msg("Invalid Amount")]
    InvalidAmount,

    #[msg("Invalid market index")]
    InvalidMarketIndex,

    #[msg("Invalid remaining accounts")]
    InvalidRemainingAccounts,

    #[msg("Open size too small")]
    OpenSizeTooSmall,

    #[msg("Failed new user list slot")]
    FailedNewUserListSlot,

    #[msg("Failed locate user list slot")]
    FailedLocateUserListSlot,

    #[msg("Failed load oracle account")]
    FailedLoadOracle,
}

pub type DexResult<T = ()> = Result<T>;
