// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Substrate chain configurations.

use acuity_runtime::{
    constants::currency::*, wasm_binary_unwrap, Block, MaxNominations, SessionKeys, StakerStatus,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_mixnet::types::AuthorityId as MixnetId;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

pub use acuity_runtime::RuntimeGenesisConfig;
pub use primitives::{AccountId, Balance, Signature};

type AccountPublic = <Signature as Verify>::Signer;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const ENDOWMENT: Balance = 1000 * DOLLARS;
const STASH: Balance = ENDOWMENT / 1000;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<Block>,
    /// The light sync state extension used by the sync-state rpc.
    pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;
pub fn acuity_config() -> Result<ChainSpec, String> {
    ChainSpec::from_json_bytes(&include_bytes!("../res/acuity.json")[..])
}

fn session_keys(
    grandpa: GrandpaId,
    babe: BabeId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
    mixnet: MixnetId,
) -> SessionKeys {
    SessionKeys {
        grandpa,
        babe,
        im_online,
        authority_discovery,
        mixnet,
    }
}

fn configure_accounts_for_staging_testnet() -> (
    Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
        MixnetId,
    )>,
    AccountId,
    Vec<AccountId>,
) {
    #[rustfmt::skip]
	let initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
		MixnetId,
	)> = vec![
        (
            array_bytes::hex_n_into_unchecked("34abc953e7af6dbfb5760dc0f36f372b218a079c19c74d3655a6e63b9509b130"),
            array_bytes::hex_n_into_unchecked("76fccc5e327f8eab3de0163db6e2ed133249df9490851a8ed46f6d4f2a0a5e17"),
            array_bytes::hex2array_unchecked("6082d796b5e35f27d42c7fad12033d23a88c9b339261a32a8a2361faed555394").unchecked_into(),
            array_bytes::hex2array_unchecked("543e8d702d39930a312e289c464698ae7c1871202a949cb5ac04a424f9d7017c").unchecked_into(),
            array_bytes::hex2array_unchecked("543e8d702d39930a312e289c464698ae7c1871202a949cb5ac04a424f9d7017c").unchecked_into(),
            array_bytes::hex2array_unchecked("543e8d702d39930a312e289c464698ae7c1871202a949cb5ac04a424f9d7017c").unchecked_into(),
            array_bytes::hex2array_unchecked("543e8d702d39930a312e289c464698ae7c1871202a949cb5ac04a424f9d7017c").unchecked_into(),
        ),
        (
            array_bytes::hex_n_into_unchecked("72fe4614cfefa10901a4d6085820bad8ff7ec93489dd4f1b9d655aa5d190ce20"),
            array_bytes::hex_n_into_unchecked("1453708476f4714cb11b39fe3b44d985d82737a434286de784a1111cf8bf8133"),
            array_bytes::hex2array_unchecked("8ffe93738513d927dd5a2b2a410fa60aa2bc7d2608da6104c2b58ea33698a672").unchecked_into(),
            array_bytes::hex2array_unchecked("b48173d8eebe192fcc9607724795258899c7ebab885ee213d3876b6471aab773").unchecked_into(),
            array_bytes::hex2array_unchecked("b48173d8eebe192fcc9607724795258899c7ebab885ee213d3876b6471aab773").unchecked_into(),
            array_bytes::hex2array_unchecked("b48173d8eebe192fcc9607724795258899c7ebab885ee213d3876b6471aab773").unchecked_into(),
            array_bytes::hex2array_unchecked("b48173d8eebe192fcc9607724795258899c7ebab885ee213d3876b6471aab773").unchecked_into(),
        ),
        (
            array_bytes::hex_n_into_unchecked("4ea6e38d990a708e24187ee5ea29f0385132ed531074dd33453e249427720e28"),
            array_bytes::hex_n_into_unchecked("0061f7514802df87bde3eba6eda071b7aa18b7edd38586ecef5b2b5589d83450"),
            array_bytes::hex2array_unchecked("a9f1d7fd3de69ddcaba977123f983f50e15a8e93547922070e29566fc49e0f6a").unchecked_into(),
            array_bytes::hex2array_unchecked("f631de0a959cf908504ad9575b268d229a2e7e7feb07454388c4b056f96d7746").unchecked_into(),
            array_bytes::hex2array_unchecked("f631de0a959cf908504ad9575b268d229a2e7e7feb07454388c4b056f96d7746").unchecked_into(),
            array_bytes::hex2array_unchecked("f631de0a959cf908504ad9575b268d229a2e7e7feb07454388c4b056f96d7746").unchecked_into(),
            array_bytes::hex2array_unchecked("f631de0a959cf908504ad9575b268d229a2e7e7feb07454388c4b056f96d7746").unchecked_into(),
        ),
        (
            array_bytes::hex_n_into_unchecked("4a422e4c698cca50261750eb813a96619b4c5f331a7f14d57497837a7e525938"),
            array_bytes::hex_n_into_unchecked("b8be314decd36d6aed4610788d9ea7fd41500c744e92a5ce6bbf093f8b14794a"),
            array_bytes::hex2array_unchecked("b0785306c932e9b8541d7ae2ea36f6878df55f57e325503449da3ec38c2ff51f").unchecked_into(),
            array_bytes::hex2array_unchecked("96c79a96217e934d73a29b89e42cdf3314fbeedcf2cff66cdaf942dee2c76c10").unchecked_into(),
            array_bytes::hex2array_unchecked("96c79a96217e934d73a29b89e42cdf3314fbeedcf2cff66cdaf942dee2c76c10").unchecked_into(),
            array_bytes::hex2array_unchecked("96c79a96217e934d73a29b89e42cdf3314fbeedcf2cff66cdaf942dee2c76c10").unchecked_into(),
            array_bytes::hex2array_unchecked("96c79a96217e934d73a29b89e42cdf3314fbeedcf2cff66cdaf942dee2c76c10").unchecked_into(),
        ),
	];

    let root_key: AccountId = array_bytes::hex_n_into_unchecked(
        "3ecbe98d482908b7ab21d224e96da8ca9ca4cf63db2a53a6833d82ed0bfde037",
    );

    let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];
    (initial_authorities, root_key, endowed_accounts)
}

fn staging_testnet_config_genesis() -> serde_json::Value {
    let (initial_authorities, root_key, endowed_accounts) =
        configure_accounts_for_staging_testnet();
    testnet_genesis(
        initial_authorities,
        vec![],
        root_key,
        Some(endowed_accounts),
    )
}

/// Staging testnet config.
pub fn staging_testnet_config() -> ChainSpec {
    ChainSpec::builder(wasm_binary_unwrap(), Default::default())
        .with_name("Staging Testnet")
        .with_id("staging_testnet")
        .with_chain_type(ChainType::Live)
        .with_genesis_config_patch(staging_testnet_config_genesis())
        .with_telemetry_endpoints(
            TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
                .expect("Staging telemetry url is valid; qed"),
        )
        .build()
}

/// Helper function to generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed.
pub fn authority_keys_from_seed(
    seed: &str,
) -> (
    AccountId,
    AccountId,
    GrandpaId,
    BabeId,
    ImOnlineId,
    AuthorityDiscoveryId,
    MixnetId,
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<ImOnlineId>(seed),
        get_from_seed::<AuthorityDiscoveryId>(seed),
        get_from_seed::<MixnetId>(seed),
    )
}

fn configure_accounts(
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
        MixnetId,
    )>,
    initial_nominators: Vec<AccountId>,
    endowed_accounts: Option<Vec<AccountId>>,
    stash: Balance,
) -> (
    Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
        MixnetId,
    )>,
    Vec<AccountId>,
    Vec<(AccountId, AccountId, Balance, StakerStatus<AccountId>)>,
) {
    let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
            get_account_id_from_seed::<sr25519::Public>("Eve"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie"),
            get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
            get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
            get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
        ]
    });
    // endow all authorities and nominators.
    initial_authorities
        .iter()
        .map(|x| &x.0)
        .chain(initial_nominators.iter())
        .for_each(|x| {
            if !endowed_accounts.contains(x) {
                endowed_accounts.push(x.clone())
            }
        });

    // stakers: all validators and nominators.
    let mut rng = rand::thread_rng();
    let stakers = initial_authorities
        .iter()
        .map(|x| (x.0.clone(), x.0.clone(), stash, StakerStatus::Validator))
        .chain(initial_nominators.iter().map(|x| {
            use rand::{seq::SliceRandom, Rng};
            let limit = (MaxNominations::get() as usize).min(initial_authorities.len());
            let count = rng.gen::<usize>() % limit;
            let nominations = initial_authorities
                .as_slice()
                .choose_multiple(&mut rng, count)
                .into_iter()
                .map(|choice| choice.0.clone())
                .collect::<Vec<_>>();
            (
                x.clone(),
                x.clone(),
                stash,
                StakerStatus::Nominator(nominations),
            )
        }))
        .collect::<Vec<_>>();

    (initial_authorities, endowed_accounts, stakers)
}

/// Helper function to create RuntimeGenesisConfig json patch for testing.
pub fn testnet_genesis(
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
        MixnetId,
    )>,
    initial_nominators: Vec<AccountId>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
) -> serde_json::Value {
    let (initial_authorities, endowed_accounts, stakers) = configure_accounts(
        initial_authorities,
        initial_nominators,
        endowed_accounts,
        STASH,
    );

    serde_json::json!({
        "balances": {
            "balances": endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect::<Vec<_>>(),
        },
        "session": {
            "keys": initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(
                            x.2.clone(),
                            x.3.clone(),
                            x.4.clone(),
                            x.5.clone(),
                            x.6.clone(),
                        ),
                    )
                })
                .collect::<Vec<_>>(),
        },
        "staking": {
            "validatorCount": initial_authorities.len() as u32,
            "minimumValidatorCount": initial_authorities.len() as u32,
            "invulnerables": initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
            "slashRewardFraction": Perbill::from_percent(10),
            "stakers": stakers.clone(),
        },
        "sudo": { "key": Some(root_key.clone()) },
        "babe": {
            "epochConfig": Some(acuity_runtime::BABE_GENESIS_EPOCH_CONFIG),
        },
        "nominationPools": {
            "minCreateBond": 10 * DOLLARS,
            "minJoinBond": 1 * DOLLARS,
        },
    })
}

fn development_config_genesis_json() -> serde_json::Value {
    testnet_genesis(
        vec![authority_keys_from_seed("Alice")],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
    )
}

/// Development config (single validator Alice).
pub fn development_config() -> ChainSpec {
    ChainSpec::builder(wasm_binary_unwrap(), Default::default())
        .with_name("Development")
        .with_id("dev")
        .with_chain_type(ChainType::Development)
        .with_genesis_config_patch(development_config_genesis_json())
        .build()
}

fn local_testnet_genesis() -> serde_json::Value {
    testnet_genesis(
        vec![
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
    )
}

/// Local testnet config (multivalidator Alice + Bob).
pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::builder(wasm_binary_unwrap(), Default::default())
        .with_name("Local Testnet")
        .with_id("local_testnet")
        .with_chain_type(ChainType::Local)
        .with_genesis_config_patch(local_testnet_genesis())
        .build()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::service::{new_full_base, NewFullBase};
    use sp_runtime::BuildStorage;

    /// Local testnet config (single validator - Alice).
    pub fn integration_test_config_with_single_authority() -> ChainSpec {
        ChainSpec::builder(wasm_binary_unwrap(), Default::default())
            .with_name("Integration Test")
            .with_id("test")
            .with_chain_type(ChainType::Development)
            .with_genesis_config_patch(testnet_genesis(
                vec![authority_keys_from_seed("Alice")],
                vec![],
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                None,
            ))
            .build()
    }

    /// Local testnet config (multivalidator Alice + Bob).
    pub fn integration_test_config_with_two_authorities() -> ChainSpec {
        ChainSpec::builder(wasm_binary_unwrap(), Default::default())
            .with_name("Integration Test")
            .with_id("test")
            .with_chain_type(ChainType::Development)
            .with_genesis_config_patch(local_testnet_genesis())
            .build()
    }

    #[test]
    fn test_create_development_chain_spec() {
        development_config().build_storage().unwrap();
    }

    #[test]
    fn test_create_local_testnet_chain_spec() {
        local_testnet_config().build_storage().unwrap();
    }

    #[test]
    fn test_staging_test_net_chain_spec() {
        staging_testnet_config().build_storage().unwrap();
    }
}
