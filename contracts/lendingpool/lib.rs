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

    /**
     * @dev Emitted on borrow() and flashLoan() when debt needs to be opened
     * @param reserve The address of the underlying asset being borrowed
     * @param user The address of the user initiating the borrow(), receiving the funds on borrow() or just
     * initiator of the transaction on flashLoan()
     * @param onBehalfOf The address that will be getting the debt
     * @param amount The amount borrowed out
     * @param borrowRateMode The rate mode: 1 for Stable, 2 for Variable
     * @param borrowRate The numeric rate at which the user has borrowed
     * @param referral The referral code used
     **/
    #[ink(event)]
    pub struct Borrow {
        #[ink(topic)]
        reserve: AccountId,
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        on_behalf_of: AccountId,
        #[ink(topic)]
        amount: Balance,
        #[ink(topic)]
        borrow_rate: u128,
    }

    /**
     * @dev Emitted on repay()
     * @param reserve The address of the underlying asset of the reserve
     * @param user The beneficiary of the repayment, getting his debt reduced
     * @param repayer The address of the user initiating the repay(), providing the funds
     * @param amount The amount repaid
     **/
    #[ink(event)]
    pub struct Repay {
        #[ink(topic)]
        reserve: AccountId,
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        repayer: AccountId,
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
            validate_withdraw (
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


        /**
         * @dev Allows users to borrow a specific `amount` of the reserve underlying asset, provided that the borrower
         * already deposited enough collateral, or he was given enough allowance by a credit delegator on the
         * corresponding debt token (StableDebtToken or VariableDebtToken)
         * - E.g. User borrows 100 USDC passing as `onBehalfOf` his own address, receiving the 100 USDC in his wallet
         *   and 100 stable/variable debt tokens, depending on the `interestRateMode`
         * @param asset The address of the underlying asset to borrow
         * @param amount The amount to be borrowed
         * @param interestRateMode The interest rate mode at which the user wants to borrow: 1 for Stable, 2 for Variable
         * @param referralCode Code used to register the integrator originating the operation, for potential rewards.
         *   0 if the action is executed directly by the user, without any middle-man
         * @param onBehalfOf Address of the user who will receive the debt. Should be the address of the borrower itself
         * calling the function if he wants to borrow against his own collateral, or the address of the credit delegator
         * if he has been given credit delegation allowance
         **/
        #[ink(message)]
        pub fun borrow(&mut self, asset: AccountId, amount: Balance, on_behalf_of: AccountId)  {
            let reserve = self.reserves.get(&asset).expect("asset does not exist");
            let user_config = self.user_config.get(&asset).expect("asset does not exist");
            // TODO: oracle asset price * amount / decimal
            let amount_in_dot: u128 = 1;
            validate_borrow (
                asset,
                reserve,
                on_behalf_of,
                amount,
                amount_in_dot,
                MAX_STABLE_RATE_BORROW_SIZE_PERCENT,
                &self.reserves,,
                user_config,
                &self.reserves_list,
                self.reserves_count,
            );

            update_state(reserve);

            let current_stable_rate: u128 = reserve.current_stable_borrow_rate;
            // debt token mint

            // TODO: atoken 
            atoken_contract
            .transfer(sender, amount)
            .expect("aToken burn failed");

            self.env().emit_event(Borrow {
                reserve: asset,
                user: sender,
                on_behalf_of,
                amount,
                current_stable_rate,
            });
        }

    }
}
