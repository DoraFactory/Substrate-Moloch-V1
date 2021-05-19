use crate::{Module, Config};
use frame_system as system;
use sp_core::H256;
use frame_support::{impl_outer_origin, impl_outer_event, parameter_types, weights::Weight};
use sp_runtime::{
	Perbill, ModuleId,
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

impl_outer_origin! {
	pub enum Origin for Test {}
}

mod moloch_v1 {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum Event for Test {
		system<T>,
		pallet_balances<T>,
		moloch_v1<T>,
	}
}

// Configure a mock runtime to test the pallet.
#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

parameter_types! {
	// for testing, set max to 10**8
    pub const MolochV1ModuleId: ModuleId = ModuleId(*b"py/moloc");
	// HARD-CODED LIMITS
    // These numbers are quite arbitrary; they are small enough to avoid overflows when doing calculations
    // with periods or shares, yet big enough to not limit reasonable use cases.
    pub const MaxVotingPeriodLength: u128 = 100_000_000; // maximum length of voting period
    pub const MaxGracePeriodLength: u128 = 100_000_000; // maximum length of grace period
    pub const MaxDilutionBound: u128 = 100_000_000; // maximum dilution bound
    pub const MaxShares: u128 = 100_000_000; // maximum number of shares that can be minted
}

impl system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Trait for Test {
	type MaxLocks = ();
	type Balance = u64;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

impl pallet_timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ();
    type WeightInfo = ();
}

impl Config for Test {
	type ModuleId = MolochV1ModuleId;
    // The Balances pallet implements the ReservableCurrency trait.
    // https://substrate.dev/rustdocs/v2.0.0/pallet_balances/index.html#implementations-2
    type Currency = pallet_balances::Module<Test>;

    // No action is taken when deposits are forfeited.
    type Slashed = ();

	type Event = Event;

	type AdminOrigin = frame_system::EnsureRoot<u64>;

	// maximum length of voting period
	type MaxVotingPeriodLength = MaxVotingPeriodLength;

	// maximum length of grace period
	type MaxGracePeriodLength = MaxGracePeriodLength;

	// maximum dilution bound
	type MaxDilutionBound = MaxDilutionBound;

	// maximum number of shares
	type MaxShares = MaxShares;

}

pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type MolochV1 = Module<Test>;
pub type Timestamp = pallet_timestamp::Module<Test>;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	// system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
	pallet_balances::GenesisConfig::<Test>{
		// Total issuance will be 1000 with internal account initialized at ED.
		balances: vec![(0, 1000), (1, 2000), (2, 3000), (3, 4000)],
	}.assimilate_storage(&mut t).unwrap();
	system::GenesisConfig::default().assimilate_storage::<Test>(&mut t).unwrap();
	t.into()
}