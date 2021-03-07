use super::generic::*;
use crate::types::*;
use ink_env::AccountId;
use ink_storage::collections::HashMap as StorageHashMap;

// * @dev Validates a deposit action
// * @param reserve The reserve object on which the user is depositing
// * @param amount The amount to be deposited
pub fn validate_deposit(reserve: &ReserveData, amount: u128) {
    assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);
    assert!(reserve.configuration.is_active, "{}", VL_NO_ACTIVE_RESERVE);
    assert!(!reserve.configuration.is_active, "{}", VL_RESERVE_FROZEN);
}

// * @dev Validates a withdraw action
// * @param reserveAddress The address of the reserve
// * @param amount The amount to be withdrawn
// * @param userBalance The balance of the user
// * @param reservesData The reserves state
// * @param userConfig The user configuration
// * @param reserves The addresses of the reserves
// * @param reservesCount The number of reserves
// * @param oracle The price oracle
pub fn validate_withdraw(
    reserve_address: AccountId,
    sender: AccountId,
    amount: u128,
    user_balance: u128,
    reserves_data: &StorageHashMap<AccountId, ReserveData>,
    user_config: &UserReserveData,
    reserves: &StorageHashMap<u128, AccountId>,
    reserves_count: u128,
    oracle: AccountId,
) {
    assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);
    assert!(
        amount <= user_balance,
        "{}",
        VL_NOT_ENOUGH_AVAILABLE_USER_BALANCE
    );

    let reserve = reserves_data
        .get(&reserve_address)
        .expect("reserve address does not exist");
    assert!(reserve.configuration.is_active, "{}", VL_NO_ACTIVE_RESERVE);

    assert!(
        balance_decrease_allowed(
            reserve_address,
            sender,
            amount,
            reserves_data,
            user_config,
            reserves,
            reserves_count,
            oracle
        ),
        "{}",
        VL_TRANSFER_NOT_ALLOWED
    );
}
