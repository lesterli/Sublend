#![cfg_attr(not(feature = "std"), no_std)]

mod types;

use ink_lang as ink;

#[ink::contract]
mod lendingpool {
    use crate::types::*;
    use stoken::SToken;

    use ink_env::call::FromAccountId;
    use ink_storage::collections::HashMap as StorageHashMap;

    /// * @dev Emitted on deposit()
    /// * @param reserve The address of the underlying asset of the reserve
    /// * @param user The address initiating the deposit
    /// * @param onBehalfOf The beneficiary of the deposit, receiving the aTokens
    /// * @param amount The amount deposited
    #[ink(event)]
    pub struct Deposit {
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
        user: AccountId,
        #[ink(topic)]
        repayer: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    #[ink(storage)]
    pub struct Lendingpool {
        // DOT
        reserve: ReserveData,

        users_config: StorageHashMap<AccountId, UserReserveData>,
    }

    impl Lendingpool {
        #[ink(constructor)]
        pub fn new(stoken: AccountId, debt_token: AccountId) -> Self {
            Self {
                reserve: ReserveData {
                    stable_liquidity_rate: 5,
                    stable_borrow_rate: 10,
                    stoken_address: stoken,
                    stable_debt_token_address: debt_token,
                },
                users_config: StorageHashMap::new(),
            }
        }

        /// * @dev Deposits an `amount` of underlying asset into the reserve, receiving in return overlying aTokens.
        /// * - E.g. User deposits 100 USDC and gets in return 100 aUSDC
        /// * @param asset The address of the underlying asset to deposit
        /// * @param amount The amount to be deposited
        /// * @param onBehalfOf The address that will receive the aTokens, same as msg.sender if the user
        /// *   wants to receive them on his own wallet, or a different address if the beneficiary of aTokens
        /// *   is a different wallet
        #[ink(message, payable)]
        pub fn deposit(&mut self, on_behalf_of: Option<AccountId>) {
            let sender = self.env().caller();
            let mut receiver = sender;
            if let Some(behalf) = on_behalf_of {
                receiver = behalf;
            }
            let amount = self.env().transferred_balance();
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);

            let mut stoken: SToken = FromAccountId::from_account_id(self.reserve.stoken_address);

            let entry = self.users_config.entry(receiver);
            let reserve_data = entry.or_insert(Default::default());
            let user_balance = stoken.balance_of(receiver);
            let interval = Self::env().block_timestamp() - reserve_data.last_update_timestamp;
            let interest =
                user_balance * interval as u128 * self.reserve.stable_liquidity_rate / 100;
            reserve_data.cumulated_liquidity_interest += interest;
            reserve_data.last_update_timestamp = Self::env().block_timestamp();

            assert!(stoken.mint(receiver, amount).is_ok());

            self.env().emit_event(Deposit {
                user: sender,
                on_behalf_of: receiver,
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
        pub fn withdraw(&mut self, amount: Balance, to: Option<AccountId>) {
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);
            let sender = self.env().caller();
            let mut receiver = sender;
            if let Some(behalf) = to {
                receiver = behalf;
            }

            let mut stoken: SToken = FromAccountId::from_account_id(self.reserve.stoken_address);
            let user_balance = stoken.balance_of(sender);
            let reserve_data = self
                .users_config
                .get_mut(&sender)
                .expect("user config does not exist");
            let interval = Self::env().block_timestamp() - reserve_data.last_update_timestamp;
            let interest =
                user_balance * interval as u128 * self.reserve.stable_liquidity_rate / 100;
            reserve_data.cumulated_liquidity_interest += interest;
            reserve_data.last_update_timestamp = Self::env().block_timestamp();

            let user_balance = user_balance + reserve_data.cumulated_liquidity_interest;
            assert!(
                amount <= user_balance,
                "{}",
                VL_NOT_ENOUGH_AVAILABLE_USER_BALANCE
            );

            if amount <= reserve_data.cumulated_liquidity_interest {
                reserve_data.cumulated_liquidity_interest -= amount;
            } else {
                let rest = amount - reserve_data.cumulated_liquidity_interest;
                reserve_data.cumulated_liquidity_interest = 0;
                stoken.burn(sender, rest).expect("sToken burn failed");
            }
            self.env()
                .transfer(receiver, amount)
                .expect("transfer failed");

            self.env().emit_event(Withdraw {
                user: sender,
                to: receiver,
                amount,
            });
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
        pub fn borrow(&mut self, amount: Balance, on_behalf_of: AccountId) {
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);
            let sender = self.env().caller();
            let _user_config = self
                .users_config
                .get(&sender)
                .expect("asset does not exist");
            // TODO: oracle asset price * amount / decimal
            let _amount_in_dot: u128 = 1;
            // validate_borrow (
            //     asset,
            //     reserve,
            //     on_behalf_of,
            //     amount,
            //     amount_in_dot,
            //     MAX_STABLE_RATE_BORROW_SIZE_PERCENT,
            //     &self.reserves,,
            //     user_config,
            //     &self.reserves_list,
            //     self.reserves_count,
            // );

            // update_state(reserve);

            let current_stable_rate: u128 = self.reserve.stable_borrow_rate;
            // debt token mint

            // TODO: atoken
            self.env()
                .transfer(sender, amount)
                .expect("aToken burn failed");

            self.env().emit_event(Borrow {
                user: sender,
                on_behalf_of,
                amount,
                borrow_rate: current_stable_rate,
            });
        }
    }
}
