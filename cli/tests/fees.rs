// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use acuity_runtime::{
    constants::{currency::*, time::SLOT_DURATION},
    Balances, CheckedExtrinsic, Multiplier, Runtime, RuntimeCall, TransactionByteFee,
    TransactionPayment,
};
use codec::{Encode, Joiner};
use frame_support::{
    dispatch::GetDispatchInfo,
    traits::Currency,
    weights::{constants::ExtrinsicBaseWeight, IdentityFee, WeightToFee},
};
use node_primitives::Balance;
use node_testing::keyring::*;
use sp_runtime::{traits::One, Perbill};

pub mod common;
use self::common::{sign, *};

fn new_account_info(free_dollars: u128) -> Vec<u8> {
    frame_system::AccountInfo {
        nonce: 0u32,
        consumers: 0,
        providers: 1,
        sufficients: 0,
        data: (
            free_dollars * DOLLARS,
            0 * DOLLARS,
            0 * DOLLARS,
            1u128 << 127,
        ),
    }
    .encode()
}

#[test]
fn transaction_fee_is_correct() {
    // This uses the exact values of substrate-node.
    //
    // weight of transfer call as of now: 1_000_000
    // if weight of the cheapest weight would be 10^7, this would be 10^9, which is:
    //   - 1 MILLICENTS in substrate node.
    //   - 1 milli-dot based on current polkadot runtime.
    // (this based on assigning 0.1 CENT to the cheapest tx with `weight = 100`)
    let mut t = new_test_ext(compact_code_unwrap());
    t.insert(
        <frame_system::Account<Runtime>>::hashed_key_for(alice()),
        new_account_info(100),
    );
    t.insert(
        <frame_system::Account<Runtime>>::hashed_key_for(bob()),
        new_account_info(10),
    );
    t.insert(
        <pallet_balances::TotalIssuance<Runtime>>::hashed_key().to_vec(),
        (110 * DOLLARS).encode(),
    );
    t.insert(
        <frame_system::BlockHash<Runtime>>::hashed_key_for(0),
        vec![0u8; 32],
    );

    let tip = 1_000_000;
    let xt = sign(CheckedExtrinsic {
        signed: Some((alice(), signed_extra(0, tip))),
        function: RuntimeCall::Balances(default_transfer_call()),
    });

    let r = executor_call(
        &mut t,
        "Core_initialize_block",
        &vec![].and(&from_block_number(1u32)),
    )
    .0;

    assert!(r.is_ok());
    let r = executor_call(
        &mut t,
        "BlockBuilder_apply_extrinsic",
        &vec![].and(&xt.clone()),
    )
    .0;
    assert!(r.is_ok());

    t.execute_with(|| {
        assert_eq!(Balances::total_balance(&bob()), (10 + 69) * DOLLARS);
        // Components deducted from alice's balances:
        // - Base fee
        // - Weight fee
        // - Length fee
        // - Tip
        // - Creation-fee of bob's account.
        let mut balance_alice = (100 - 69) * DOLLARS;

        let base_weight = ExtrinsicBaseWeight::get();
        let base_fee = IdentityFee::<Balance>::weight_to_fee(&base_weight);

        let length_fee = TransactionByteFee::get() * (xt.clone().encode().len() as Balance);
        balance_alice -= length_fee;

        let weight = default_transfer_call().get_dispatch_info().weight;
        let weight_fee = IdentityFee::<Balance>::weight_to_fee(&weight);

        // we know that weight to fee multiplier is effect-less in block 1.
        // current weight of transfer = 200_000_000
        // Linear weight to fee is 1:1 right now (1 weight = 1 unit of balance)
        assert_eq!(weight_fee, weight.ref_time() as Balance);
        balance_alice -= base_fee;
        balance_alice -= weight_fee;
        balance_alice -= tip;

        assert_eq!(Balances::total_balance(&alice()), balance_alice);
    });
}

#[test]
#[should_panic]
#[cfg(feature = "stress-test")]
fn block_weight_capacity_report() {
    // Just report how many transfer calls you could fit into a block. The number should at least
    // be a few hundred (250 at the time of writing but can change over time). Runs until panic.
    use node_primitives::Nonce;

    // execution ext.
    let mut t = new_test_ext(compact_code_unwrap());
    // setup ext.
    let mut tt = new_test_ext(compact_code_unwrap());

    let factor = 50;
    let mut time = 10;
    let mut nonce: Nonce = 0;
    let mut block_number = 1;
    let mut previous_hash: node_primitives::Hash = GENESIS_HASH.into();

    loop {
        let num_transfers = block_number * factor;
        let mut xts = (0..num_transfers)
            .map(|i| CheckedExtrinsic {
                signed: Some((charlie(), signed_extra(nonce + i as Nonce, 0))),
                function: RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
                    dest: bob().into(),
                    value: 0,
                }),
            })
            .collect::<Vec<CheckedExtrinsic>>();

        xts.insert(
            0,
            CheckedExtrinsic {
                signed: None,
                function: RuntimeCall::Timestamp(pallet_timestamp::Call::set { now: time * 1000 }),
            },
        );

        // NOTE: this is super slow. Can probably be improved.
        let block = construct_block(
            &mut tt,
            block_number,
            previous_hash,
            xts,
            (time * 1000 / SLOT_DURATION).into(),
        );

        let len = block.0.len();
        print!(
            "++ Executing block with {} transfers. Block size = {} bytes / {} kb / {} mb",
            num_transfers,
            len,
            len / 1024,
            len / 1024 / 1024,
        );

        let r = executor_call(&mut t, "Core_execute_block", &block.0).0;

        println!(" || Result = {:?}", r);
        assert!(r.is_ok());

        previous_hash = block.1;
        nonce += num_transfers;
        time += 10;
        block_number += 1;
    }
}

#[test]
#[should_panic]
#[cfg(feature = "stress-test")]
fn block_length_capacity_report() {
    // Just report how big a block can get. Executes until panic. Should be ignored unless if
    // manually inspected. The number should at least be a few megabytes (5 at the time of
    // writing but can change over time).
    use node_primitives::Nonce;

    // execution ext.
    let mut t = new_test_ext(compact_code_unwrap());
    // setup ext.
    let mut tt = new_test_ext(compact_code_unwrap());

    let factor = 256 * 1024;
    let mut time = 10;
    let mut nonce: Nonce = 0;
    let mut block_number = 1;
    let mut previous_hash: node_primitives::Hash = GENESIS_HASH.into();

    loop {
        // NOTE: this is super slow. Can probably be improved.
        let block = construct_block(
            &mut tt,
            block_number,
            previous_hash,
            vec![
                CheckedExtrinsic {
                    signed: None,
                    function: RuntimeCall::Timestamp(pallet_timestamp::Call::set {
                        now: time * 1000,
                    }),
                },
                CheckedExtrinsic {
                    signed: Some((charlie(), signed_extra(nonce, 0))),
                    function: RuntimeCall::System(frame_system::Call::remark {
                        remark: vec![0u8; (block_number * factor) as usize],
                    }),
                },
            ],
            (time * 1000 / SLOT_DURATION).into(),
        );

        let len = block.0.len();
        print!(
            "++ Executing block with big remark. Block size = {} bytes / {} kb / {} mb",
            len,
            len / 1024,
            len / 1024 / 1024,
        );

        let r = executor_call(&mut t, "Core_execute_block", &block.0).0;

        println!(" || Result = {:?}", r);
        assert!(r.is_ok());

        previous_hash = block.1;
        nonce += 1;
        time += 10;
        block_number += 1;
    }
}
