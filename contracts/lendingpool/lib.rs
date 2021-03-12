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
     * @dev Emitted on borrow() when debt needs to be opened
     * @param user The address of the user initiating the borrow(), receiving the funds on borrow() 
     * @param onBehalfOf The address that will be getting the debt
     * @param amount The amount borrowed out
     * @param borrowRate The numeric rate at which the user has borrowed
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
     * @param receiver The beneficiary of the repayment, getting his debt reduced
     * @param repayer The address of the user initiating the repay(), providing the funds
     * @param amount The amount repaid
     **/
    #[ink(event)]
    pub struct Repay {
        #[ink(topic)]
        receiver: AccountId,
        #[ink(topic)]
        repayer: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    #[ink(storage)]
    pub struct Lendingpool {
        // DOT
        reserve: ReserveData,

        users_data: StorageHashMap<AccountId, UserReserveData>,
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
                users_data: StorageHashMap::new(),
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

            let entry = self.users_data.entry(receiver);
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
                .users_data
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
         * corresponding debt token
         * - E.g. User borrows 100 USDC passing as `onBehalfOf` his own address, receiving the 100 USDC in his wallet
         *   and 100 stable debt tokens
         * @param amount The amount to be borrowed
         * @param onBehalfOf Address of the user who will receive the debt. Should be the address of the borrower itself
         * calling the function if he wants to borrow against his own collateral, or the address of the credit delegator
         * if he has been given credit delegation allowance
         **/
        #[ink(message, payable)]
        pub fn borrow(&mut self, amount: Balance, on_behalf_of: AccountId) {
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);

            let sender = self.env().caller();
            let receiver = on_behalf_of;

            let mut stoken: SToken = FromAccountId::from_account_id(self.reserve.stoken_address);
            // credit delegation allowances
            let credit_balance = stoken.allowance(receiver, sender);
            assert!(
                amount <= credit_balance,
                "{}",
                VL_NOT_ENOUGH_AVAILABLE_USER_BALANCE
            );

            let mut dtoken: DebtToken = FromAccountId::from_account_id(self.reserve.stable_debt_token_address);
            // stoken - debetoken
            let liquidation_threshold = stoken.balance_of(receiver) * 0.75 - dtoken.balance_of(receiver);
            assert!(
                amount <= liquidation_threshold,
                "{}",
                LP_NOT_ENOUGH_LIQUIDITY_TO_BORROW
            );
            
            let entry = self.users_data.entry(receiver);
            let reserve_data = entry.or_insert(Default::default());
            let interval = Self::env().block_timestamp() - reserve_data.last_update_timestamp;
            let interest = amount * interval as u128 * self.reserve.stable_borrow_rate / 100;
            reserve_data.cumulated_stable_borrow_interest += interest;
            reserve_data.last_update_timestamp = Self::env().block_timestamp();
            // mint debt token to receiver
            assert!(dtoken.mint(receiver, amount).is_ok());

            // transfer reserve asset to sender
            self.env()
                .transfer(sender, amount)
                .expect("transfer failed");
            
            self.env().emit_event(Borrow {
                user: sender,
                on_behalf_of,
                amount,
                borrow_rate: self.reserve.stable_borrow_rate,
            });
        }


        /**
         * @notice Repays a borrowed `amount` on a specific reserve, burning the equivalent debt tokens owned
         * - E.g. User repays 100 USDC, burning 100 stable debt tokens of the `onBehalfOf` address
         * @param amount The amount to repay
         * - Send the value type(uint256).max in order to repay the whole debt for `asset` on the specific `debtMode`
         * @param onBehalfOf Address of the user who will get his debt reduced/removed. Should be the address of the
         * user calling the function if he wants to reduce/remove his own debt, or the address of any other
         * other borrower whose debt should be removed
         * @return The final amount repaid
         **/
        #[ink(message)]
        pub fn repay(&mut self, amount: Balance, on_behalf_of: AccountId) {
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);

            let sender = self.env().caller();

            let mut dtoken: DebtToken = FromAccountId::from_account_id(self.reserve.stable_debt_token_address);
            // burn debt token for receiver
            assert!(dtoken.burn(receiver, amount).is_ok());

            // transfer reserve asset to receiver
            self.env()
                .transfer(receiver, amount)
                .expect("transfer failed");

            self.env().emit_event(Borrow {
                receiver: on_behalf_of,
                repayer: sender,
                amount,
            });

        }

    }
}
