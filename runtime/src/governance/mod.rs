// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! New governance configurations for the Polkadot runtime.

use super::*;
use frame_support::{parameter_types, traits::EitherOf};
use frame_system::EnsureRootWithSuccess;

mod origins;
pub use origins::{
    pallet_custom_origins, AuctionAdmin, FellowshipAdmin, GeneralAdmin, LeaseAdmin,
    ReferendumCanceller, ReferendumKiller, Spender, StakingAdmin, Treasurer, WhitelistedCaller,
};
mod tracks;
pub use tracks::TracksInfo;

pub enum BodyId {
    /// The only body in its context.
    Unit,
    /// A named body.
    Moniker([u8; 4]),
    /// An indexed body.
    // Index(#[codec(compact)] u32),
    /// The unambiguous executive body (for Polkadot, this would be the Polkadot council).
    Executive,
    /// The unambiguous technical body (for Polkadot, this would be the Technical Committee).
    Technical,
    /// The unambiguous legislative body (for Polkadot, this could be considered the opinion of a
    /// majority of lock-voters).
    Legislative,
    /// The unambiguous judicial body (this doesn't exist on Polkadot, but if it were to get a
    /// "grand oracle", it may be considered as that).
    Judicial,
    /// The unambiguous defense body (for Polkadot, an opinion on the topic given via a public
    /// referendum on the `staking_admin` track).
    Defense,
    /// The unambiguous administration body (for Polkadot, an opinion on the topic given via a
    /// public referendum on the `general_admin` track).
    Administration,
    /// The unambiguous treasury body (for Polkadot, an opinion on the topic given via a public
    /// referendum on the `treasurer` track).
    Treasury,
}

parameter_types! {
    pub const VoteLockingPeriod: BlockNumber = 7 * DAYS;
}

impl pallet_conviction_voting::Config for Runtime {
    type WeightInfo = pallet_conviction_voting::weights::SubstrateWeight<Runtime>;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type VoteLockingPeriod = VoteLockingPeriod;
    type MaxVotes = ConstU32<512>;
    type MaxTurnout =
        frame_support::traits::tokens::currency::ActiveIssuanceOf<Balances, Self::AccountId>;
    type Polls = Referenda;
}

parameter_types! {
    pub const AlarmInterval: BlockNumber = 1;
    pub const SubmissionDeposit: Balance = 1 * DOLLARS;
    pub const UndecidingTimeout: BlockNumber = 14 * DAYS;
}

parameter_types! {
    pub const MaxBalance: Balance = Balance::max_value();
}
pub type TreasurySpender = EitherOf<EnsureRootWithSuccess<AccountId, MaxBalance>, Spender>;

impl origins::pallet_custom_origins::Config for Runtime {}

parameter_types! {
    // Fellows pluralistic body.
    pub const FellowsBodyId: BodyId = BodyId::Technical;
}

impl pallet_whitelist::Config for Runtime {
    type WeightInfo = pallet_whitelist::weights::SubstrateWeight<Runtime>;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type WhitelistOrigin = EnsureRoot<Self::AccountId>;
    type DispatchWhitelistedOrigin = EitherOf<EnsureRoot<Self::AccountId>, WhitelistedCaller>;
    type Preimages = Preimage;
}

impl pallet_referenda::Config for Runtime {
    type WeightInfo = pallet_referenda::weights::SubstrateWeight<Runtime>;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type Scheduler = Scheduler;
    type Currency = Balances;
    type SubmitOrigin = frame_system::EnsureSigned<AccountId>;
    type CancelOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumCanceller>;
    type KillOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumKiller>;
    type Slash = Treasury;
    type Votes = pallet_conviction_voting::VotesOf<Runtime>;
    type Tally = pallet_conviction_voting::TallyOf<Runtime>;
    type SubmissionDeposit = SubmissionDeposit;
    type MaxQueued = ConstU32<100>;
    type UndecidingTimeout = UndecidingTimeout;
    type AlarmInterval = AlarmInterval;
    type Tracks = TracksInfo;
    type Preimages = Preimage;
}
