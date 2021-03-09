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

/**
 * @dev Validates a borrow action
 * @param asset The address of the asset to borrow
 * @param reserve The reserve state from which the user is borrowing
 * @param userAddress The address of the user
 * @param amount The amount to be borrowed
 * @param amountInETH The amount to be borrowed, in ETH
 * @param interestRateMode The interest rate mode at which the user is borrowing
 * @param maxStableLoanPercent The max amount of the liquidity that can be borrowed at stable rate, in percentage
 * @param reservesData The state of all the reserves
 * @param userConfig The state of the user for the specific reserve
 * @param reserves The addresses of all the active reserves
 * @param oracle The price oracle
 */
pub fn validate_borrow(
    asset: AccountId,
    reserve: &StorageHashMap<AccountId, ReserveData>,
    user_address: AccountId,
    amount: Balance,
    amount_in_dot: u128,
    max_stable_loan_percent: u128,
    reserves_data: &StorageHashMap<AccountId, ReserveData>,
    user_config: &UserReserveData,
    reserves: &StorageHashMap<u128, AccountId>,
    reserves_count: u128,
) {
    // TODO
}

/**
 * @dev Validates a repay action
 * @param reserve The reserve state from which the user is repaying
 * @param amountSent The amount sent for the repayment. Can be an actual value or uint(-1)
 * @param onBehalfOf The address of the user msg.sender is repaying for
 * @param stableDebt The borrow balance of the user
 * @param variableDebt The borrow balance of the user
 */
pub fn validate_repay(
    reserve: &StorageHashMap<AccountId, ReserveData>,
    amount_sent: u128,
    on_behalf_of: AccountId,
    stable_debt: u128,
) {
    // TODO
}
