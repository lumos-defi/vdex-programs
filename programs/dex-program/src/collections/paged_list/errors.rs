use thiserror::Error;

#[derive(Error, Debug, Clone, Eq, PartialEq)]
pub enum Error {
    #[error("Cannot borrow from account")]
    CannotBorrowFromAccount,

    #[error("Cannot initialize already in use")]
    AlreadyInUse,

    #[error("Not initialized")]
    NotInitialized,

    #[error("Page Not initialized")]
    PageNotInitialized,

    #[error("Not implemented")]
    NotImplemented,

    #[error("Invalid list header")]
    InvalidListHeader,

    #[error("Invalid next_raw")]
    InvalidNextRaw,

    #[error("Invalid index")]
    InvalidIndex,

    #[error("No free or raw slot")]
    NoFreeOrRawSlot,

    #[error("Page not chained")]
    PageNotChained,

    #[error("No pages to append")]
    NoPagesToAppend,

    #[error("Slot is in use")]
    SlotNotInUse,

    #[error("Too items in one page is 0xff")]
    TooManyItemsInOnePage,
}
