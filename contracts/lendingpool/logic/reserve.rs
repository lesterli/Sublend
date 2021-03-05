use crate::types::*;
use ink_env::AccountId;

/// * @dev Updates the liquidity cumulative index and the variable borrow index.
/// * @param reserve the reserve object
pub fn update_state(_reserve: &mut ReserveData) {
    unimplemented!()
}

/// * @dev Updates the reserve current stable borrow rate, the current variable borrow rate and the current liquidity rate
/// * @param reserve The address of the reserve to be updated
/// * @param liquidityAdded The amount of liquidity added to the protocol (deposit or repay) in the previous action
/// * @param liquidityTaken The amount of liquidity taken from the protocol (redeem or borrow)
pub fn update_interest_rates(
    _reserve: &mut ReserveData,
    _reserve_address: AccountId,
    _atoken_address: AccountId,
    _liquidity_added: u128,
    _liquidity_taken: u128,
) {
    unimplemented!()
}
