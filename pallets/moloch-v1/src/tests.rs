use crate::{Error, mock::*};
use frame_support::{assert_ok, assert_noop};
use super::RawEvent;
use sp_std::convert::{TryInto};


fn last_event() -> RawEvent<u64, u64> {
	System::events().into_iter().map(|r| r.event)
		.filter_map(|e| {
			if let Event::moloch_v1(inner) = e { Some(inner) } else { None }
		})
		.last()
		.unwrap()
}

/// A helper function to summon moloch for each test case
fn summon_with(initial_member: u64) {
	// in seconds
	let period_duration = 10;
	let voting_period_length = 2;
	let grace_period_length = 2;
	let abort_window = 1;
	let dilution_bound = 1;
	let proposal_deposit = 100;
	let processing_reward = 50;

	let _ = MolochV1::summon(Origin::signed(initial_member), period_duration, voting_period_length, grace_period_length, abort_window, dilution_bound, proposal_deposit, processing_reward);
}

#[test]
fn summon_works() {
	new_test_ext().execute_with(|| {
		// in seconds
		let period_duration = 10;
		let voting_period_length = 2;
		let grace_period_length = 2;
		let abort_window = 1;
		let dilution_bound = 1;
		let proposal_deposit = 100;
		let processing_reward = 50;

		assert_ok!(MolochV1::summon(Origin::signed(1), period_duration, voting_period_length, grace_period_length, abort_window, dilution_bound, proposal_deposit, processing_reward));
		// check the constants
		assert_eq!(MolochV1::period_duration(), period_duration);
		assert_eq!(MolochV1::voting_period_length(), voting_period_length);
		assert_eq!(MolochV1::grace_period_length(), grace_period_length);
		assert_eq!(MolochV1::abort_window(), abort_window);
		assert_eq!(MolochV1::dilution_bound(), dilution_bound);
		assert_eq!(MolochV1::proposal_deposit(), proposal_deposit);
		assert_eq!(MolochV1::processing_reward(), processing_reward);

		// check the shares and member
		assert_eq!(MolochV1::totoal_shares(), 1);
		assert_eq!(MolochV1::members(1).exists, true);
	});
}

#[test]
fn submit_proposal_works() {
	new_test_ext().execute_with(|| {
		// IMPORTANT, event won't emit in block 0
		System::set_block_number(1);
		let initial_member = 1;
		summon_with(initial_member);

		// failed when member propose for applicant who did not deposit in custody account
		let token_tribute = 50;
		let shares_requested = 5;
		let applicant = 2;
		let detail = b"test_proposal".to_vec();
		assert_noop!(
			MolochV1::submit_proposal(Origin::signed(1), applicant, token_tribute, shares_requested, detail.clone()),
			Error::<Test>::NoCustodyFound
		);

		// deposit custody and resubmit
		assert_ok!(MolochV1::custody(Origin::signed(applicant), token_tribute));
		assert_ok!(MolochV1::submit_proposal(Origin::signed(1), applicant, token_tribute, shares_requested, detail));
		assert_eq!(last_event(), RawEvent::SubmitProposal(0, 1, 1, applicant, token_tribute.into(), shares_requested));
	});
}

#[test]
fn add_member_works() {
	new_test_ext().execute_with(|| {
		// IMPORTANT, event won't emit in block 0
		System::set_block_number(1);
		let initial_member = 1;
		summon_with(initial_member);

		// failed when member propose for applicant who did not deposit in custody account
		let token_tribute = 50;
		let shares_requested = 5;
		let applicant = 2;
		let detail = b"test_proposal".to_vec();
		// deposit custody and submit
		assert_ok!(MolochV1::custody(Origin::signed(applicant), token_tribute));
		assert_ok!(MolochV1::submit_proposal(Origin::signed(1), applicant, token_tribute, shares_requested, detail));

		// set the timestamp to make voting period effect
		let now = Timestamp::now();
		let period_duration = TryInto::<u64>::try_into(MolochV1::period_duration() * 1000 * 2).ok().unwrap();
		Timestamp::set_timestamp(now + period_duration);

		// vote yes
		assert_ok!(MolochV1::submit_vote(Origin::signed(1), 0, 1));
		 
		// pass grace period
		Timestamp::set_timestamp(now + period_duration * 4);
		let processor = 3;
		let balance_before = Balances::free_balance(processor);
		let processing_reward = MolochV1::processing_reward();
		assert_ok!(MolochV1::process_proposal(Origin::signed(processor), 0));
		// make sure the processor get rewarded
		assert_eq!(Balances::free_balance(processor), processing_reward + balance_before);

		// check the applicant has become a member
		assert_eq!(MolochV1::members(applicant).exists, true);
	});
}