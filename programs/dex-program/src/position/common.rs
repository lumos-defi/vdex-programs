use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefMut,
};

use anchor_lang::prelude::*;

use crate::{
    collections::PagedList,
    dex::UserListItem,
    errors::{DexError, DexResult},
    user::UserState,
    utils::NIL32,
};

pub fn update_user_serial_number(
    user_list: &PagedList<UserListItem>,
    mut user: RefMut<UserState>,
    user_pubkey: Pubkey,
) -> DexResult {
    user.borrow_mut().inc_serial_number();

    if user.release_user_list_slot() {
        user_list
            .release_slot(user.borrow().meta.user_list_index)
            .map_err(|e| {
                msg!("{}", e.to_string());
                DexError::FailedReleaseUserListSlot
            })?;

        return Ok(());
    }

    let serial_number = user.borrow().serial_number();
    if user.borrow().user_list_index() == NIL32 {
        let slot = user_list
            .new_slot()
            .map_err(|_| DexError::FailedNewUserListSlot)?;
        slot.data
            .init_serial_number(user_pubkey.to_bytes(), serial_number);

        user.borrow_mut().set_user_list_index(slot.index());
    } else {
        let slot = user_list
            .from_index(user.borrow().user_list_index())
            .map_err(|_| DexError::FailedLocateUserListSlot)?;
        slot.data.update_serial_number(serial_number);
    }

    Ok(())
}
