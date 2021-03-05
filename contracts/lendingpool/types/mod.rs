#[allow(unused)]
mod errors;

use ink_env::AccountId;
use ink_storage::traits::{PackedLayout, SpreadLayout};

pub use errors::*;

/// refer to the whitepaper, section 1.1 basic concepts for a formal description of these properties.
#[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
)]
pub struct ReserveData {
    //stores the reserve configuration
    pub configuration: ReserveConfigurationMap,

    //the liquidity index. Expressed in ray
    pub liquidity_index: u128,
    //variable borrow index. Expressed in ray
    pub variable_borrow_index: u128,
    //the current supply rate. Expressed in ray
    pub current_liquidity_rate: u128,
    //the current variable borrow rate. Expressed in ray
    pub current_variable_borrow_rate: u128,
    //the current stable borrow rate. Expressed in ray
    pub current_stable_borrow_rate: u128,
    pub last_update_timestamp: u64,
    //tokens addresses
    pub atoken_address: AccountId,
    pub stable_debt_token_address: AccountId,
    pub variable_debt_token_address: AccountId,
    //address of the interest rate strategy
    pub interest_rate_strategy_address: AccountId,
    //the id of the reserve. Represents the position in the list of the active reserves
    pub id: u8,
}

#[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
)]
pub struct ReserveConfigurationMap {
    //bit 0-15: LTV
    //bit 16-31: Liq. threshold
    //bit 32-47: Liq. bonus
    //bit 48-55: Decimals
    //bit 56: Reserve is active
    //bit 57: reserve is frozen
    //bit 58: borrowing is enabled
    //bit 59: stable rate borrowing enabled
    //bit 60-63: reserved
    //bit 64-79: reserve factor
    pub is_active: bool,
    pub is_frozen: bool,
}
