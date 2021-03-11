#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod stabledebttoken {

    use incentivizederc20::Erc20;
    use ink_prelude::string::String;
    use ink_storage::{collections::HashMap as StorageHashMap, lazy::Lazy};

    ///TODO lendingpool
    #[ink(storage)]
    pub struct Stabledebttoken {
        ///underlying asset address
        underlying_asset_address: AccountId,

        borrowallowances: StorageHashMap<(AccountId, AccountId), Balance>,

        insdebttoken: Lazy<Erc20>,

        avgstablerate: u32,

        timestamps: StorageHashMap<AccountId, Timestamp>,

        userstablerate: StorageHashMap<AccountId, u32>,

        totalsupplytimestamp: Timestamp,
        //TODO need a lendingpool instance
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
    )]
    pub struct MintLocalVars {
        previoussupply: Balance,
        next: Balance,
        amountinray: u128,
        newstablerate: u32,
        currentavgstablerate: u32,
    }

    impl Stabledebttoken {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(
            underaddress: AccountId,
            incentivizede_contract_hash: Hash,
            name: Option<String>,
            symbol: Option<String>,
            version: u32,
        ) -> Self {
            let salt = version.to_le_bytes();
            let insdebttoken = Erc20::new(0, name, symbol, 18)
                .endowment(0)
                .code_hash(incentivizede_contract_hash)
                .salt_bytes(salt)
                .params()
                .instantiate()
                .expect("debttoken instantiate failed");

            Self {
                insdebttoken: Lazy::new(insdebttoken),
                borrowallowances: StorageHashMap::new(),
                timestamps: StorageHashMap::new(),
                userstablerate: StorageHashMap::new(),
                avgstablerate: 0,
                totalsupplytimestamp: 0,
                underlying_asset_address: underaddress,
            }
        }

        // * @dev delegates borrowing power to a user on the specific debt token
        // * @param delegatee the address receiving the delegated borrowing power
        // * @param amount the maximum amount being delegated. Delegation will still
        // * respect the liquidation constraints (even if delegated, a delegatee cannot
        // * force a delegator HF to go below 1)
        #[ink(message)]
        pub fn approvedelegation(&mut self, delegatee: AccountId, amount: Balance) {
            self.borrowallowances
                .insert((self.env().caller(), delegatee), amount);
        }

        // * @dev returns the borrow allowance of the user
        // * @param fromUser The user to giving allowance
        // * @param toUser The user to give allowance to
        // * @return the current allowance of toUser
        #[ink(message)]
        pub fn borrowallowance(&self, fromuser: AccountId, touser: AccountId) -> Balance {
            self.borrowallowances
                .get(&(fromuser, touser))
                .copied()
                .unwrap_or(0)
        }

        #[ink(message)]
        pub fn decreaseborrowallowance(
            &mut self,
            delegator: AccountId,
            delegatee: AccountId,
            amount: Balance,
        ) {
            let cur_allowance = self
                .borrowallowances
                .get(&(delegator, delegatee))
                .copied()
                .unwrap_or(0);
            let mut new_amount = 0;
            if cur_allowance > amount {
                new_amount = cur_allowance - amount;
            }
            self.borrowallowances
                .insert((self.env().caller(), delegatee), new_amount);
        }

        // * @dev Returns the average stable rate across all the stable rate debt
        // * @return the average stable rate
        #[ink(message)]
        pub fn getavgstablerate(&self) -> u32 {
            self.avgstablerate
        }

        //    * @dev Returns the timestamp of the last user action
        //    * @return The last update timestamp
        #[ink(message)]
        pub fn getuserlastupdated(&self, user: AccountId) -> Timestamp {
            self.timestamps.get(&user).copied().unwrap_or(0)
        }

        // * @dev Returns the stable rate of the user
        // * @param user The address of the user
        // * @return The stable rate of user
        #[ink(message)]
        pub fn getuserstalerate(&self, user: AccountId) -> u32 {
            self.userstablerate.get(&user).copied().unwrap_or(0)
        }

        // * @dev Mints debt token to the `onBehalfOf` address.
        // * -  Only callable by the LendingPool
        // * - The resulting rate is the weighted average between the rate of the new debt
        // * and the rate of the previous debt
        // * @param user The address receiving the borrowed underlying, being the delegatee in case
        // * of credit delegate, or same as `onBehalfOf` otherwise
        // * @param onBehalfOf The address receiving the debt tokens
        // * @param amount The amount of debt tokens to mint
        // * @param rate The rate of the debt being minted
        #[ink(message)]
        pub fn mint(
            &mut self,
            user: AccountId,
            onbehalfof: AccountId,
            amount: Balance,
            rate: u32,
        ) -> bool {
            if user != onbehalfof {
                self.decreaseborrowallowance(onbehalfof, user, amount);
            }
            let (_, currbalance, balanceincrease) = self.caculatebalanceincrease(onbehalfof);
            let previoussupply = self.totalsupply();
            let currentavgstablerate = self.avgstablerate;
            let nextsupply = previoussupply + amount;
            self.insdebttoken.total_supply = Lazy::new(nextsupply);
            //wadtoray should be called
            let amountinray: u32 = amount as u32;
            let mut newstablerate =
                self.userstablerate.get(&onbehalfof).copied().unwrap_or(0) * currbalance as u32;
            newstablerate = (newstablerate.into() + (amountinray * rate.into()))
                / (currbalance + amount) as u128;
            self.userstablerate.insert(onbehalfof, newstablerate);
            self.totalsupplytimestamp = self.timestamps.get(&onbehalfof).copied().unwrap_or(0);
            let mut newcurstablerate = currentavgstablerate as u128 * previoussupply as u128;
            newcurstablerate = (newcurstablerate + (rate * amountinray) as u128) / nextsupply;
            self._mint(onbehalfof, amount + balanceincrease, previoussupply);
            currbalance == 0
        }

        //    * @dev Burns debt of `user`
        //    * @param user The address of the user getting his debt burned
        //    * @param amount The amount of debt tokens getting burned
        #[ink(message)]
        pub fn burn(&mut self, user: AccountId, amount: Balance) {
            //TODO need onlylendingpool
            let (_, currentbalance, balanceincrease) = self.caculatebalanceincrease(user);
            let previoussupply = self.totalsupply();
            let mut newavgstablerate = 0;
            let mut nextsupply = 0;
            let userstablerate = self.userstablerate.get(&user).copied().unwrap_or(0);
            // Since the total supply and each single user debt accrue separately,
            // there might be accumulation errors so that the last borrower repaying
            // mght actually try to repay more than the available debt supply.
            // In this case we simply set the total supply and the avg stable rate to 0
            if previoussupply <= amount {
                self.avgstablerate = 0;
                self.insdebttoken.totalsupply = 0;
            } else {
                nextsupply = previoussupply - amount;
                self.insdebttoken.totalsupply = nextsupply;
                let firsterm = self.avgstablerate as u128 * previoussupply;
                let secondterm = userstablerate as u128 * amount;
                // For the same reason described above, when the last user is repaying it might
                // happen that user rate * user balance > avg rate * total supply. In that case,
                // we simply set the avg rate to 0
                if secondterm >= firsterm {
                    newavgstablerate = 0;
                    self.insdebttoken.totalsupply = newavgstablerate;
                    self.avgstablerate = 0;
                } else {
                    newavgstablerate = (firsterm - secondterm) / nextsupply;
                    self.avgstablerate = newavgstablerate as u32;
                }
            }

            if amount == currentbalance {
                self.userstablerate.insert(user, 0);
                self.timestamps.insert(user, 0);
            } else {
                self.timestamps.insert(user, self.env().block_timestamp());
            }
            self.totalsupplytimestamp = self.env().block_timestamp();
            if balanceincrease > amount {
                let amounttomint = balanceincrease - amount;
                self._mint(user, amounttomint, previoussupply);
            } else {
                let amounttoburn = amount - balanceincrease;
                self._burn(user, amounttoburn, previoussupply);
            }
        }

        // * @dev Returns the principal and total supply, the average borrow rate and the last supply update timestamp
        #[ink(message)]
        pub fn getsupplydata(&self) -> (Balance, Balance, u32, Timestamp) {
            let avgrate = self.avgstablerate;
            (
                self.insdebttoken.total_supply(),
                self.calctotalsupply(avgrate),
                avgrate,
                self.totalsupplytimestamp,
            )
        }

        //* @dev Returns the the total supply and the average stable rate
        #[ink(message)]
        pub fn gettotalsupplyandavgrate(&self) -> (Balance, u32) {
            let avgrate = self.avgstablerate;
            (self.calctotalsupply(avgrate), avgrate)
        }

        //* @dev Returns the timestamp at which the total supply was updated
        #[ink(message)]
        pub fn gettotalsupplylastupdated(&self) -> Timestamp {
            self.totalsupplytimestamp
        }

        //    * @dev Returns the principal debt balance of the user from
        //    * @param user The user's address
        //    * @return The debt balance of the user since the last burn/mint action
        #[ink(message)]
        pub fn principalbalanceof(&self, user: AccountId) -> Balance {
            self.insdebttoken.balance_of(user)
        }

        // * @dev returns the total supply
        fn totalsupply(&self) -> Balance {
            self.calctotalsupply(self.avgstablerate)
        }

        //    * @dev Calculates the current user debt balance
        //    * @return The accumulated debt of the user
        pub fn balanceof(&mut self, account: AccountId) -> Balance {
            let accountbalance = self.insdebttoken.balance_of(account);
            let stablerate = self.userstablerate.get(&account);
            if accountbalance == 0 {
                return 0;
            }
            //this should be caculate by a formula
            let cumulatedinterest = 0.1;
            accountbalance * cumulatedinterest
        }

        //    * @dev Calculates the increase in balance since the last user interaction
        //    * @param user The address of the user for which the interest is being accumulated
        //    * @return The previous principal balance, the new principal balance and the balance increase
        fn caculatebalanceincrease(&self, user: AccountId) -> (Balance, Balance, Balance) {
            let previousprincipalbalance = self.insdebttoken.balance_of(user);
            if previousprincipalbalance == 0 {
                return (0, 0, 0);
            }
            let balanceincrease = self.balanceof(user) - previousprincipalbalance;

            (
                previousprincipalbalance,
                previousprincipalbalance + balanceincrease,
                balanceincrease,
            )
        }

        //    * @dev Calculates the total supply
        //    * @param avgRate The average rate at which the total supply increases
        //    * @return The debt balance of the user since the last burn/mint action
        fn calctotalsupply(&self, avgrate: u32) -> Balance {
            let principalsupply = self.insdebttoken.total_supply();
            if principalsupply == 0 {
                return 0;
            }

            //this should be caculate by a formula
            let cumulatedinterest = 0.1;

            principalsupply * cumulatedinterest
        }

        //    * @dev Mints stable debt tokens to an user
        //    * @param account The account receiving the debt tokens
        //    * @param amount The amount being minted
        //    * @param oldTotalSupply the total supply before the minting event
        fn _mint(&mut self, account: AccountId, amount: Balance, oldtotalsupply: Balance) {
            let oldaccountbalance = self.insdebttoken.balance_of(account);
            self.insdebttoken
                .balances
                .insert(account, oldaccountbalance + amount);
        }

        //    * @dev Burns stable debt tokens of an user
        //    * @param account The user getting his debt burned
        //    * @param amount The amount being burned
        //    * @param oldTotalSupply The total supply before the burning event
        fn _burn(&mut self, account: AccountId, amount: Balance, oldtotalsupply: Balance) {
            let oldaccountbalance = self.insdebttoken.balance_of(account);
            self.insdebttoken
                .balances
                .insert(account, oldaccountbalance - amount);
        }
    }
}
