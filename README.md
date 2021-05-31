# Substrate-Moloch-V1
Moloch DAO implementation in substrate, it's based on substrate v2.0.1.

## Introduction  
The application implmented [V1 protocol of Moloch](https://github.com/MolochVentures/moloch/blob/master/v1_contracts/).considering the difference between solidity contracts and substrate pallet. We made a small enhancement for the design. As there's no token approval in substrate, we split the submit proposal into 2 steps.  
1. Applicants transfer some tokens(`token_tribute`) to our custody account, which is an sub account of our guildbank.
2. Applicants call on members to propose, just make sure the `token_tribute` aligns  

Once the proposal get processed, the tokens in custody account will be transfered to guildbank if passed, otherwise returned to applicant. Also, the applicant can withdraw the tokens by calling abort. But this operation can only succeed when the proposal is still in abort window.

## Test
For unit test, just run
```
cargo test -p pallet-moloch-v1
````
For an integration/simulation test, please follow our detailed [test guide](./doc/test-guide.md)