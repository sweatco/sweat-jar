## Project Overview

The DeFi Jars contract allows users to stake their NEP-141 fungible tokens and accrue interest from them. It is primarily designed to work with $SWEAT, but in general, it can be configured to work with any NEP-141 FT.

The contract enables a contract administrator to register Products describing the terms of deposits. Users can then stake FTs into Jars, which are deposits based on the specified Products. The contract also provides the ability to restrict the creation of Jars for specific Products based on external conditions and involves third-party services.

## Terminology

The contract operates with the following entities:

- **Earnings Rate (ER):** This represents the rate, updated per minute, at which a user earns interest on their staked $SWEAT.
- **Product:** A Product defines a set of rules and terms for deposits (Jars).
- **Fixed Product:** A Product with a lockup period into which users can stake $SWEAT. Once the lockup period has matured, the staked amount becomes available for re-staking (with the same ER and period) or for unstaking (withdrawn into the user’s liquid balance). However, the user stops earning $SWEAT (ER) until it is re-staked.
- **Flexible Product:** A Product with no lockup period.
- **Growth Jar:** This is a deposit that follows the terms of a Product. It includes the principal amount and earned interest.
- **Fixed Jar:** A Jar that follows the rules of a Fixed Product.
- **Flexible Jar:** A Jar that follows the rules of a Flexible Product.

The contract allows users to perform the following actions:

- **Grow:** This term is used when referring to earning $SWEAT by depositing it into a Growth Jar.
- **Stake:** This is the act of sending funds to the contract under specific terms defined by the chosen Product.
- **Unstake:** This is the act of requesting the smart contract to release the exact amount of staked funds back to the original staker's address.
- **Restake:** This refers to the act of re-enacting a previous “stake” action under the same terms.
- **Claim:** This is the act of a user requesting the smart contract to release the accrued earnings from applied ERs on all or selected Jars containing funds.

## 1. Functional Requirements

### 1.1. Roles

The DeFi Jars contract defines three roles:

- **Admin:** The Admin role manages Products. It can register new Products, enable or disable existing Products, and change verifying (public) keys.
- **User:** Users can create Jars by staking tokens, claim accrued interest, unstake their funds, and restake mature Jars.
- **Oracle:** While not directly represented in the contract, the Oracle role can issue signatures to restrict Users' access to specific Products based on conditions that cannot be evaluated within the contract itself.  

### 1.2. Features
### 1.3. Use cases