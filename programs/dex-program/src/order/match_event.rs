use crate::{collections::SingleEventQueue, errors::DexResult};

pub struct MatchEvent {
    pub user_state: [u8; 32],
    pub order_slot: u32,
    pub user_order_slot: u8,
    _padding: [u8; 3],
}

pub trait AppendSingleEvent {
    fn append(&mut self, user_state: &[u8; 32], order_slot: u32, user_order_slot: u8) -> DexResult;
}

impl AppendSingleEvent for SingleEventQueue<'_, MatchEvent> {
    fn append(&mut self, user_state: &[u8; 32], order_slot: u32, user_order_slot: u8) -> DexResult {
        let new_event = self.new_tail()?;
        new_event.data.user_state.copy_from_slice(&user_state[..]);

        new_event.data.order_slot = order_slot;
        new_event.data.user_order_slot = user_order_slot;

        Ok(())
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use std::str::FromStr;

    use super::*;
    use crate::{collections::SingleEvent, utils::test::*};
    use anchor_lang::prelude::Pubkey;
    use bumpalo::Bump;

    fn random_pubkey(i: usize) -> Pubkey {
        let key_table = [
            "5hHnY9LS5ejhjN28VJxGQjXoZKr4Ncj99YpNQTZ8sSRN",
            "Cnmd31wVZfpagWiDWWDgwzB1rAwV64gCUJyHjYgLSbje",
            "EvKGpxdnAj3e8Z1DQwCGRByxwBUUisYSvWQ2fVef22JL",
            "B5Uh5Sz8tUkg6zwKMAdpynosEh5RYssYvhcWQrZyZ8Cb",
            "FLaejoErypVTbQwQtMiYiYdRWATFw1hjkoESRgBbKbDk",
        ];

        Pubkey::from_str(key_table[i % 5]).assert_unwrap()
    }

    fn append_event(q: &mut SingleEventQueue<MatchEvent>, i: usize) {
        q.append(&random_pubkey(i).to_bytes(), i as u32, i as u8)
            .assert_ok();
    }

    fn assert_event(event: &MatchEvent, i: usize) {
        assert_eq!(event.user_state, random_pubkey(i).to_bytes());
        assert_eq!(event.order_slot, i as u32);
        assert_eq!(event.user_order_slot, i as u8);
    }

    #[test]
    fn test_post_match_queue_one_by_one() {
        let bump = Bump::new();
        let account_size = 64 * 1024;
        let account = gen_account(account_size, &bump);

        let mut q = SingleEventQueue::<MatchEvent>::mount(&account, false).assert_unwrap();
        q.initialize().assert_ok();

        let max_events = q.header_ref().assert_unwrap().total_raw as usize - 1;

        for i in 0..max_events * 20 {
            // Append one event
            append_event(&mut q, i);

            // Read event
            let SingleEvent { data } = q.read_head().assert_unwrap();

            // Check event
            assert_event(data, i);

            // Crank
            q.remove_head().assert_ok();
            let header = q.header_ref().assert_unwrap();
            assert_eq!(header.head, header.tail);

            // No more events
            q.read_head().assert_err();
            q.remove_head().assert_err();
        }

        q.read_head().assert_err();
        q.remove_head().assert_err();
    }

    #[test]
    fn test_post_match_queue_batch() {
        let bump = Bump::new();
        let account_size = 64 * 1024;
        let account = gen_account(account_size, &bump);

        let mut q = SingleEventQueue::<MatchEvent>::mount(&account, false).assert_unwrap();
        q.initialize().assert_ok();

        let max_events = q.header_ref().assert_unwrap().total_raw as usize - 1;

        for _ in 0..20 {
            for i in 0..max_events / 3 {
                // Append one event
                append_event(&mut q, i);
            }

            for i in 0..max_events / 3 {
                // Read event
                let SingleEvent { data } = q.read_head().assert_unwrap();

                // Check event
                assert_event(data, i);

                // Crank
                q.remove_head().assert_ok();
            }

            let header = q.header_ref().assert_unwrap();
            assert_eq!(header.head, header.tail);

            // No more events
            q.read_head().assert_err();
            q.remove_head().assert_err();
        }
    }
}
