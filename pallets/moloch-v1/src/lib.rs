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
use pallet_timestamp;

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
pub struct Member<AccountId> {
	// the # of shares assigned to this member
	pub shares: u128,
	// highest proposal index # on which the member voted YES
	pub highest_index_yes_vote: u128,
	// always true once a member has been created
	pub exists: bool,
	// the key responsible for submitting proposals and voting - defaults to member address unless updated
	pub delegate_key: AccountId,
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

type MemberOf<T> = Member<<T as frame_system::Trait>::AccountId>;
type ProposalOf<T> = Proposal<<T as frame_system::Trait>::AccountId>;
type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::NegativeImbalance;

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
		TotalShares: u128;
		TotalSharesRequested: u128;
		PeriodDuration: u32;
        VotingPeriodLength: u128;
        GracePeriodLength: u128;
        AbortWindow: u128;
        ProposalDeposit: BalanceOf<T>;
        DilutionBound: u128;
        ProcessingReward: BalanceOf<T>;
		SummonTime get(fn summon_time): T::Moment;
		Members get(fn members): map hasher(blake2_128_concat) T::AccountId  => MemberOf<T>;
		ProposalQueue get(fn proposal_queue): Vec<ProposalOf<T>>
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
		VotingPeriodLengthTooBig,
		DilutionBoundTooBig,
		GracePeriodLengthTooBig,
		AbortWindowTooBig,
		NoEnoughProposalDeposit,
		NoEnoughShares,
		NotMember,
		SharesOverFlow,
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
		pub fn summon(origin, period_duration: u32, voting_period_length: u128,
			          grace_period_length: u128, abort_window: u128, dilution_bound: u128,
					  #[compact] proposal_deposit: BalanceOf<T>, 
					  #[compact]  processing_reward: BalanceOf<T>) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(voting_period_length <= T::MaxVotingPeriodLength::get(), Error::<T>::VotingPeriodLengthTooBig);
			ensure!(grace_period_length <= T::MaxGracePeriodLength::get(), Error::<T>::GracePeriodLengthTooBig);
			ensure!(dilution_bound <= T::MaxDilutionBound::get(), Error::<T>::DilutionBoundTooBig);
			ensure!(abort_window <= voting_period_length, Error::<T>::AbortWindowTooBig);
			ensure!(proposal_deposit >= processing_reward, Error::<T>::NoEnoughProposalDeposit);

			SummonTime::<T>::put(pallet_timestamp::Module::<T>::now());
			PeriodDuration::put(period_duration);
			VotingPeriodLength::put(voting_period_length);
			GracePeriodLength::put(grace_period_length);
			AbortWindow::put(abort_window);
			DilutionBound::put(dilution_bound);

			ProposalDeposit::<T>::put(proposal_deposit);
			ProcessingReward::<T>::put(processing_reward);
			let member = Member {
				shares: 1,
				highest_index_yes_vote: 0,
				exists: true,
				delegate_key: who.clone(),
			};
			Members::<T>::insert(who.clone(), member);
			TotalShares::put(1);
			Self::deposit_event(RawEvent::SummonComplete(who, 1));
			Ok(())
		}

		/// One of the members submit a proposal
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn submit_proposal(origin, applicant: T::AccountId, #[compact] token_tribute: BalanceOf<T>,
			                   shares_requested: u128, details: Vec<u8>) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Members::<T>::contains_key(who.clone()), Error::<T>::NotMember);
			ensure!(Members::<T>::get(who.clone()).shares > 0, Error::<T>::NoEnoughShares);
			let total_requested = TotalSharesRequested::get().checked_add(shares_requested).unwrap();
			let future_shares = TotalShares::get().checked_add(total_requested).unwrap();
			ensure!(future_shares <= T::MaxShares::get(), Error::<T>::SharesOverFlow);
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