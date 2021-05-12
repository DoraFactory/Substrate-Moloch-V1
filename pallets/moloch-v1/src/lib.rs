#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame
/// debug guide https://substrate.dev/recipes/runtime-printing.html
use frame_support::{
	decl_module, decl_storage, decl_event, decl_error, dispatch, debug, ensure,
	traits::{Currency, EnsureOrigin, ReservableCurrency, OnUnbalanced, Get, ExistenceRequirement::{KeepAlive}},
};
use sp_runtime::{ModuleId, traits::{ Hash, AccountIdConversion}};
use frame_support::codec::{Encode, Decode};
use frame_system::{ensure_signed};
use sp_std::{vec, vec::Vec, convert::{TryInto}};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Clone, PartialEq)]
pub enum Vote{
	// default value, counted as abstention
	Null,
	Yes,
	No
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Member<AccountId, Hash> {
	// the # of shares assigned to this member
	pub shares: u128,
	// highest proposal index # on which the member voted YES
	pub highest_index_yes_vote: u128,
	// always true once a member has been created
	pub exists: bool,
	// the key responsible for submitting proposals and voting - defaults to member address unless updated
	pub delegate_key: Hash,
	// memeber account
	pub account: AccountId,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Proposal<AccountId> {
	// the member who submitted the proposal
	pub proposer: AccountId,
	// the applicant who wishes to become a member - this key will be used for withdrawals
	pub applicant: AccountId,
	// the # of shares the applicant is requesting
	pub shares_requested: u128,
	// the period in which voting can start for this proposal
	pub starting_period: u128,
	// the total number of YES votes for this proposal
	pub yes_votes: u128,
	// the total number of NO votes for this proposal
	pub no_votes: u128,
	// true only if the proposal has been processed
	pub processed: bool,
	// true only if the proposal passed
	pub did_pass: bool,
	// true only if applicant calls "abort" fn before end of voting period
	pub aborted: bool,
	// amount of tokens offered as tribute
	pub token_tribute: u128,
	// proposal details - Must be ascii chars, limited length
	pub details: Vec<u8>,
	// the maximum # of total shares encountered at a yes vote on this proposal
	pub max_total_shared_at_yes: u128,
	// mapping (address => Vote) votesByMember; // the votes on this proposal by each member
}

type MemberOf<T> = Member<<T as frame_system::Trait>::AccountId, <T as frame_system::Trait>::Hash>;
type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::NegativeImbalance;
use pallet_timestamp;
/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: pallet_timestamp::Trait + frame_system::Trait {
	// used to generate sovereign account
	// refer: https://github.com/paritytech/substrate/blob/743accbe3256de2fc615adcaa3ab03ebdbbb4dbd/frame/treasury/src/lib.rs#L92
	type ModuleId: Get<ModuleId>;

	/// Origin from which admin must come.
	type AdminOrigin: EnsureOrigin<Self::Origin>;

    // The runtime must supply this pallet with an Event type that satisfies the pallet's requirements.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// The currency trait.
	type Currency: ReservableCurrency<Self::AccountId>;

	/// What to do with slashed funds.
	type Slashed: OnUnbalanced<NegativeImbalanceOf<Self>>;

	// maximum length of voting period
	type MaxVotingPeriodLength: Get<u128>;

	// maximum length of grace period
	type MaxGracePeriodLength: Get<u128>;

	// maximum dilution bound
	type MaxDilutionBound: Get<u128>;

	// maximum number of shares
	type MaxShares: Get<u128>;

	
}

// The pallet's runtime storage items.
// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
	// A unique name is used to ensure that the pallet's storage items are isolated.
	// This name may be updated, but each pallet in the runtime must use a unique name.
	trait Store for Module<T: Config> as MolochV1 {
		// Learn more about declaring storage items:
		// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
		// Map, each round start with an id => bool 
		TotalShares: u32;
		TotalSharesRequested: u32;
		PeriodDuration: u32;
        VotingPeriodLength: u32;
        GracePeriodLength: u32;
        AbortWindow: u32;
        ProposalDeposit: u32;
        DilutionBound: u32;
        ProcessingReward: u32;
		SummonTime get(fn summon_time): T::Moment;
	}
	add_extra_genesis {
		build(|_config| {
			// Create pallet's internal account
			let _ = T::Currency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::Currency::minimum_balance(),
			);
		});
	}
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId, Hash =  <T as frame_system::Trait>::Hash, {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [proposalIndex, delegateKey, memberAddress, applicant, tokenTribute, sharesRequested] 
		SubmitProposal(u128, Hash, AccountId, AccountId, u128, u128),
		/// parameters. [proposalIndex, delegateKey, memberAddress, uintVote]
		SubmitVote(u128, Hash, AccountId, u8),
		/// parameters. [proposalIndex, applicant, memberAddress, tokenTribute, sharesRequested, didPass]
		ProcessProposal(u128, AccountId, AccountId, u128, u128, bool),
		/// parameters. [memberAddress, sharesToBurn]
		Ragequit(AccountId, u128),
		/// parameters. [proposalIndex, applicantAddress]
		Abort(u128, AccountId),
		/// parameters. [memberAddress, newDelegateKey]
		UpdateDelegateKey(AccountId, Hash),
		/// parameters. [summoner, shares]
		SummonComplete(AccountId, u128),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Config> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}
}

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;
		const MaxVotingPeriodLength: u128 = T::MaxVotingPeriodLength::get();
		const MaxGracePeriodLength: u128 = T::MaxGracePeriodLength::get();
		const MaxDilutionBound: u128 = T::MaxDilutionBound::get();
		const MaxShares: u128 = T::MaxShares::get();
		
		/// Summon a group or orgnization
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn summon(origin) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			let now = pallet_timestamp::Module::<T>::now();
			SummonTime::<T>::put(now);
			debug::info!("======>>>>>>>>Request sent by: {:?}", now);
			Ok(())
		}

		/// One of the members submit a proposal
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn submit_proposal(origin) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			Ok(())
		}

		/// One of the members submit a vote
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn submit_vote(origin) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			Ok(())
		}

		/// Process a proposal in queue
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn process_proposal(origin) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			Ok(())
		}

		/// Member rage quit
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn ragequit(origin) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			Ok(())
		}

		/// Member rage quit
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn abort(origin) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			Ok(())
		}

	}
}

impl<T: Config> Module<T> {
	// Add public immutables and private mutables.

	/// refer https://github.com/paritytech/substrate/blob/743accbe3256de2fc615adcaa3ab03ebdbbb4dbd/frame/treasury/src/lib.rs#L351
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	pub fn u128_to_balance(cost: u128) -> BalanceOf<T> {
		TryInto::<BalanceOf::<T>>::try_into(cost).ok().unwrap()
	}

	pub fn balance_to_u128(balance: BalanceOf<T>) -> u128 {
		TryInto::<u128>::try_into(balance).ok().unwrap()
	}
}