#[cfg(test)]
#[allow(dead_code)]
mod list_index_test_suite {
    use crate::collections::PagedListIndex;

    #[test]
    fn test_list_index_should_be_series_successfully() {
        assert_eq!(
            PagedListIndex::new(0x12345678),
            PagedListIndex {
                page_no: 0x1234,
                offset: 0x5678
            }
        );
        assert_eq!(
            PagedListIndex {
                page_no: 0x1234,
                offset: 0x5678
            }
            .to_u32(),
            0x12345678
        );
    }

    #[test]
    fn test_list_index_compare() {
        assert!(
            PagedListIndex {
                page_no: 0x0001,
                offset: 0xffff
            } < PagedListIndex {
                page_no: 0x0002,
                offset: 0
            }
        );

        assert!(
            PagedListIndex {
                page_no: 0x0001,
                offset: 0xffff
            } == PagedListIndex {
                page_no: 0x0001,
                offset: 0xffff
            }
        );

        assert!(
            PagedListIndex {
                page_no: 0x0002,
                offset: 0x0000
            } > PagedListIndex {
                page_no: 0x0001,
                offset: 0xffff
            }
        );
        assert!(
            PagedListIndex {
                page_no: 0x0002,
                offset: 0x0002
            } > PagedListIndex {
                page_no: 0x0002,
                offset: 0x0001
            }
        );
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod paged_linked_list_test_suite {
    use anchor_lang::prelude::AccountInfo;
    use bumpalo::Bump;

    const LIST_HEAD_SIZE: u16 = std::mem::size_of::<PagedListHeader>() as u16;
    const PAGE_HEAD_SIZE: u16 = std::mem::size_of::<RemainingPageHeader>() as u16;
    const USER_SLOT_SIZE: u16 = std::mem::size_of::<PagedListSlot<UserSlot>>() as u16;

    use crate::{
        collections::{
            paged_list::{
                errors::Error, get_offset_cast, PagedList, PagedListHeader, PagedListIndex,
                RemainingPageHeader, NIL_LIST_INDEX, PAGE_NIL,
            },
            MountMode, PagedListSlot,
        },
        utils::test::gen_account,
    };

    #[derive(Clone, PartialEq, Debug, Copy)]
    struct UserSlot {
        user_id: u64,
        user_address: u64,
    }

    fn create_accounts<'a>(sizes: &'a [u16], bump: &'a Bump) -> Vec<AccountInfo<'a>> {
        sizes
            .iter()
            .map(|&size| gen_account(size as usize, bump))
            .collect::<Vec<AccountInfo>>()
    }

    fn create_list<'a>(
        accounts: &Vec<AccountInfo>,
        magic_byte: u8,
        mode: MountMode,
    ) -> PagedList<'a, UserSlot> {
        let list = PagedList::mount(&accounts[0], &accounts[1..accounts.len()], magic_byte, mode)
            .expect("List mount failed!");

        list
    }

    #[test]
    fn test_initialize_for_one_accounts() {
        let bump = Bump::new();
        let list = create_list(&create_accounts(&[100], &bump), 2, MountMode::Initialize);
        assert_eq!(list.pages.len(), 1);
        assert_eq!(
            *list.header,
            *get_offset_cast::<PagedListHeader>(0)(list.pages[0].account_ptr).unwrap()
        );
        assert_eq!(
            *list.header,
            PagedListHeader {
                magic: 0xd1c34402,
                next_raw: PagedListIndex::new(0),
                top_free: NIL_LIST_INDEX,
                last_slot: PagedListIndex {
                    page_no: 0,
                    offset: ((100 - PAGE_HEAD_SIZE) / USER_SLOT_SIZE) as u16
                },
                first_page_total_raw: ((100 - LIST_HEAD_SIZE) / USER_SLOT_SIZE) as u16,
                next_page: PAGE_NIL,
                head: NIL_LIST_INDEX,
                tail: NIL_LIST_INDEX
            }
        );
    }

    #[test]
    fn test_mount_buf_for_one_accounts() {
        let bump = Bump::new();
        let accounts = create_accounts(&[100], &bump);
        create_list(&accounts, 2, MountMode::Initialize);
        let first_buf = accounts[0].data.borrow().as_ref().to_vec();
        let list = PagedList::<UserSlot>::mount_buf(first_buf, vec![], 2).unwrap();

        assert_eq!(list.pages.len(), 1);
        assert_eq!(
            *list.header,
            *get_offset_cast::<PagedListHeader>(0)(list.pages[0].account_ptr).unwrap()
        );
        assert_eq!(
            *list.header,
            PagedListHeader {
                magic: 0xd1c34402,
                next_raw: PagedListIndex::new(0),
                top_free: NIL_LIST_INDEX,
                last_slot: PagedListIndex {
                    page_no: 0,
                    offset: ((100 - PAGE_HEAD_SIZE) / USER_SLOT_SIZE) as u16
                },
                first_page_total_raw: ((100 - LIST_HEAD_SIZE) / USER_SLOT_SIZE) as u16,
                next_page: PAGE_NIL,
                head: NIL_LIST_INDEX,
                tail: NIL_LIST_INDEX
            }
        );
    }

    #[test]
    fn test_initialize_for_two_accounts() {
        let bump = Bump::new();
        let accounts = create_accounts(&[100, 100], &bump);
        let list = create_list(&accounts, 2, MountMode::Initialize);
        assert_eq!(list.pages.len(), 2);
        assert_eq!(
            *list.header,
            *get_offset_cast::<PagedListHeader>(0)(list.pages[0].account_ptr).unwrap()
        );
        assert_eq!(
            *list.header,
            PagedListHeader {
                magic: 0xd1c34402,
                next_raw: PagedListIndex::new(0),
                top_free: NIL_LIST_INDEX,
                last_slot: PagedListIndex {
                    page_no: 1,
                    offset: (100 - PAGE_HEAD_SIZE) / USER_SLOT_SIZE
                },
                first_page_total_raw: (100 - LIST_HEAD_SIZE) / USER_SLOT_SIZE,
                next_page: *accounts[1].key,
                head: NIL_LIST_INDEX,
                tail: NIL_LIST_INDEX
            }
        );
        assert_eq!(
            *get_offset_cast::<RemainingPageHeader>(0)(list.pages[1].account_ptr).unwrap(),
            RemainingPageHeader {
                magic: 0xd1c34402,
                page_no: 1,
                total_raw: (100 - PAGE_HEAD_SIZE) / USER_SLOT_SIZE,
                next_page: PAGE_NIL
            }
        );
    }

    #[test]
    fn test_mount_bufs_for_two_accounts() {
        let bump = Bump::new();
        let accounts = create_accounts(&[100, 100], &bump);
        create_list(&accounts, 2, MountMode::Initialize);
        let first_buf = accounts[0].data.borrow().as_ref().to_vec();
        let remaining_bufs = vec![accounts[1].data.borrow().as_ref().to_vec()];
        let list = PagedList::<UserSlot>::mount_buf(first_buf, remaining_bufs, 2).unwrap();

        assert_eq!(list.pages.len(), 2);
        assert_eq!(
            *list.header,
            *get_offset_cast::<PagedListHeader>(0)(list.pages[0].account_ptr).unwrap()
        );
        assert_eq!(
            *list.header,
            PagedListHeader {
                magic: 0xd1c34402,
                next_raw: PagedListIndex::new(0),
                top_free: NIL_LIST_INDEX,
                last_slot: PagedListIndex {
                    page_no: 1,
                    offset: (100 - PAGE_HEAD_SIZE) / USER_SLOT_SIZE
                },
                first_page_total_raw: (100 - LIST_HEAD_SIZE) / USER_SLOT_SIZE,
                next_page: *accounts[1].key,
                head: NIL_LIST_INDEX,
                tail: NIL_LIST_INDEX
            }
        );
        assert_eq!(
            *get_offset_cast::<RemainingPageHeader>(0)(list.pages[1].account_ptr).unwrap(),
            RemainingPageHeader {
                magic: 0xd1c34402,
                page_no: 1,
                total_raw: (100 - PAGE_HEAD_SIZE) / USER_SLOT_SIZE,
                next_page: PAGE_NIL
            }
        );
    }

    #[test]
    fn test_new_slot_and_fetch() {
        let bump = Bump::new();
        let accounts = create_accounts(
            &[
                LIST_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE,
            ],
            &bump,
        );
        let list = create_list(&accounts, 2, MountMode::Initialize);

        let slot = list.new_slot().expect("when new slot");
        assert_eq!(
            *list.header,
            PagedListHeader {
                magic: 0xd1c34402,
                next_raw: PagedListIndex::new(0x00010000),
                top_free: NIL_LIST_INDEX,
                last_slot: PagedListIndex {
                    page_no: 1,
                    offset: (100 - PAGE_HEAD_SIZE) / USER_SLOT_SIZE
                },
                first_page_total_raw: (100 - LIST_HEAD_SIZE) / USER_SLOT_SIZE,
                next_page: *accounts[1].key,
                head: PagedListIndex::new(0),
                tail: PagedListIndex::new(0)
            }
        );

        slot.data = UserSlot {
            user_address: 3,
            user_id: 1,
        };

        let fetched_slot = list.from_index(slot.index()).expect("fetch failed");

        assert_eq!(
            *fetched_slot,
            PagedListSlot {
                data: UserSlot {
                    user_address: 3,
                    user_id: 1,
                },
                index: PagedListIndex::new(0),
                is_in_use: true,
                next: NIL_LIST_INDEX,
                prev: NIL_LIST_INDEX,
                padding: [0; 3]
            }
        );
    }

    #[test]
    fn test_iterator_new_3() {
        let bump = Bump::new();
        let accounts = create_accounts(
            &[
                LIST_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE * 2,
            ],
            &bump,
        );
        let list = create_list(&accounts, 2, MountMode::Initialize);

        for i in 0..3u64 {
            let slot = list.new_slot().expect("when new slot");
            slot.data = UserSlot {
                user_address: 3,
                user_id: i,
            };
        }

        assert_eq!(
            list.into_iter()
                .map(|x| x.data.user_id)
                .collect::<Vec<u64>>(),
            vec![0, 1, 2]
        )
    }

    #[test]
    fn test_iterator_new_3_release_head() {
        let bump = Bump::new();
        let accounts = create_accounts(
            &[
                LIST_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE * 2,
            ],
            &bump,
        );
        let list = create_list(&accounts, 2, MountMode::Initialize);

        for i in 0..3u64 {
            let slot = list.new_slot().expect("when new slot");
            slot.data = UserSlot {
                user_address: 3,
                user_id: i,
            };
        }
        list.release_slot(0).unwrap();

        assert_eq!(
            list.into_iter()
                .map(|x| x.data.user_id)
                .collect::<Vec<u64>>(),
            vec![1, 2]
        )
    }

    #[test]
    fn test_iterator_new_3_release_tail() {
        let bump = Bump::new();
        let accounts = create_accounts(
            &[
                LIST_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE * 2,
            ],
            &bump,
        );
        let list = create_list(&accounts, 2, MountMode::Initialize);

        for i in 0..3u64 {
            let slot = list.new_slot().expect("when new slot");
            slot.data = UserSlot {
                user_address: 3,
                user_id: i,
            };
        }
        list.release_slot(list.header.tail.to_u32()).unwrap();

        assert_eq!(
            list.into_iter()
                .map(|x| x.data.user_id)
                .collect::<Vec<u64>>(),
            vec![0, 1]
        )
    }

    #[test]
    fn test_iterator_new_1_release_1() {
        let bump = Bump::new();
        let accounts = create_accounts(
            &[
                LIST_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE * 2,
            ],
            &bump,
        );
        let list = create_list(&accounts, 2, MountMode::Initialize);

        let slot = list.new_slot().expect("when new slot");
        slot.data = UserSlot {
            user_address: 3,
            user_id: 1,
        };
        list.release_slot(0).unwrap();

        assert_eq!(
            list.into_iter()
                .map(|x| x.data.user_id)
                .collect::<Vec<u64>>(),
            vec![1, 0]
        )
    }

    #[test]
    fn test_new_slot_and_fetch_across_page() {
        let bump = Bump::new();
        let accounts = create_accounts(
            &[
                LIST_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE,
            ],
            &bump,
        );
        let list = create_list(&accounts, 2, MountMode::Initialize);
        {
            let slot = list.new_slot().expect("when new 1st slot");
            assert_eq!(
                *list.header,
                PagedListHeader {
                    magic: 0xd1c34402,
                    next_raw: PagedListIndex::new(0x00010000),
                    top_free: NIL_LIST_INDEX,
                    last_slot: PagedListIndex {
                        page_no: 1,
                        offset: 1
                    },
                    first_page_total_raw: 1,
                    next_page: *accounts[1].key,
                    head: PagedListIndex::new(0),
                    tail: PagedListIndex::new(0)
                }
            );
            assert_eq!(
                *slot,
                PagedListSlot {
                    data: UserSlot {
                        user_address: 0,
                        user_id: 0,
                    },
                    index: PagedListIndex::new(0),
                    is_in_use: true,
                    next: NIL_LIST_INDEX,
                    prev: NIL_LIST_INDEX,
                    padding: [0; 3]
                }
            );
        }

        {
            let slot = list.new_slot().expect("when get 2nd page");

            slot.data = UserSlot {
                user_address: 3,
                user_id: 1,
            };

            let fetched_slot = list.from_index(slot.index()).expect("fetch failed");

            assert_eq!(
                *fetched_slot,
                PagedListSlot {
                    data: UserSlot {
                        user_address: 3,
                        user_id: 1,
                    },
                    index: PagedListIndex {
                        page_no: 1,
                        offset: 0
                    },
                    is_in_use: true,
                    next: NIL_LIST_INDEX,
                    prev: PagedListIndex {
                        page_no: 0,
                        offset: 0
                    },
                    padding: [0; 3]
                }
            );
        }
    }

    #[test]
    #[should_panic(expected = "when get 3nd page: NoFreeOrRawSlot")]
    fn test_should_fail_if_no_more_raw() {
        let bump = Bump::new();
        let list = create_list(
            &create_accounts(
                &[
                    LIST_HEAD_SIZE + USER_SLOT_SIZE,
                    PAGE_HEAD_SIZE + USER_SLOT_SIZE,
                ],
                &bump,
            ),
            2,
            MountMode::Initialize,
        );
        list.new_slot().expect("when new slot");
        list.new_slot().expect("when new slot");
        list.new_slot().expect("when get 3nd page");
    }

    #[test]
    fn test_should_allow_to_new_if_release() {
        let bump = Bump::new();
        let accounts = create_accounts(
            &[
                LIST_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE,
            ],
            &bump,
        );
        let list = create_list(&accounts, 2, MountMode::Initialize);
        list.new_slot().expect("when new 1st slot");
        {
            let slot = list.new_slot().expect("when new 2nd slot");
            list.release_slot(slot.index()).expect("when release slot");
            assert_eq!(
                *list.header,
                PagedListHeader {
                    magic: 0xd1c34402,
                    next_raw: PagedListIndex {
                        page_no: 2,
                        offset: 0
                    },
                    top_free: PagedListIndex {
                        page_no: 1,
                        offset: 0
                    },
                    last_slot: PagedListIndex {
                        page_no: 1,
                        offset: 1
                    },
                    first_page_total_raw: 1,
                    next_page: *accounts[1].key,
                    head: PagedListIndex {
                        page_no: 0,
                        offset: 0
                    },
                    tail: PagedListIndex {
                        page_no: 0,
                        offset: 0
                    },
                }
            );
        }
        let slot = list.new_slot().expect("when new 3nd slot");
        assert_eq!(
            *list.header,
            PagedListHeader {
                magic: 0xd1c34402,
                next_raw: PagedListIndex {
                    page_no: 2,
                    offset: 0
                },
                top_free: NIL_LIST_INDEX,
                last_slot: PagedListIndex {
                    page_no: 1,
                    offset: 1
                },
                first_page_total_raw: 1,
                next_page: *accounts[1].key,
                head: PagedListIndex::new(0),
                tail: slot.index,
            }
        );
    }

    #[test]
    #[should_panic(expected = "when get 4th page: NoFreeOrRawSlot")]
    fn test_should_throw_if_used_free_up() {
        let bump = Bump::new();
        let list = create_list(
            &create_accounts(
                &[
                    LIST_HEAD_SIZE + USER_SLOT_SIZE,
                    PAGE_HEAD_SIZE + USER_SLOT_SIZE,
                ],
                &bump,
            ),
            2,
            MountMode::Initialize,
        );
        list.new_slot().expect("when new 1st slot");
        {
            let slot = list.new_slot().expect("when new 2nd slot");
            list.release_slot(slot.index())
                .expect("when releasing 2nd slot");
        }
        list.new_slot().expect("when get 3nd page");
        list.new_slot().expect("when get 4th page");
    }

    #[test]
    fn test_should_mount_read_write() {
        let bump = Bump::new();
        let accounts = create_accounts(
            &[
                LIST_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE,
            ],
            &bump,
        );
        create_list(&accounts, 2, MountMode::Initialize)
            .new_slot()
            .expect("when new 1st slot");
        let list = create_list(&accounts, 2, MountMode::ReadWrite);
        list.new_slot()
            .and_then(|slot| list.release_slot(slot.index()))
            .expect("when new 2nd slot or release 2nd slot");
        list.new_slot().expect("when get 3nd page");
    }

    #[test]
    #[should_panic(expected = "List mount failed!: PageNotChained")]
    fn test_should_error_with_wrong_next_page() {
        let bump = Bump::new();
        let accounts = create_accounts(
            &[
                LIST_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE,
            ],
            &bump,
        );
        create_list(&accounts, 2, MountMode::Initialize);
        let fake_account = create_accounts(&[PAGE_HEAD_SIZE + USER_SLOT_SIZE], &bump);
        create_list(
            &Vec::from([accounts[0].clone(), fake_account[0].clone()]),
            2,
            MountMode::ReadWrite,
        );
    }

    #[test]
    fn test_append_success() {
        let bump = Bump::new();
        let accounts = create_accounts(
            &[
                LIST_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE,
            ],
            &bump,
        );
        {
            let list = create_list(&accounts, 2, MountMode::Initialize);
            list.new_slot().expect("when 1st new slot");
            list.new_slot().expect("when 2nd new slot");
        }

        let new_accounts = create_accounts(
            &[
                PAGE_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE,
            ],
            &bump,
        );

        let list = PagedList::<UserSlot>::append_pages(
            accounts.first().unwrap(),
            &accounts[1..],
            &new_accounts[..],
            2,
        )
        .expect("when append pages");

        list.new_slot().expect("when 3rd new slot");
        list.new_slot().expect("when 4th new slot");
    }

    #[test]
    fn test_should_new_slot_fail_after_new_1000_slot_release_1000_slot_append_1000_slot_new_2000_slot(
    ) {
        let bump = Bump::new();
        let mut sizes = vec![LIST_HEAD_SIZE + USER_SLOT_SIZE * 100];
        {
            sizes.append(&mut vec![PAGE_HEAD_SIZE + USER_SLOT_SIZE * 100; 9]);
        }
        let accounts = create_accounts(&sizes, &bump);
        {
            let list = create_list(&accounts, 2, MountMode::Initialize);

            for i in 0..1000 {
                list.new_slot()
                    .expect(&format!("when new {} slot", i).to_owned());
            }
            match list.new_slot() {
                Ok(_) => panic!("should failed"),
                Err(_) => {}
            }
            for i in 0..1000 {
                list.release_slot(
                    PagedListIndex {
                        page_no: i / 100,
                        offset: i % 100,
                    }
                    .to_u32(),
                )
                .expect(&format!("when release {} slot", i).to_owned());
            }
        }
        let new_accounts = create_accounts(&[PAGE_HEAD_SIZE + USER_SLOT_SIZE * 100; 10], &bump);

        let list =
            PagedList::<UserSlot>::append_pages(&accounts[0], &accounts[1..], &new_accounts[..], 2)
                .expect("when appending");
        println!("xxxxxxx {:?}", list.header);
        println!("xxxxxxx1 {:?}", list.pages);

        for i in 0..1000 {
            list.new_slot()
                .expect(&format!("when new {} slot again", i).to_owned());
        }
        for i in 1000..2000 {
            list.new_slot()
                .expect(&format!("when new {} slot again", i).to_owned());
        }
        match list.new_slot() {
            Ok(_) => panic!("should failed"),
            Err(_) => {}
        }
    }

    #[test]
    fn test_should_fail_free_slot_not_in_use() {
        let bump = Bump::new();
        let accounts = create_accounts(
            &[
                LIST_HEAD_SIZE + USER_SLOT_SIZE,
                PAGE_HEAD_SIZE + USER_SLOT_SIZE,
            ],
            &bump,
        );
        let list = create_list(&accounts, 2, MountMode::Initialize);
        let slot = list.new_slot().expect("when 1st new slot");
        list.release_slot(slot.index())
            .expect("when release 1st time");
        match list.release_slot(slot.index()) {
            Ok(_) => panic!("should fail"),
            Err(err) => assert_eq!(err, Error::SlotNotInUse),
        }
    }
}
