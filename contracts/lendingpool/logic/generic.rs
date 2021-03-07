use crate::types::*;
use ink_env::AccountId;
use ink_storage::collections::HashMap as StorageHashMap;

// * @dev Checks if a specific balance decrease is allowed
// * (i.e. doesn't bring the user borrow position health factor under HEALTH_FACTOR_LIQUIDATION_THRESHOLD)
// * @param asset The address of the underlying asset of the reserve
// * @param user The address of the user
// * @param amount The amount to decrease
// * @param reservesData The data of all the reserves
// * @param userConfig The user configuration
// * @param reserves The list of all the active reserves
// * @param oracle The address of the oracle contract
// * @return true if the decrease of the balance is allowed
pub fn balance_decrease_allowed(
    _asset: AccountId,
    _user: AccountId,
    _amount: u128,
    _reserves_data: &StorageHashMap<AccountId, ReserveData>,
    _user_config: &UserReserveData,
    _reserves: &StorageHashMap<u128, AccountId>,
    _reserves_count: u128,
    _oracle: AccountId,
) -> bool {
    unimplemented!()
}
