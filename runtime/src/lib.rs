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

//! The Substrate runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limits.
#![recursion_limit = "1024"]

extern crate alloc;

use alloc::{vec, vec::Vec};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_election_provider_support::{
    bounds::{ElectionBounds, ElectionBoundsBuilder},
    onchain, BalancingConfig, ElectionDataProvider, SequentialPhragmen, VoteWeight,
};
use frame_support::{
    derive_impl,
    dispatch::DispatchClass,
    genesis_builder_helper::{build_state, get_preset},
    instances::{Instance1, Instance2},
    ord_parameter_types,
    pallet_prelude::Get,
    parameter_types,
    traits::{
        fungible::{
            Balanced, Credit, HoldConsideration, ItemOf, NativeFromLeft, NativeOrWithId, UnionOf,
        },
        tokens::{
            imbalance::ResolveAssetTo, nonfungibles_v2::Inspect, pay::PayAssetFromAccount,
            ConversionFromAssetBalance, GetSalary, PayFromAccount,
        },
        AsEnsureOriginWithArg, ConstBool, ConstU128, ConstU16, ConstU32, Contains, Currency,
        EitherOf, EitherOfDiverse, EnsureOriginWithArg, EqualPrivilegeOnly, Imbalance, InsideBoth,
        InstanceFilter, KeyOwnerProofSystem, LinearStoragePrice, LockIdentifier, Nothing,
        OnUnbalanced, VariantCountOf, WithdrawReasons,
    },
    weights::{
        constants::{
            BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
        },
        ConstantMultiplier, IdentityFee, Weight,
    },
    BoundedVec, PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, EnsureRootWithSuccess, EnsureSigned, EnsureSignedBy, EnsureWithSuccess,
};
pub use node_primitives::{AccountId, Signature};
use node_primitives::{AccountIndex, Balance, BlockNumber, Hash, Moment, Nonce};
use pallet_asset_conversion::{AccountIdConverter, Ascending, Chain, WithFirstAsset};
use pallet_asset_conversion_tx_payment::SwapAssetAdapter;
// use pallet_broker::{CoreAssignment, CoreIndex, CoretimeInterface, PartsOf57600};
use pallet_election_provider_multi_phase::{GeometricDepositBase, SolutionAccuracyOf};
use pallet_identity::legacy::IdentityInfo;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
// use pallet_nis::WithMaximumOf;
use pallet_session::historical as pallet_session_historical;
// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
// use pallet_broker::TaskId;
#[allow(deprecated)]
pub use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
// use pallet_tx_pause::RuntimeCallNameOf;
use sp_api::impl_runtime_apis;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_beefy::{
    ecdsa_crypto::{AuthorityId as BeefyId, Signature as BeefySignature},
    mmr::MmrLeafVersion,
};
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
    create_runtime_str,
    curve::PiecewiseLinear,
    generic, impl_opaque_keys,
    traits::{
        self, AccountIdConversion, BlakeTwo256, Block as BlockT, Bounded, ConvertInto,
        MaybeConvert, NumberFor, OpaqueKeys, SaturatedConversion, StaticLookup,
    },
    transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, FixedPointNumber, FixedU128, Perbill, Percent, Permill, Perquintill,
    RuntimeDebug,
};
use sp_std::prelude::*;
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use static_assertions::const_assert;

#[cfg(any(feature = "std", test))]
pub use frame_system::Call as SystemCall;
#[cfg(any(feature = "std", test))]
pub use pallet_balances::Call as BalancesCall;
#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;
use impls::Author;

pub mod constants;
use constants::{currency::*, time::*};
use sp_runtime::generic::Era;

/// A timestamp: milliseconds since the unix epoch.
/// `u64` is enough to represent a duration of half a billion years, when the
/// time scale is milliseconds.
//pub type Timestamp = u64;

/// Digest item type.
pub type DigestItem = generic::DigestItem;

pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
// pub mod opaque {
//     use super::*;

//     pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

//     /// Opaque block header type.
//     pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
//     /// Opaque block type.
//     pub type Block = generic::Block<Header, UncheckedExtrinsic>;
//     /// Opaque block identifier type.
//     pub type BlockId = generic::BlockId<Block>;
// }

// mod weights;

/// Generated voter bag information.
mod voter_bags;

// Governance configurations.
pub mod governance;
use governance::{pallet_custom_origins, GeneralAdmin, StakingAdmin, Treasurer, TreasurySpender};

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Max size for serialized extrinsic params for this testing runtime.
/// This is a quite arbitrary but empirically battle tested value.
#[cfg(test)]
pub const CALL_PARAMS_MAX_SIZE: usize = 244;

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
    WASM_BINARY.expect(
        "Development wasm binary is not available. This means the client is built with \
		 `SKIP_WASM_BUILD` flag and it is only usable for production chains. Please rebuild with \
		 the flag disabled.",
    )
}

/// Runtime version (Acuity).
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("acuity"),
    impl_name: create_runtime_str!("acuity-substrate"),
    authoring_version: 3,
    // Per convention: if the runtime behavior changes, increment spec_version
    // and set impl_version to 0. If only runtime
    // implementation changes and behavior does not, then leave spec_version as
    // is and increment impl_version.
    spec_version: 11,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 5,
    state_version: 1,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
    sp_consensus_babe::BabeEpochConfiguration {
        c: PRIMARY_PROBABILITY,
        allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
    };

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_unbalanceds(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
        if let Some(fees) = fees_then_tips.next() {
            // for fees, 80% to treasury, 20% to author
            let mut split = fees.ration(80, 20);
            if let Some(tips) = fees_then_tips.next() {
                // for tips, if any, 80% to treasury, 20% to author (though this can be anything)
                tips.ration_merge_into(80, 20, &mut split);
            }
            Treasury::on_unbalanced(split.0);
            Author::on_unbalanced(split.1);
        }
    }
}

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time, with maximum proof size.
const MAXIMUM_BLOCK_WEIGHT: Weight =
    Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    pub const Version: RuntimeVersion = VERSION;
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

const_assert!(NORMAL_DISPATCH_RATIO.deconstruct() >= AVERAGE_ON_INITIALIZE_RATIO.deconstruct());

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig as frame_system::DefaultConfig)]
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
    type BaseCallFilter = ();
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type DbWeight = RocksDbWeight;
    type Nonce = Nonce;
    type Hash = Hash;
    type AccountId = AccountId;
    type Lookup = Indices;
    type Block = Block;
    type BlockHashCount = BlockHashCount;
    type Version = Version;
    type AccountData = pallet_balances::AccountData<Balance>;
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    type SS58Prefix = ConstU16<42>;
    type MaxConsumers = ConstU32<16>;
    type MultiBlockMigrator = ();
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

impl pallet_utility::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
}

impl pallet_multisig::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = ConstU32<100>;
    type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    // One storage item; key size 32, value size 8; .
    pub const ProxyDepositBase: Balance = deposit(1, 8);
    // Additional storage item size of 33 bytes.
    pub const ProxyDepositFactor: Balance = deposit(0, 33);
    pub const AnnouncementDepositBase: Balance = deposit(1, 8);
    pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Encode,
    Decode,
    RuntimeDebug,
    MaxEncodedLen,
    scale_info::TypeInfo,
)]
pub enum ProxyType {
    Any,
    NonTransfer,
    Governance,
    Staking,
}
impl Default for ProxyType {
    fn default() -> Self {
        Self::Any
    }
}
impl InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::NonTransfer => !matches!(
                c,
                RuntimeCall::Balances(..)
                    | RuntimeCall::Vesting(pallet_vesting::Call::vested_transfer { .. })
                    | RuntimeCall::Indices(pallet_indices::Call::transfer { .. })
            ),
            ProxyType::Governance => matches!(c, RuntimeCall::Treasury(..)),
            ProxyType::Staking => {
                matches!(c, RuntimeCall::Staking(..) | RuntimeCall::FastUnstake(..))
            }
        }
    }
    fn is_superset(&self, o: &Self) -> bool {
        match (self, o) {
            (x, y) if x == y => true,
            (ProxyType::Any, _) => true,
            (_, ProxyType::Any) => false,
            (ProxyType::NonTransfer, _) => true,
            _ => false,
        }
    }
}

impl pallet_proxy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type ProxyType = ProxyType;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type MaxProxies = ConstU32<32>;
    type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
    type MaxPending = ConstU32<32>;
    type CallHasher = BlakeTwo256;
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
        RuntimeBlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    #[cfg(feature = "runtime-benchmarks")]
    type MaxScheduledPerBlock = ConstU32<512>;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type MaxScheduledPerBlock = ConstU32<50>;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type Preimages = Preimage;
}

parameter_types! {
    pub const PreimageBaseDeposit: Balance = 1 * DOLLARS;
    // One cent: $10,000 / MB
    pub const PreimageByteDeposit: Balance = 1 * CENTS;
    pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
    type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type Consideration = HoldConsideration<
        AccountId,
        Balances,
        PreimageHoldReason,
        LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
    >;
}

parameter_types! {
    // NOTE: Currently it is not possible to change the epoch duration after the chain has started.
    //       Attempting to do so will brick block production.
    pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
    pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
    pub const ReportLongevity: u64 =
        BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl pallet_babe::Config for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;
    type DisabledValidators = Session;
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = MaxNominators;
    type KeyOwnerProof =
        <Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
    type EquivocationReportSystem =
        pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
    pub const IndexDeposit: Balance = 1 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
    type AccountIndex = AccountIndex;
    type Currency = Balances;
    type Deposit = IndexDeposit;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_indices::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = ACU / 1_000;
    // For weight estimation, we assume that the most locks on an individual account will be 50.
    // This number may need to be adjusted in the future if this assumption no longer holds true.
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
}

parameter_types! {
    pub const TransactionByteFee: Balance = 10 * MILLICENTS;
    pub const OperationalFeeMultiplier: u8 = 5;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
    pub MaximumMultiplier: Multiplier = Bounded::max_value();
}

// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
#[allow(deprecated)]
impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type WeightToFee = IdentityFee<Balance>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = TargetedFeeAdjustment<
        Self,
        TargetBlockFullness,
        AdjustmentVariable,
        MinimumMultiplier,
        MaximumMultiplier,
    >;
}

impl pallet_asset_conversion_tx_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = NativeOrWithId<u32>;
    type OnChargeAssetTransaction = SwapAssetAdapter<
        Native,
        NativeAndAssets,
        AssetConversion,
        ResolveAssetTo<TreasuryAccount, NativeAndAssets>,
    >;
}

impl pallet_skip_feeless_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
    pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = Moment;
    type OnTimestampSet = Babe;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
    type EventHandler = (Staking, ImOnline);
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub grandpa: Grandpa,
        pub babe: Babe,
        pub im_online: ImOnline,
        pub authority_discovery: AuthorityDiscovery,
        pub mixnet: Mixnet,
        pub beefy: Beefy,
    }
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_staking::StashOf<Self>;
    type ShouldEndSession = Babe;
    type NextSessionRotation = Babe;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

pallet_staking_reward_curve::build! {
    const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
        min_inflation: 0_025_000,
        max_inflation: 0_100_000,
        ideal_stake: 0_500_000,
        falloff: 0_050_000,
        max_piece_count: 40,
        test_precision: 0_005_000,
    );
}

parameter_types! {
    pub const SessionsPerEra: sp_staking::SessionIndex = 6;
    pub const BondingDuration: sp_staking::EraIndex = 24 * 28;
    pub const SlashDeferDuration: sp_staking::EraIndex = 24 * 7; // 1/4 the bonding duration.
    pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
    pub const MaxNominators: u32 = 64;
    pub const MaxControllersInDeprecationBatch: u32 = 5900;
    pub OffchainRepeat: BlockNumber = 5;
    pub HistoryDepth: u32 = 84;
}

/// Upper limit on the number of NPOS nominations.
const MAX_QUOTA_NOMINATIONS: u32 = 16;

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
    type MaxNominators = ConstU32<1000>;
    type MaxValidators = ConstU32<1000>;
}

impl pallet_staking::Config for Runtime {
    type Currency = Balances;
    type CurrencyBalance = Balance;
    type UnixTime = Timestamp;
    type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
    type RewardRemainder = Treasury;
    type RuntimeEvent = RuntimeEvent;
    type Slash = Treasury; // send the slashed funds to the treasury.
    type Reward = (); // rewards are minted from the void
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    type AdminOrigin = EitherOf<EnsureRoot<Self::AccountId>, StakingAdmin>;
    type SessionInterface = Self;
    type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
    type NextNewSession = Session;
    type MaxExposurePageSize = ConstU32<256>;
    type ElectionProvider = ElectionProviderMultiPhase;
    type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type VoterList = VoterList;
    type NominationsQuota = pallet_staking::FixedNominationsQuota<MAX_QUOTA_NOMINATIONS>;
    // This a placeholder, to be introduced in the next PR as an instance of bags-list
    type TargetList = pallet_staking::UseValidatorsMap<Self>;
    type MaxUnlockingChunks = ConstU32<32>;
    type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
    type HistoryDepth = HistoryDepth;
    type EventListeners = NominationPools;
    type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
    type BenchmarkingConfig = StakingBenchmarkingConfig;
    type DisablingStrategy = pallet_staking::UpToLimitDisablingStrategy;
}

impl pallet_fast_unstake::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ControlOrigin = frame_system::EnsureRoot<AccountId>;
    type BatchSize = ConstU32<64>;
    type Deposit = ConstU128<{ DOLLARS }>;
    type Currency = Balances;
    type Staking = Staking;
    type MaxErasToCheckPerBlock = ConstU32<1>;
    type WeightInfo = ();
}

parameter_types! {
    // phase durations. 1/4 of the last session for each.
    pub const SignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;
    pub const UnsignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;

    // signed config
    pub const SignedRewardBase: Balance = 1 * DOLLARS;
    pub const SignedFixedDeposit: Balance = 1 * DOLLARS;
    pub const SignedDepositIncreaseFactor: Percent = Percent::from_percent(10);
    pub const SignedDepositByte: Balance = 1 * CENTS;

    // miner configs
    pub const MultiPhaseUnsignedPriority: TransactionPriority = StakingUnsignedPriority::get() - 1u64;
    pub MinerMaxWeight: Weight = RuntimeBlockWeights::get()
        .get(DispatchClass::Normal)
        .max_extrinsic.expect("Normal extrinsics have a weight limit configured; qed")
        .saturating_sub(BlockExecutionWeight::get());
    // Solution can occupy 90% of normal block size
    pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
        *RuntimeBlockLength::get()
        .max
        .get(DispatchClass::Normal);
}

frame_election_provider_support::generate_solution_type!(
    #[compact]
    pub struct NposSolution16::<
        VoterIndex = u32,
        TargetIndex = u16,
        Accuracy = sp_runtime::PerU16,
        MaxVoters = MaxElectingVotersSolution,
    >(16)
);

parameter_types! {
    // Note: the EPM in this runtime runs the election on-chain. The election bounds must be
    // carefully set so that an election round fits in one block.
    pub ElectionBoundsMultiPhase: ElectionBounds = ElectionBoundsBuilder::default()
        .voters_count(10_000.into()).targets_count(1_500.into()).build();
    pub ElectionBoundsOnChain: ElectionBounds = ElectionBoundsBuilder::default()
        .voters_count(5_000.into()).targets_count(1_250.into()).build();

    pub MaxNominations: u32 = <NposSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;
    pub MaxElectingVotersSolution: u32 = 40_000;
    // The maximum winners that can be elected by the Election pallet which is equivalent to the
    // maximum active validators the staking pallet can have.
    pub MaxActiveValidators: u32 = 1000;
}

/// The numbers configured here could always be more than the the maximum limits of staking pallet
/// to ensure election snapshot will not run out of memory. For now, we set them to smaller values
/// since the staking is bounded and the weight pipeline takes hours for this single pallet.
pub struct ElectionProviderBenchmarkConfig;
impl pallet_election_provider_multi_phase::BenchmarkingConfig for ElectionProviderBenchmarkConfig {
    const VOTERS: [u32; 2] = [1000, 2000];
    const TARGETS: [u32; 2] = [500, 1000];
    const ACTIVE_VOTERS: [u32; 2] = [500, 800];
    const DESIRED_TARGETS: [u32; 2] = [200, 400];
    const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
    const MINER_MAXIMUM_VOTERS: u32 = 1000;
    const MAXIMUM_TARGETS: u32 = 300;
}

/// Maximum number of iterations for balancing that will be executed in the embedded OCW
/// miner of election provider multi phase.
pub const MINER_MAX_ITERATIONS: u32 = 10;

/// A source of random balance for NposSolver, which is meant to be run by the OCW election miner.
pub struct OffchainRandomBalancing;
impl Get<Option<BalancingConfig>> for OffchainRandomBalancing {
    fn get() -> Option<BalancingConfig> {
        use sp_runtime::traits::TrailingZeroInput;
        let iterations = match MINER_MAX_ITERATIONS {
            0 => 0,
            max => {
                let seed = sp_io::offchain::random_seed();
                let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
                    .expect("input is padded with zeroes; qed")
                    % max.saturating_add(1);
                random as usize
            }
        };

        let config = BalancingConfig {
            iterations,
            tolerance: 0,
        };
        Some(config)
    }
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
    type System = Runtime;
    type Solver = SequentialPhragmen<
        AccountId,
        pallet_election_provider_multi_phase::SolutionAccuracyOf<Runtime>,
    >;
    type DataProvider = <Runtime as pallet_election_provider_multi_phase::Config>::DataProvider;
    type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
    type MaxWinners = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
    type Bounds = ElectionBoundsOnChain;
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
    type AccountId = AccountId;
    type MaxLength = MinerMaxLength;
    type MaxWeight = MinerMaxWeight;
    type Solution = NposSolution16;
    type MaxVotesPerVoter =
	<<Self as pallet_election_provider_multi_phase::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter;
    type MaxWinners = MaxActiveValidators;

    // The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
    // weight estimate function is wired to this call's weight.
    fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
        <
			<Self as pallet_election_provider_multi_phase::Config>::WeightInfo
			as
			pallet_election_provider_multi_phase::WeightInfo
		>::submit_unsigned(v, t, a, d)
    }
}

impl pallet_election_provider_multi_phase::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EstimateCallFee = TransactionPayment;
    type SignedPhase = SignedPhase;
    type UnsignedPhase = UnsignedPhase;
    type BetterSignedThreshold = ();
    type OffchainRepeat = OffchainRepeat;
    type MinerTxPriority = MultiPhaseUnsignedPriority;
    type MinerConfig = Self;
    type SignedMaxSubmissions = ConstU32<10>;
    type SignedRewardBase = SignedRewardBase;
    type SignedDepositBase =
        GeometricDepositBase<Balance, SignedFixedDeposit, SignedDepositIncreaseFactor>;
    type SignedDepositByte = SignedDepositByte;
    type SignedMaxRefunds = ConstU32<3>;
    type SignedDepositWeight = ();
    type SignedMaxWeight = MinerMaxWeight;
    type SlashHandler = (); // burn slashes
    type RewardHandler = (); // nothing to do upon rewards
    type DataProvider = Staking;
    type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, OffchainRandomBalancing>;
    type ForceOrigin = EitherOf<EnsureRoot<Self::AccountId>, StakingAdmin>;
    type MaxWinners = MaxActiveValidators;
    type ElectionBounds = ElectionBoundsMultiPhase;
    type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
    type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Self>;
}

parameter_types! {
    pub const BagThresholds: &'static [u64] = &voter_bags::THRESHOLDS;
}

type VoterBagsListInstance = pallet_bags_list::Instance1;
impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    /// The voter bags-list is loosely kept up to date, and the real source of truth for the score
    /// of each node is the staking pallet.
    type ScoreProvider = Staking;
    type BagThresholds = BagThresholds;
    type Score = VoteWeight;
    type WeightInfo = pallet_bags_list::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const PostUnbondPoolsWindow: u32 = 4;
    pub const NominationPoolsPalletId: PalletId = PalletId(*b"py/nopls");
    pub const MaxPointsToBalance: u8 = 10;
}

use sp_runtime::traits::{Convert, Keccak256};
pub struct BalanceToU256;
impl Convert<Balance, sp_core::U256> for BalanceToU256 {
    fn convert(balance: Balance) -> sp_core::U256 {
        sp_core::U256::from(balance)
    }
}
pub struct U256ToBalance;
impl Convert<sp_core::U256, Balance> for U256ToBalance {
    fn convert(n: sp_core::U256) -> Balance {
        n.try_into().unwrap_or(Balance::max_value())
    }
}

impl pallet_nomination_pools::Config for Runtime {
    type WeightInfo = ();
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type RewardCounter = FixedU128;
    type BalanceToU256 = BalanceToU256;
    type U256ToBalance = U256ToBalance;
    type StakeAdapter = pallet_nomination_pools::adapter::TransferStake<Self, Staking>;
    type PostUnbondingPoolsWindow = PostUnbondPoolsWindow;
    type MaxMetadataLen = ConstU32<256>;
    type MaxUnbonding = ConstU32<8>;
    type PalletId = NominationPoolsPalletId;
    type MaxPointsToBalance = MaxPointsToBalance;
    type AdminOrigin = EnsureRoot<AccountId>;
}

impl pallet_remark::Config for Runtime {
    type WeightInfo = pallet_remark::weights::SubstrateWeight<Self>;
    type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
    pub const SpendPeriod: BlockNumber = 1 * DAYS;
    pub const Burn: Permill = Permill::from_percent(50);
    pub const TipCountdown: BlockNumber = 1 * DAYS;
    pub const TipFindersFee: Percent = Percent::from_percent(20);
    pub const TipReportDepositBase: Balance = 1 * DOLLARS;
    pub const DataDepositPerByte: Balance = 1 * CENTS;
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const MaximumReasonLength: u32 = 300;
    pub const MaxApprovals: u32 = 100;
    pub const MaxBalance: Balance = Balance::max_value();
    pub const SpendPayoutPeriod: BlockNumber = 30 * DAYS;
    pub TreasuryAccount: AccountId = Treasury::account_id();
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type RejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
    type RuntimeEvent = RuntimeEvent;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = ();
    type SpendFunds = Bounties;
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type MaxApprovals = MaxApprovals;
    type SpendOrigin = EnsureWithSuccess<EnsureRoot<AccountId>, AccountId, MaxBalance>;
    type AssetKind = u32;
    type Beneficiary = AccountId;
    type BeneficiaryLookup = Indices;
    type Paymaster = PayAssetFromAccount<Assets, TreasuryAccount>;
    type BalanceConverter = AssetRate;
    type PayoutPeriod = SpendPayoutPeriod;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

impl pallet_asset_rate::Config for Runtime {
    type CreateOrigin = EnsureRoot<AccountId>;
    type RemoveOrigin = EnsureRoot<AccountId>;
    type UpdateOrigin = EnsureRoot<AccountId>;
    type Currency = Balances;
    type AssetKind = u32;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_asset_rate::weights::SubstrateWeight<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

parameter_types! {
    pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
    pub const BountyValueMinimum: Balance = 5 * DOLLARS;
    pub const BountyDepositBase: Balance = 1 * DOLLARS;
    pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
    pub const CuratorDepositMin: Balance = 1 * DOLLARS;
    pub const CuratorDepositMax: Balance = 100 * DOLLARS;
    pub const BountyDepositPayoutDelay: BlockNumber = 1 * DAYS;
    pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;
}

impl pallet_bounties::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BountyDepositBase = BountyDepositBase;
    type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
    type BountyUpdatePeriod = BountyUpdatePeriod;
    type CuratorDepositMultiplier = CuratorDepositMultiplier;
    type CuratorDepositMin = CuratorDepositMin;
    type CuratorDepositMax = CuratorDepositMax;
    type BountyValueMinimum = BountyValueMinimum;
    type DataDepositPerByte = DataDepositPerByte;
    type MaximumReasonLength = MaximumReasonLength;
    type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
    type ChildBountyManager = ChildBounties;
    type OnSlash = Treasury;
}

parameter_types! {
    /// Allocate at most 20% of each block for message processing.
    ///
    /// Is set to 20% since the scheduler can already consume a maximum of 80%.
    pub MessageQueueServiceWeight: Option<Weight> = Some(Perbill::from_percent(20) * RuntimeBlockWeights::get().max_block);
}

impl pallet_message_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    /// NOTE: Always set this to `NoopMessageProcessor` for benchmarking.
    type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<u32>;
    type Size = u32;
    type QueueChangeHandler = ();
    type QueuePausedQuery = ();
    type HeapSize = ConstU32<{ 64 * 1024 }>;
    type MaxStale = ConstU32<128>;
    type ServiceWeight = MessageQueueServiceWeight;
    type IdleMaxServiceWeight = ();
}

parameter_types! {
    pub const ChildBountyValueMinimum: Balance = 1 * DOLLARS;
}

impl pallet_child_bounties::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxActiveChildBountyCount = ConstU32<5>;
    type ChildBountyValueMinimum = ChildBountyValueMinimum;
    type WeightInfo = pallet_child_bounties::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
    /// We prioritize im-online heartbeats over election solution submission.
    pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
    pub const MaxAuthorities: u32 = 100;
    pub const MaxKeys: u32 = 10_000;
    pub const MaxPeerInHeartbeats: u32 = 10_000;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
    RuntimeCall: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: RuntimeCall,
        public: <Signature as traits::Verify>::Signer,
        account: AccountId,
        nonce: Nonce,
    ) -> Option<(
        RuntimeCall,
        <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload,
    )> {
        let tip = 0;
        // take the biggest period possible.
        let period = BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;
        let current_block = System::block_number()
            .saturated_into::<u64>()
            // The `System::block_number` is initialized with `n+1`,
            // so the actual block number is `n`.
            .saturating_sub(1);
        let era = Era::mortal(period, current_block);
        let extra = (
            frame_system::CheckNonZeroSender::<Runtime>::new(),
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(era),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_skip_feeless_payment::SkipCheckIfFeeless::from(
                pallet_asset_conversion_tx_payment::ChargeAssetTxPayment::<Runtime>::from(
                    tip, None,
                ),
            ),
            frame_metadata_hash_extension::CheckMetadataHash::new(false),
        );
        let raw_payload = SignedPayload::new(call, extra)
            .map_err(|e| {
                log::warn!("Unable to create signed payload: {:?}", e);
            })
            .ok()?;
        let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
        let address = Indices::unlookup(account);
        let (call, extra, _) = raw_payload.deconstruct();
        Some((call, (address, signature, extra)))
    }
}

impl frame_system::offchain::SigningTypes for Runtime {
    type Public = <Signature as traits::Verify>::Signer;
    type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    RuntimeCall: From<C>,
{
    type Extrinsic = UncheckedExtrinsic;
    type OverarchingCall = RuntimeCall;
}

impl pallet_im_online::Config for Runtime {
    type AuthorityId = ImOnlineId;
    type RuntimeEvent = RuntimeEvent;
    type NextSessionRotation = Babe;
    type ValidatorSet = Historical;
    type ReportUnresponsiveness = Offences;
    type UnsignedPriority = ImOnlineUnsignedPriority;
    type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
    type MaxKeys = MaxKeys;
    type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

impl pallet_offences::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
    type OnOffenceHandler = Staking;
}

impl pallet_authority_discovery::Config for Runtime {
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = MaxNominators;
    type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
    type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
    type EquivocationReportSystem =
        pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
    // difference of 26 bytes on-chain for the registration and 9 bytes on-chain for the identity
    // information, already accounted for by the byte deposit
    pub const BasicDeposit: Balance = deposit(1, 17);
    pub const ByteDeposit: Balance = deposit(0, 1);
    pub const SubAccountDeposit: Balance = 2 * DOLLARS;   // 53 bytes on-chain
    pub const MaxSubAccounts: u32 = 100;
    pub const MaxAdditionalFields: u32 = 100;
    pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BasicDeposit = BasicDeposit;
    type ByteDeposit = ByteDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = MaxSubAccounts;
    type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
    type MaxRegistrars = MaxRegistrars;
    type Slashed = Treasury;
    type ForceOrigin = EitherOf<EnsureRoot<Self::AccountId>, GeneralAdmin>;
    type RegistrarOrigin = EitherOf<EnsureRoot<Self::AccountId>, GeneralAdmin>;
    type OffchainSignature = Signature;
    type SigningPublicKey = <Signature as traits::Verify>::Signer;
    type UsernameAuthorityOrigin = EnsureRoot<Self::AccountId>;
    type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
    type MaxSuffixLength = ConstU32<7>;
    type MaxUsernameLength = ConstU32<32>;
    type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ConfigDepositBase: Balance = 5 * DOLLARS;
    pub const FriendDepositFactor: Balance = 50 * CENTS;
    pub const MaxFriends: u16 = 9;
    pub const RecoveryDeposit: Balance = 5 * DOLLARS;
}

impl pallet_recovery::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_recovery::weights::SubstrateWeight<Runtime>;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type ConfigDepositBase = ConfigDepositBase;
    type FriendDepositFactor = FriendDepositFactor;
    type MaxFriends = MaxFriends;
    type RecoveryDeposit = RecoveryDeposit;
}

parameter_types! {
    pub const MinVestedTransfer: Balance = 100 * DOLLARS;
    pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
        WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
    type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
    type BlockNumberProvider = System;
    // `VestingInfo` encode length is 36bytes. 28 schedules gets encoded as 1009 bytes, which is the
    // highest number of schedules that encodes less than 2^10.
    const MAX_VESTING_SCHEDULES: u32 = 28;
}

impl pallet_mmr::Config for Runtime {
    const INDEXING_PREFIX: &'static [u8] = b"mmr";
    type Hashing = Keccak256;
    type LeafData = pallet_mmr::ParentNumberAndHash<Self>;
    type OnNewRoot = pallet_beefy_mmr::DepositBeefyDigest<Runtime>;
    type BlockHashProvider = pallet_mmr::DefaultBlockHashProvider<Runtime>;
    type WeightInfo = ();
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

parameter_types! {
    pub LeafVersion: MmrLeafVersion = MmrLeafVersion::new(0, 0);
}

impl pallet_beefy_mmr::Config for Runtime {
    type LeafVersion = LeafVersion;
    type BeefyAuthorityToMerkleLeaf = pallet_beefy_mmr::BeefyEcdsaToEthereum;
    type LeafExtra = Vec<u8>;
    type BeefyDataProvider = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const AssetDeposit: Balance = 100 * DOLLARS;
    pub const ApprovalDeposit: Balance = 1 * DOLLARS;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = 10 * DOLLARS;
    pub const MetadataDepositPerByte: Balance = 1 * DOLLARS;
}

impl pallet_assets::Config<Instance1> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type AssetId = u32;
    type AssetIdParameter = codec::Compact<u32>;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = ConstU128<DOLLARS>;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type CallbackHandle = ();
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
    type RemoveItemsLimit = ConstU32<1000>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

ord_parameter_types! {
    pub const AssetConversionOrigin: AccountId = AccountIdConversion::<AccountId>::into_account_truncating(&AssetConversionPalletId::get());
}

impl pallet_assets::Config<Instance2> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type AssetId = u32;
    type AssetIdParameter = codec::Compact<u32>;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSignedBy<AssetConversionOrigin, AccountId>>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = ConstU128<DOLLARS>;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
    type RemoveItemsLimit = ConstU32<1000>;
    type CallbackHandle = ();
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

parameter_types! {
    pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
    pub const PoolSetupFee: Balance = 1 * DOLLARS; // should be more or equal to the existential deposit
    pub const MintMinLiquidity: Balance = 100;  // 100 is good enough when the main currency has 10-12 decimals.
    pub const LiquidityWithdrawalFee: Permill = Permill::from_percent(0);
    pub const Native: NativeOrWithId<u32> = NativeOrWithId::Native;
}

pub type NativeAndAssets =
    UnionOf<Balances, Assets, NativeFromLeft, NativeOrWithId<u32>, AccountId>;

impl pallet_asset_conversion::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type HigherPrecisionBalance = sp_core::U256;
    type AssetKind = NativeOrWithId<u32>;
    type Assets = NativeAndAssets;
    type PoolId = (Self::AssetKind, Self::AssetKind);
    type PoolLocator = Chain<
        WithFirstAsset<
            Native,
            AccountId,
            NativeOrWithId<u32>,
            AccountIdConverter<AssetConversionPalletId, Self::PoolId>,
        >,
        Ascending<
            AccountId,
            NativeOrWithId<u32>,
            AccountIdConverter<AssetConversionPalletId, Self::PoolId>,
        >,
    >;
    type PoolAssetId = <Self as pallet_assets::Config<Instance2>>::AssetId;
    type PoolAssets = PoolAssets;
    type PoolSetupFee = PoolSetupFee;
    type PoolSetupFeeAsset = Native;
    type PoolSetupFeeTarget = ResolveAssetTo<AssetConversionOrigin, Self::Assets>;
    type PalletId = AssetConversionPalletId;
    type LPFee = ConstU32<3>; // means 0.3%
    type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
    type WeightInfo = pallet_asset_conversion::weights::SubstrateWeight<Runtime>;
    type MaxSwapPathLength = ConstU32<4>;
    type MintMinLiquidity = MintMinLiquidity;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

parameter_types! {
    pub const AtomicSwapPalletId: PalletId = PalletId(*b"atomcswp");
}

impl pallet_transaction_storage::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeCall = RuntimeCall;
    type FeeDestination = ();
    type WeightInfo = pallet_transaction_storage::weights::SubstrateWeight<Runtime>;
    type MaxBlockTransactions =
        ConstU32<{ pallet_transaction_storage::DEFAULT_MAX_BLOCK_TRANSACTIONS }>;
    type MaxTransactionSize =
        ConstU32<{ pallet_transaction_storage::DEFAULT_MAX_TRANSACTION_SIZE }>;
}

parameter_types! {
    pub StatementCost: Balance = 1 * DOLLARS;
    pub StatementByteCost: Balance = 100 * MILLICENTS;
    pub const MinAllowedStatements: u32 = 4;
    pub const MaxAllowedStatements: u32 = 10;
    pub const MinAllowedBytes: u32 = 1024;
    pub const MaxAllowedBytes: u32 = 4096;
}

impl pallet_statement::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type StatementCost = StatementCost;
    type ByteCost = StatementByteCost;
    type MinAllowedStatements = MinAllowedStatements;
    type MaxAllowedStatements = MaxAllowedStatements;
    type MinAllowedBytes = MinAllowedBytes;
    type MaxAllowedBytes = MaxAllowedBytes;
}

pub struct IntoAuthor;
impl OnUnbalanced<Credit<AccountId, Balances>> for IntoAuthor {
    fn on_nonzero_unbalanced(credit: Credit<AccountId, Balances>) {
        if let Some(author) = Authorship::author() {
            let _ = <Balances as Balanced<_>>::resolve(&author, credit);
        }
    }
}

// pub struct CoretimeProvider;
// impl CoretimeInterface for CoretimeProvider {
//     type AccountId = AccountId;
//     type Balance = Balance;
//     type RelayChainBlockNumberProvider = System;
//     fn request_core_count(_count: CoreIndex) {}
//     fn request_revenue_info_at(_when: u32) {}
//     fn credit_account(_who: Self::AccountId, _amount: Self::Balance) {}
//     fn assign_core(
//         _core: CoreIndex,
//         _begin: u32,
//         _assignment: Vec<(CoreAssignment, PartsOf57600)>,
//         _end_hint: Option<u32>,
//     ) {
//     }
// }

// pub struct SovereignAccountOf;
// Dummy implementation which converts `TaskId` to `AccountId`.
// impl MaybeConvert<TaskId, AccountId> for SovereignAccountOf {
//     fn maybe_convert(task: TaskId) -> Option<AccountId> {
//         let mut account: [u8; 32] = [0; 32];
//         account[..4].copy_from_slice(&task.to_le_bytes());
//         Some(account.into())
//     }
// }
// impl pallet_broker::Config for Runtime {
//     type RuntimeEvent = RuntimeEvent;
//     type Currency = Balances;
//     type OnRevenue = IntoAuthor;
//     type TimeslicePeriod = ConstU32<2>;
//     type MaxLeasedCores = ConstU32<5>;
//     type MaxReservedCores = ConstU32<5>;
//     type Coretime = CoretimeProvider;
//     type ConvertBalance = traits::Identity;
//     type WeightInfo = ();
//     type PalletId = BrokerPalletId;
//     type AdminOrigin = EnsureRoot<AccountId>;
//     type SovereignAccountOf = SovereignAccountOf;
//     type MaxAutoRenewals = ConstU32<10>;
//     type PriceAdapter = pallet_broker::CenterTargetPrice<Balance>;
// }

parameter_types! {
    pub const MixnetNumCoverToCurrentBlocks: BlockNumber = 3;
    pub const MixnetNumRequestsToCurrentBlocks: BlockNumber = 3;
    pub const MixnetNumCoverToPrevBlocks: BlockNumber = 3;
    pub const MixnetNumRegisterStartSlackBlocks: BlockNumber = 3;
    pub const MixnetNumRegisterEndSlackBlocks: BlockNumber = 3;
    pub const MixnetRegistrationPriority: TransactionPriority = ImOnlineUnsignedPriority::get() - 1;
}

impl pallet_mixnet::Config for Runtime {
    type MaxAuthorities = MaxAuthorities;
    type MaxExternalAddressSize = ConstU32<128>;
    type MaxExternalAddressesPerMixnode = ConstU32<16>;
    type NextSessionRotation = Babe;
    type NumCoverToCurrentBlocks = MixnetNumCoverToCurrentBlocks;
    type NumRequestsToCurrentBlocks = MixnetNumRequestsToCurrentBlocks;
    type NumCoverToPrevBlocks = MixnetNumCoverToPrevBlocks;
    type NumRegisterStartSlackBlocks = MixnetNumRegisterStartSlackBlocks;
    type NumRegisterEndSlackBlocks = MixnetNumRegisterEndSlackBlocks;
    type RegistrationPriority = MixnetRegistrationPriority;
    type MinMixnodes = ConstU32<7>; // Low to allow small testing networks
}

#[cfg(feature = "runtime-benchmarks")]
impl Default for RuntimeParameters {
    fn default() -> Self {
        RuntimeParameters::Storage(dynamic_params::storage::Parameters::BaseDeposit(
            dynamic_params::storage::BaseDeposit,
            Some(1 * DOLLARS),
        ))
    }
}

#[frame_support::runtime]
mod runtime {
    use super::*;

    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask
    )]
    pub struct Runtime;

    #[runtime::pallet_index(0)]
    pub type System = frame_system::Pallet<Runtime>;

    #[runtime::pallet_index(1)]
    pub type Utility = pallet_utility::Pallet<Runtime>;

    #[runtime::pallet_index(2)]
    pub type Babe = pallet_babe::Pallet<Runtime>;

    #[runtime::pallet_index(3)]
    pub type Timestamp = pallet_timestamp::Pallet<Runtime>;

    // Authorship must be before session in order to note author in the correct session and era
    // for im-online and staking.
    #[runtime::pallet_index(4)]
    pub type Authorship = pallet_authorship::Pallet<Runtime>;

    #[runtime::pallet_index(5)]
    pub type Indices = pallet_indices::Pallet<Runtime>;

    #[runtime::pallet_index(6)]
    pub type Balances = pallet_balances::Pallet<Runtime>;

    #[runtime::pallet_index(7)]
    pub type TransactionPayment = pallet_transaction_payment::Pallet<Runtime>;

    #[runtime::pallet_index(9)]
    pub type AssetConversionTxPayment = pallet_asset_conversion_tx_payment::Pallet<Runtime>;

    #[runtime::pallet_index(10)]
    pub type ElectionProviderMultiPhase = pallet_election_provider_multi_phase::Pallet<Runtime>;

    #[runtime::pallet_index(11)]
    pub type Staking = pallet_staking::Pallet<Runtime>;

    #[runtime::pallet_index(12)]
    pub type Session = pallet_session::Pallet<Runtime>;

    // #[runtime::pallet_index(13)]
    // pub type Democracy = pallet_democracy::Pallet<Runtime>;

    // #[runtime::pallet_index(14)]
    // pub type Council = pallet_collective::Pallet<Runtime, Instance1>;

    // #[runtime::pallet_index(15)]
    // pub type TechnicalCommittee = pallet_collective::Pallet<Runtime, Instance2>;

    // #[runtime::pallet_index(16)]
    // pub type Elections = pallet_elections_phragmen::Pallet<Runtime>;

    // #[runtime::pallet_index(17)]
    // pub type TechnicalMembership = pallet_membership::Pallet<Runtime, Instance1>;

    #[runtime::pallet_index(18)]
    pub type Grandpa = pallet_grandpa::Pallet<Runtime>;

    #[runtime::pallet_index(19)]
    pub type Treasury = pallet_treasury::Pallet<Runtime>;

    #[runtime::pallet_index(20)]
    pub type AssetRate = pallet_asset_rate::Pallet<Runtime>;

    // #[runtime::pallet_index(21)]
    // pub type Contracts = pallet_contracts::Pallet<Runtime>;

    #[runtime::pallet_index(22)]
    pub type Sudo = pallet_sudo::Pallet<Runtime>;

    #[runtime::pallet_index(23)]
    pub type ImOnline = pallet_im_online::Pallet<Runtime>;

    #[runtime::pallet_index(24)]
    pub type AuthorityDiscovery = pallet_authority_discovery::Pallet<Runtime>;

    #[runtime::pallet_index(25)]
    pub type Offences = pallet_offences::Pallet<Runtime>;

    #[runtime::pallet_index(26)]
    pub type Historical = pallet_session_historical::Pallet<Runtime>;

    #[runtime::pallet_index(27)]
    pub type RandomnessCollectiveFlip = pallet_insecure_randomness_collective_flip::Pallet<Runtime>;

    #[runtime::pallet_index(28)]
    pub type Identity = pallet_identity::Pallet<Runtime>;

    // #[runtime::pallet_index(29)]
    // pub type Society = pallet_society::Pallet<Runtime>;

    #[runtime::pallet_index(30)]
    pub type Recovery = pallet_recovery::Pallet<Runtime>;

    #[runtime::pallet_index(31)]
    pub type Vesting = pallet_vesting::Pallet<Runtime>;

    #[runtime::pallet_index(32)]
    pub type Scheduler = pallet_scheduler::Pallet<Runtime>;

    // #[runtime::pallet_index(33)]
    // pub type Glutton = pallet_glutton::Pallet<Runtime>;

    #[runtime::pallet_index(34)]
    pub type Preimage = pallet_preimage::Pallet<Runtime>;

    #[runtime::pallet_index(35)]
    pub type Proxy = pallet_proxy::Pallet<Runtime>;

    #[runtime::pallet_index(36)]
    pub type Multisig = pallet_multisig::Pallet<Runtime>;

    #[runtime::pallet_index(37)]
    pub type Bounties = pallet_bounties::Pallet<Runtime>;

    // #[runtime::pallet_index(38)]
    // pub type Tips = pallet_tips::Pallet<Runtime>;

    #[runtime::pallet_index(39)]
    pub type Assets = pallet_assets::Pallet<Runtime, Instance1>;

    #[runtime::pallet_index(40)]
    pub type PoolAssets = pallet_assets::Pallet<Runtime, Instance2>;

    #[runtime::pallet_index(41)]
    pub type Beefy = pallet_beefy::Pallet<Runtime>;

    // MMR leaf construction must be after session in order to have a leaf's next_auth_set
    // refer to block<N>. See issue polkadot-fellows/runtimes#160 for details.
    #[runtime::pallet_index(42)]
    pub type Mmr = pallet_mmr::Pallet<Runtime>;

    #[runtime::pallet_index(43)]
    pub type MmrLeaf = pallet_beefy_mmr::Pallet<Runtime>;

    // #[runtime::pallet_index(44)]
    // pub type Lottery = pallet_lottery::Pallet<Runtime>;

    // #[runtime::pallet_index(45)]
    // pub type Nis = pallet_nis::Pallet<Runtime>;

    // #[runtime::pallet_index(46)]
    // pub type Uniques = pallet_uniques::Pallet<Runtime>;

    // #[runtime::pallet_index(49)]
    // pub type Salary = pallet_salary::Pallet<Runtime>;

    // #[runtime::pallet_index(50)]
    // pub type CoreFellowship = pallet_core_fellowship::Pallet<Runtime>;

    #[runtime::pallet_index(51)]
    pub type TransactionStorage = pallet_transaction_storage::Pallet<Runtime>;

    #[runtime::pallet_index(52)]
    pub type VoterList = pallet_bags_list::Pallet<Runtime, Instance1>;

    // #[runtime::pallet_index(53)]
    // pub type StateTrieMigration = pallet_state_trie_migration::Pallet<Runtime>;

    #[runtime::pallet_index(54)]
    pub type ChildBounties = pallet_child_bounties::Pallet<Runtime>;

    #[runtime::pallet_index(55)]
    pub type Referenda = pallet_referenda::Pallet<Runtime>;

    #[runtime::pallet_index(56)]
    pub type Remark = pallet_remark::Pallet<Runtime>;

    // #[runtime::pallet_index(57)]
    // pub type RootTesting = pallet_root_testing::Pallet<Runtime>;

    #[runtime::pallet_index(58)]
    pub type ConvictionVoting = pallet_conviction_voting::Pallet<Runtime>;

    #[runtime::pallet_index(59)]
    pub type Whitelist = pallet_whitelist::Pallet<Runtime>;

    // #[runtime::pallet_index(60)]
    // pub type AllianceMotion = pallet_collective::Pallet<Runtime, Instance3>;

    // #[runtime::pallet_index(61)]
    // pub type Alliance = pallet_alliance::Pallet<Runtime>;

    #[runtime::pallet_index(62)]
    pub type NominationPools = pallet_nomination_pools::Pallet<Runtime>;

    // #[runtime::pallet_index(63)]
    // pub type RankedPolls = pallet_referenda::Pallet<Runtime, Instance2>;

    // #[runtime::pallet_index(64)]
    // pub type RankedCollective = pallet_ranked_collective::Pallet<Runtime>;

    #[runtime::pallet_index(65)]
    pub type AssetConversion = pallet_asset_conversion::Pallet<Runtime>;

    #[runtime::pallet_index(66)]
    pub type FastUnstake = pallet_fast_unstake::Pallet<Runtime>;

    #[runtime::pallet_index(67)]
    pub type MessageQueue = pallet_message_queue::Pallet<Runtime>;

    // #[runtime::pallet_index(68)]
    // pub type Pov = frame_benchmarking_pallet_pov::Pallet<Runtime>;

    // #[runtime::pallet_index(69)]
    // pub type TxPause = pallet_tx_pause::Pallet<Runtime>;

    // #[runtime::pallet_index(70)]
    // pub type SafeMode = pallet_safe_mode::Pallet<Runtime>;

    #[runtime::pallet_index(71)]
    pub type Statement = pallet_statement::Pallet<Runtime>;

    // #[runtime::pallet_index(72)]
    // pub type MultiBlockMigrations = pallet_migrations::Pallet<Runtime>;

    // #[runtime::pallet_index(73)]
    // pub type Broker = pallet_broker::Pallet<Runtime>;

    #[runtime::pallet_index(75)]
    pub type Mixnet = pallet_mixnet::Pallet<Runtime>;

    // #[runtime::pallet_index(76)]
    // pub type Parameters = pallet_parameters::Pallet<Runtime>;

    #[runtime::pallet_index(77)]
    pub type SkipFeelessPayment = pallet_skip_feeless_payment::Pallet<Runtime>;

    // #[runtime::pallet_index(78)]
    // pub type PalletExampleMbms = pallet_example_mbm::Pallet<Runtime>;

    // #[runtime::pallet_index(79)]
    // pub type AssetConversionMigration = pallet_asset_conversion_ops::Pallet<Runtime>;

    // #[runtime::pallet_index(80)]
    // pub type Revive = pallet_revive::Pallet<Runtime>;

    // OpenGov stuff.
    #[runtime::pallet_index(84)]
    pub type Origins = pallet_custom_origins::Pallet<Runtime>;
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
///
/// When you change this, you **MUST** modify [`sign`] in `bin/node/testing/src/keyring.rs`!
///
/// [`sign`]: <../../testing/src/keyring.rs.html>
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_skip_feeless_payment::SkipCheckIfFeeless<
        Runtime,
        pallet_asset_conversion_tx_payment::ChargeAssetTxPayment<Runtime>,
    >,
    frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,
>;

// All migrations executed on runtime upgrade as a nested tuple of types implementing
// `OnRuntimeUpgrade`.
type Migrations = ();

type EventRecord = frame_system::EventRecord<
    <Runtime as frame_system::Config>::RuntimeEvent,
    <Runtime as frame_system::Config>::Hash,
>;

parameter_types! {
    pub const BeefySetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_beefy::Config for Runtime {
    type BeefyId = BeefyId;
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = ConstU32<0>;
    type MaxSetIdSessionEntries = BeefySetIdSessionEntries;
    type OnNewValidatorSet = MmrLeaf;
    type AncestryHelper = MmrLeaf;
    type WeightInfo = ();
    type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, BeefyId)>>::Proof;
    type EquivocationReportSystem =
        pallet_beefy::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

/// MMR helper types.
mod mmr {
    use super::*;
    pub use pallet_mmr::primitives::*;

    pub type Leaf = <<Runtime as pallet_mmr::Config>::LeafData as LeafDataProvider>::LeafData;
    pub type Hash = <Hashing as sp_runtime::traits::Hash>::Output;
    pub type Hashing = <Runtime as pallet_mmr::Config>::Hashing;
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    frame_benchmarking::define_benchmarks!(
        [frame_benchmarking, BaselineBench::<Runtime>]
        // [frame_benchmarking_pallet_pov, Pov]
        // [pallet_alliance, Alliance]
        // [pallet_assets, Assets]
        [pallet_babe, Babe]
        [pallet_bags_list, VoterList]
        [pallet_balances, Balances]
        [pallet_beefy_mmr, MmrLeaf]
        [pallet_bounties, Bounties]
        // [pallet_broker, Broker]
        [pallet_child_bounties, ChildBounties]
        // [pallet_collective, Council]
        [pallet_conviction_voting, ConvictionVoting]
        // [pallet_contracts, Contracts]
        // [pallet_revive, Revive]
        // [pallet_core_fellowship, CoreFellowship]
        // [tasks_example, TasksExample]
        [pallet_democracy, Democracy]
        [pallet_asset_conversion, AssetConversion]
        [pallet_election_provider_multi_phase, ElectionProviderMultiPhase]
        [pallet_election_provider_support_benchmarking, EPSBench::<Runtime>]
        // [pallet_elections_phragmen, Elections]
        [pallet_fast_unstake, FastUnstake]
        // [pallet_nis, Nis]
        // [pallet_parameters, Parameters]
        [pallet_grandpa, Grandpa]
        [pallet_identity, Identity]
        [pallet_im_online, ImOnline]
        [pallet_indices, Indices]
        // [pallet_lottery, Lottery]
        // [pallet_membership, TechnicalMembership]
        [pallet_message_queue, MessageQueue]
        // [pallet_migrations, MultiBlockMigrations]
        [pallet_mmr, Mmr]
        [pallet_multisig, Multisig]
        [pallet_nomination_pools, NominationPoolsBench::<Runtime>]
        [pallet_offences, OffencesBench::<Runtime>]
        [pallet_preimage, Preimage]
        [pallet_proxy, Proxy]
        // [pallet_ranked_collective, RankedCollective]
        // [pallet_referenda, Referenda]
        [pallet_recovery, Recovery]
        [pallet_remark, Remark]
        // [pallet_salary, Salary]
        [pallet_scheduler, Scheduler]
        // [pallet_glutton, Glutton]
        [pallet_session, SessionBench::<Runtime>]
        // [pallet_society, Society]
        [pallet_staking, Staking]
        // [pallet_state_trie_migration, StateTrieMigration]
        [pallet_sudo, Sudo]
        [frame_system, SystemBench::<Runtime>]
        [pallet_timestamp, Timestamp]
        // [pallet_tips, Tips]
        [pallet_transaction_storage, TransactionStorage]
        [pallet_treasury, Treasury]
        [pallet_asset_rate, AssetRate]
        // [pallet_uniques, Uniques]
        [pallet_utility, Utility]
        [pallet_vesting, Vesting]
        [pallet_whitelist, Whitelist]
        // [pallet_tx_pause, TxPause]
        // [pallet_safe_mode, SafeMode]
        // [pallet_example_mbm, PalletExampleMbms]
        // [pallet_asset_conversion_ops, AssetConversionMigration]
    );
}

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block);
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> alloc::vec::Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_statement_store::runtime_api::ValidateStatement<Block> for Runtime {
        fn validate_statement(
            source: sp_statement_store::runtime_api::StatementSource,
            statement: sp_statement_store::Statement,
        ) -> Result<sp_statement_store::runtime_api::ValidStatement, sp_statement_store::runtime_api::InvalidStatement> {
            Statement::validate_statement(source, statement)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> sp_consensus_grandpa::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: sp_consensus_grandpa::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Grandpa::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }

        fn generate_key_ownership_proof(
            _set_id: sp_consensus_grandpa::SetId,
            authority_id: GrandpaId,
        ) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
            use codec::Encode;

            Historical::prove((sp_consensus_grandpa::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_grandpa::OpaqueKeyOwnershipProof::new)
        }
    }

    impl pallet_nomination_pools_runtime_api::NominationPoolsApi<Block, AccountId, Balance> for Runtime {
        fn pending_rewards(who: AccountId) -> Balance {
            NominationPools::api_pending_rewards(who).unwrap_or_default()
        }

        fn points_to_balance(pool_id: pallet_nomination_pools::PoolId, points: Balance) -> Balance {
            NominationPools::api_points_to_balance(pool_id, points)
        }

        fn balance_to_points(pool_id: pallet_nomination_pools::PoolId, new_funds: Balance) -> Balance {
            NominationPools::api_balance_to_points(pool_id, new_funds)
        }

        fn pool_pending_slash(pool_id: pallet_nomination_pools::PoolId) -> Balance {
            NominationPools::api_pool_pending_slash(pool_id)
        }

        fn member_pending_slash(member: AccountId) -> Balance {
            NominationPools::api_member_pending_slash(member)
        }

        fn pool_needs_delegate_migration(pool_id: pallet_nomination_pools::PoolId) -> bool {
            NominationPools::api_pool_needs_delegate_migration(pool_id)
        }

        fn member_needs_delegate_migration(member: AccountId) -> bool {
            NominationPools::api_member_needs_delegate_migration(member)
        }

        fn member_total_balance(member: AccountId) -> Balance {
            NominationPools::api_member_total_balance(member)
        }

        fn pool_balance(pool_id: pallet_nomination_pools::PoolId) -> Balance {
            NominationPools::api_pool_balance(pool_id)
        }
    }

    impl pallet_staking_runtime_api::StakingApi<Block, Balance, AccountId> for Runtime {
        fn nominations_quota(balance: Balance) -> u32 {
            Staking::api_nominations_quota(balance)
        }

        fn eras_stakers_page_count(era: sp_staking::EraIndex, account: AccountId) -> sp_staking::Page {
            Staking::api_eras_stakers_page_count(era, account)
        }

        fn pending_rewards(era: sp_staking::EraIndex, account: AccountId) -> bool {
            Staking::api_pending_rewards(era, account)
        }
    }

    impl sp_consensus_babe::BabeApi<Block> for Runtime {
        fn configuration() -> sp_consensus_babe::BabeConfiguration {
            let epoch_config = Babe::epoch_config().unwrap_or(BABE_GENESIS_EPOCH_CONFIG);
            sp_consensus_babe::BabeConfiguration {
                slot_duration: Babe::slot_duration(),
                epoch_length: EpochDuration::get(),
                c: epoch_config.c,
                authorities: Babe::authorities().to_vec(),
                randomness: Babe::randomness(),
                allowed_slots: epoch_config.allowed_slots,
            }
        }

        fn current_epoch_start() -> sp_consensus_babe::Slot {
            Babe::current_epoch_start()
        }

        fn current_epoch() -> sp_consensus_babe::Epoch {
            Babe::current_epoch()
        }

        fn next_epoch() -> sp_consensus_babe::Epoch {
            Babe::next_epoch()
        }

        fn generate_key_ownership_proof(
            _slot: sp_consensus_babe::Slot,
            authority_id: sp_consensus_babe::AuthorityId,
        ) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
            use codec::Encode;

            Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
            key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Babe::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }
    }

    impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
        fn authorities() -> Vec<AuthorityDiscoveryId> {
            AuthorityDiscovery::authorities()
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
        Block,
        Balance,
    > for Runtime {
        fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
        for Runtime
    {
        fn query_call_info(call: RuntimeCall, len: u32) -> RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_call_info(call, len)
        }
        fn query_call_fee_details(call: RuntimeCall, len: u32) -> FeeDetails<Balance> {
            TransactionPayment::query_call_fee_details(call, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    #[api_version(5)]
    impl sp_consensus_beefy::BeefyApi<Block, BeefyId> for Runtime {
        fn beefy_genesis() -> Option<BlockNumber> {
            pallet_beefy::GenesisBlock::<Runtime>::get()
        }

        fn validator_set() -> Option<sp_consensus_beefy::ValidatorSet<BeefyId>> {
            Beefy::validator_set()
        }

        fn submit_report_double_voting_unsigned_extrinsic(
            equivocation_proof: sp_consensus_beefy::DoubleVotingProof<
                BlockNumber,
                BeefyId,
                BeefySignature,
            >,
            key_owner_proof: sp_consensus_beefy::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Beefy::submit_unsigned_double_voting_report(
                equivocation_proof,
                key_owner_proof,
            )
        }

        fn submit_report_fork_voting_unsigned_extrinsic(
            equivocation_proof:
                sp_consensus_beefy::ForkVotingProof<
                    <Block as BlockT>::Header,
                    BeefyId,
                    sp_runtime::OpaqueValue
                >,
            key_owner_proof: sp_consensus_beefy::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            Beefy::submit_unsigned_fork_voting_report(
                equivocation_proof.try_into()?,
                key_owner_proof.decode()?,
            )
        }

        fn submit_report_future_block_voting_unsigned_extrinsic(
            equivocation_proof: sp_consensus_beefy::FutureBlockVotingProof<BlockNumber, BeefyId>,
            key_owner_proof: sp_consensus_beefy::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            Beefy::submit_unsigned_future_block_voting_report(
                equivocation_proof,
                key_owner_proof.decode()?,
            )
        }

        fn generate_key_ownership_proof(
            _set_id: sp_consensus_beefy::ValidatorSetId,
            authority_id: BeefyId,
        ) -> Option<sp_consensus_beefy::OpaqueKeyOwnershipProof> {
            Historical::prove((sp_consensus_beefy::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_beefy::OpaqueKeyOwnershipProof::new)
        }

        fn generate_ancestry_proof(
            prev_block_number: BlockNumber,
            best_known_block_number: Option<BlockNumber>,
        ) -> Option<sp_runtime::OpaqueValue> {
            use sp_consensus_beefy::AncestryHelper;

            MmrLeaf::generate_proof(prev_block_number, best_known_block_number)
                .map(|p| p.encode())
                .map(sp_runtime::OpaqueValue::new)
        }
    }

    impl pallet_mmr::primitives::MmrApi<
        Block,
        mmr::Hash,
        BlockNumber,
    > for Runtime {
        fn mmr_root() -> Result<mmr::Hash, mmr::Error> {
            Ok(pallet_mmr::RootHash::<Runtime>::get())
        }

        fn mmr_leaf_count() -> Result<mmr::LeafIndex, mmr::Error> {
            Ok(pallet_mmr::NumberOfLeaves::<Runtime>::get())
        }

        fn generate_proof(
            block_numbers: Vec<BlockNumber>,
            best_known_block_number: Option<BlockNumber>,
        ) -> Result<(Vec<mmr::EncodableOpaqueLeaf>, mmr::LeafProof<mmr::Hash>), mmr::Error> {
            Mmr::generate_proof(block_numbers, best_known_block_number).map(
                |(leaves, proof)| {
                    (
                        leaves
                            .into_iter()
                            .map(|leaf| mmr::EncodableOpaqueLeaf::from_leaf(&leaf))
                            .collect(),
                        proof,
                    )
                },
            )
        }

        fn verify_proof(leaves: Vec<mmr::EncodableOpaqueLeaf>, proof: mmr::LeafProof<mmr::Hash>)
            -> Result<(), mmr::Error>
        {
            let leaves = leaves.into_iter().map(|leaf|
                leaf.into_opaque_leaf()
                .try_decode()
                .ok_or(mmr::Error::Verify)).collect::<Result<Vec<mmr::Leaf>, mmr::Error>>()?;
            Mmr::verify_leaves(leaves, proof)
        }

        fn verify_proof_stateless(
            root: mmr::Hash,
            leaves: Vec<mmr::EncodableOpaqueLeaf>,
            proof: mmr::LeafProof<mmr::Hash>
        ) -> Result<(), mmr::Error> {
            let nodes = leaves.into_iter().map(|leaf|mmr::DataOrHash::Data(leaf.into_opaque_leaf())).collect();
            pallet_mmr::verify_leaves_proof::<mmr::Hashing, _>(root, nodes, proof)
        }
    }

    impl sp_mixnet::runtime_api::MixnetApi<Block> for Runtime {
        fn session_status() -> sp_mixnet::types::SessionStatus {
            Mixnet::session_status()
        }

        fn prev_mixnodes() -> Result<Vec<sp_mixnet::types::Mixnode>, sp_mixnet::types::MixnodesErr> {
            Mixnet::prev_mixnodes()
        }

        fn current_mixnodes() -> Result<Vec<sp_mixnet::types::Mixnode>, sp_mixnet::types::MixnodesErr> {
            Mixnet::current_mixnodes()
        }

        fn maybe_register(session_index: sp_mixnet::types::SessionIndex, mixnode: sp_mixnet::types::Mixnode) -> bool {
            Mixnet::maybe_register(session_index, mixnode)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here. If any of the pre/post migration checks fail, we shall stop
            // right here and right now.
            let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, RuntimeBlockWeights::get().max_block)
        }

        fn execute_block(
            block: Block,
            state_root_check: bool,
            signature_check: bool,
            select: frame_try_runtime::TryStateSelect
        ) -> Weight {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here.
            Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;

            // Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
            // issues. To get around that, we separated the Session benchmarks into its own crate,
            // which is why we need these two lines below.
            use pallet_session_benchmarking::Pallet as SessionBench;
            use pallet_offences_benchmarking::Pallet as OffencesBench;
            use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;
            use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();

            (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch};
            use sp_storage::TrackedStorageKey;

            // Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
            // issues. To get around that, we separated the Session benchmarks into its own crate,
            // which is why we need these two lines below.
            use pallet_session_benchmarking::Pallet as SessionBench;
            use pallet_offences_benchmarking::Pallet as OffencesBench;
            use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;
            use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;

            impl pallet_session_benchmarking::Config for Runtime {}
            impl pallet_offences_benchmarking::Config for Runtime {}
            impl pallet_election_provider_support_benchmarking::Config for Runtime {}
            impl frame_system_benchmarking::Config for Runtime {}
            impl baseline::Config for Runtime {}
            impl pallet_nomination_pools_benchmarking::Config for Runtime {}

            use frame_support::traits::WhitelistedStorageKeys;
            let mut whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

            // Treasury Account
            // TODO: this is manual for now, someday we might be able to use a
            // macro for this particular key
            let treasury_key = frame_system::Account::<Runtime>::hashed_key_for(Treasury::account_id());
            whitelist.push(treasury_key.to_vec().into());

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);
            add_benchmarks!(params, batches);
            Ok(batches)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            get_preset::<RuntimeGenesisConfig>(id, |_| None)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_election_provider_support::NposSolution;
    use frame_system::offchain::CreateSignedTransaction;
    use sp_runtime::UpperOf;

    #[test]
    fn validate_transaction_submitter_bounds() {
        fn is_submit_signed_transaction<T>()
        where
            T: CreateSignedTransaction<RuntimeCall>,
        {
        }

        is_submit_signed_transaction::<Runtime>();
    }

    #[test]
    fn perbill_as_onchain_accuracy() {
        type OnChainAccuracy =
			<<Runtime as pallet_election_provider_multi_phase::MinerConfig>::Solution as NposSolution>::Accuracy;
        let maximum_chain_accuracy: Vec<UpperOf<OnChainAccuracy>> = (0..MaxNominations::get())
            .map(|_| <UpperOf<OnChainAccuracy>>::from(OnChainAccuracy::one().deconstruct()))
            .collect();
        let _: UpperOf<OnChainAccuracy> = maximum_chain_accuracy
            .iter()
            .fold(0, |acc, x| acc.checked_add(*x).unwrap());
    }

    #[test]
    fn call_size() {
        let size = core::mem::size_of::<RuntimeCall>();
        assert!(
            size <= CALL_PARAMS_MAX_SIZE,
            "size of RuntimeCall {} is more than {CALL_PARAMS_MAX_SIZE} bytes.
			 Some calls have too big arguments, use Box to reduce the size of RuntimeCall.
			 If the limit is too strong, maybe consider increase the limit.",
            size,
        );
    }
}
