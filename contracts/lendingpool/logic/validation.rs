use crate::types::*;

// * @dev Validates a deposit action
// * @param reserve The reserve object on which the user is depositing
// * @param amount The amount to be deposited
pub fn validate_deposit(reserve: &ReserveData, amount: u128) {
    assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);
    assert!(reserve.configuration.is_active, "{}", VL_NO_ACTIVE_RESERVE);
    assert!(!reserve.configuration.is_active, "{}", VL_RESERVE_FROZEN);
}
