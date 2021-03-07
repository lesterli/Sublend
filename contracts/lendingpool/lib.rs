#![cfg_attr(not(feature = "std"), no_std)]

mod logic;
mod types;

use ink_lang as ink;

#[ink::contract]
mod lendingpool {
    use crate::logic::*;
    use crate::types::*;
    use asset::ERC20;
    use atoken::AToken;

    use ink_env::call::FromAccountId;
    use ink_storage::collections::HashMap as StorageHashMap;

    /// * @dev Emitted on deposit()
    /// * @param reserve The address of the underlying asset of the reserve
    /// * @param user The address initiating the deposit
    /// * @param onBehalfOf The beneficiary of the deposit, receiving the aTokens
    /// * @param amount The amount deposited
    /// * @param referral The referral code used
    #[ink(event)]
    pub struct Deposit {
        #[ink(topic)]
        reserve: AccountId,
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        on_behalf_of: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    /// * @dev Emitted on withdraw()
    /// * @param reserve The address of the underlyng asset being withdrawn
    /// * @param user The address initiating the withdrawal, owner of aTokens
    /// * @param to Address that will receive the underlying
    /// * @param amount The amount to be withdrawn
    #[ink(event)]
    pub struct Withdraw {
        #[ink(topic)]
        reserve: AccountId,
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    // * @dev Emitted on setUserUseReserveAsCollateral()
    // * @param reserve The address of the underlying asset of the reserve
    // * @param user The address of the user enabling the usage as collateral
    #[ink(event)]
    pub struct ReserveUsedAsCollateralDisabled {
        reserve: AccountId,
        user: AccountId,
    }

    #[ink(storage)]
    pub struct Lendingpool {
        reserves: StorageHashMap<AccountId, ReserveData>,

        users_config: StorageHashMap<AccountId, UserReserveData>,

        // the list of the available reserves, structured as a mapping for gas savings reasons
        reserves_list: StorageHashMap<u128, AccountId>,

        reserves_count: u128,
    }

    impl Lendingpool {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                reserves: StorageHashMap::new(),
                users_config: StorageHashMap::new(),
                reserves_list: StorageHashMap::new(),
                reserves_count: 0,
            }
        }

        /// * @dev Deposits an `amount` of underlying asset into the reserve, receiving in return overlying aTokens.
        /// * - E.g. User deposits 100 USDC and gets in return 100 aUSDC
        /// * @param asset The address of the underlying asset to deposit
        /// * @param amount The amount to be deposited
        /// * @param onBehalfOf The address that will receive the aTokens, same as msg.sender if the user
        /// *   wants to receive them on his own wallet, or a different address if the beneficiary of aTokens
        /// *   is a different wallet
        /// * @param referralCode Code used to register the integrator originating the operation, for potential rewards.
        /// *   0 if the action is executed directly by the user, without any middle-man
        #[ink(message)]
        pub fn deposit(
            &mut self,
            asset: AccountId,
            amount: Balance,
            on_behalf_of: AccountId,
            _referral_code: u16,
        ) {
            let reserve = self.reserves.get_mut(&asset).expect("asset does not exist");
            validate_deposit(reserve, amount);

            let atoken = reserve.atoken_address;

            update_state(reserve);
            update_interest_rates(reserve, asset, atoken, amount, 0);

            let sender = self.env().caller();

            let mut asset_contract: ERC20 = FromAccountId::from_account_id(asset);
            assert!(asset_contract.transfer_from(sender, atoken, amount).is_ok());

            let mut atoken_contract: AToken = FromAccountId::from_account_id(atoken);
            assert!(atoken_contract.mint(on_behalf_of, amount).is_ok());

            // TODO AToken 接口实现
            // bool isFirstDeposit = IAToken(aToken).mint(onBehalfOf, amount, reserve.liquidityIndex);
            // if (isFirstDeposit) {
            //     _usersConfig[onBehalfOf].setUsingAsCollateral(reserve.id, true);
            //     emit ReserveUsedAsCollateralEnabled(asset, onBehalfOf);
            // }

            self.env().emit_event(Deposit {
                reserve: asset,
                user: sender,
                on_behalf_of,
                amount,
            });
        }

        /// * @dev Withdraws an `amount` of underlying asset from the reserve, burning the equivalent aTokens owned
        /// * E.g. User has 100 aUSDC, calls withdraw() and receives 100 USDC, burning the 100 aUSDC
        /// * @param asset The address of the underlying asset to withdraw
        /// * @param amount The underlying amount to be withdrawn
        /// *   - Send the value type(uint256).max in order to withdraw the whole aToken balance
        /// * @param to Address that will receive the underlying, same as msg.sender if the user
        /// *   wants to receive it on his own wallet, or a different address if the beneficiary is a
        /// *   different wallet
        /// * @return The final amount withdrawn
        #[ink(message)]
        pub fn withdraw(&mut self, asset: AccountId, amount: Balance, to: AccountId) -> Balance {
            let sender = self.env().caller();

            let reserve = self.reserves.get(&asset).expect("asset does not exist");

            let atoken = reserve.atoken_address;
            let mut atoken_contract: AToken = FromAccountId::from_account_id(atoken);
            let user_balance = atoken_contract.balance_of(sender);

            let amount_to_withdraw = amount;
            let user_config = self
                .users_config
                .get_mut(&sender)
                .expect("user config does not exist");
            validate_withdraw(
                asset,
                sender,
                amount_to_withdraw,
                user_balance,
                &self.reserves,
                user_config,
                &self.reserves_list,
                self.reserves_count,
                Default::default(),
            );

            let reserve = self.reserves.get_mut(&asset).unwrap();
            update_state(reserve);
            update_interest_rates(reserve, asset, atoken, 0, amount_to_withdraw);

            if amount_to_withdraw == user_balance {
                user_config.use_as_collateral = false;
                self.env().emit_event(ReserveUsedAsCollateralDisabled {
                    reserve: asset,
                    user: sender,
                });
            }
            // TODO
            atoken_contract
                .burn(sender, amount_to_withdraw)
                .expect("aToken burn failed");

            self.env().emit_event(Withdraw {
                reserve: asset,
                user: sender,
                to,
                amount: amount_to_withdraw,
            });

            amount_to_withdraw
        }
    }
}
