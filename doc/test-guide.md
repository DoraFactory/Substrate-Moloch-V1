### Build
Run `cargo +nightly build --release`  
Then run `./target/release/node-template --dev`

### Test
1. Navigate to https://polkadot.js.org/apps/#/explorer, and  copy the exmaple to settings->developer
2. Click the lef top icon to open settings and choose the local node, just like below
![alt select-node](images/select-node.png)
3. `summon`, this is to set up initial configuration for your moloch DAO.  
- period_duration, timing unit in seconds, for test you can set it to 120.  
- voting_period_length, number of periods for voting, after that you can not vote anymore.  
- grace_period_length, number of periods for silencing next behind voting, in case any member `ragequit`.  
- abort_window, number of periods to abort, after this window, no permission to abort.  
- proposal_deposit, tokens to deposit when member proposed a proposal.
- proposal_reward, tokens will be distribute to anyone processed a proposal, this will be deducted from proposer's deposit, so it's NOT greater than proposal_deposit

4. `submit_proposal`, propose one member, define the shares and tribute
5. `submit_vote`, only member can vote a YES/NO to a propsoal which is valid
6. `process_proposal`, anyone can process a proposal, after passed, the applicant will become a member, no matter pass or not
the processor will get reward, which is a global constant
7. `abort`, the proposer can abort a proposal
ragequite, the member can rage quite
