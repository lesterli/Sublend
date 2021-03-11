#![cfg_attr(not(feature = "std"), no_std)]

pub use self::atoken::AToken;
use ink_lang as ink;

#[ink::contract]
mod atoken {

    use incentivizederc20::Erc20;
    #[cfg(not(feature = "ink-as-dependency"))]
    use ink_env::call::FromAccountId;
    use ink_prelude::string::String;
    #[cfg(not(feature = "ink-as-dependency"))]
    use ink_storage::{collections::HashMap as StorageHashMap, lazy::Lazy};

    /// The ERC-20 error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned if not enough balance to fulfill a request is available.
        InsufficientBalance,
        InsufficientSupply,
        /// Returned if not enough allowance to fulfill a request is available.
        InsufficientAllowance,
    }

    /// The ERC-20 result type.
    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(storage)]
    pub struct AToken {
        ///underlying asset
        underasset: Lazy<Erc20>,
        ///reserve asset address
        reserve_asset_address: AccountId,
        ///reserve asset
        reserveasset: Lazy<Erc20>,
        ///atoken instance
        insatoken: Lazy<Erc20>,
    }

    impl AToken {
        /// Creates a new ERC-20 contract with the specified initial supply.
        #[ink(constructor)]
        pub fn new(
            underlying_asset_address: AccountId,
            reserve_asset_address: AccountId,
            incentivizede_contract_hash: Hash,
            name: Option<String>,
            symbol: Option<String>,
            version: u32,
        ) -> Self {
            let unasset: Erc20 = FromAccountId::from_account_id(underlying_asset_address);
            let reasset: Erc20 = FromAccountId::from_account_id(reserve_asset_address);
            let salt = version.to_le_bytes();
            let insatoken = Erc20::new(0, name, symbol, 18)
                .endowment(0)
                .code_hash(incentivizede_contract_hash)
                .salt_bytes(salt)
                .params()
                .instantiate()
                .expect("atoken instantiate failed");

            Self {
                underasset: unasset,
                reserveasset: reasset,
                insatoken: insatoken,
                reserve_asset_address: reserve_asset_address,
            }
        }

        // * @dev Burns aTokens from `user` and sends the equivalent amount of underlying to `receiverOfUnderlying`
        // * - Only callable by the LendingPool, as extra state updates there need to be managed
        // * @param user The owner of the aTokens, getting them burned
        // * @param receiverOfUnderlying The address that will receive the underlying
        // * @param amount The amount being burned
        // * @param index The new liquidity index of the reserve
        #[ink(message)]
        pub fn burn(
            &mut self,
            user: AccountId,
            receiverofunderlying: AccountId,
            amount: Balance,
            index: u32,
        ) {
            //TODO need only lendingpool
            let amountscaled = amount / index.into();
            assert_ne!(amountscaled, 0);
            self.insatoken.burn(user, amountscaled);
            self.underasset.transfer(receiverofunderlying, amount);
        }

        //    * @dev Mints `amount` aTokens to `user`
        //    * - Only callable by the LendingPool, as extra state updates there need to be managed
        //    * @param user The address receiving the minted tokens
        //    * @param amount The amount of tokens getting minted
        //    * @param index The new liquidity index of the reserve
        //    * @return `true` if the the previous balance of the user was 0
        #[ink(message)]
        pub fn mint(&mut self, user: AccountId, amount: Balance, index: u32) -> bool {
            //TODO need only lendingpool
            let previousbalance = self.insatoken.balance_of(user);
            let amountscaled = amount / index.into();
            assert_ne!(amountscaled, 0);
            self.insatoken.mint(user, amountscaled);
            previousbalance == 0
        }

        //    * @dev Mints aTokens to the reserve treasury
        //    * - Only callable by the LendingPool
        //    * @param amount The amount of tokens getting minted
        //    * @param index The new liquidity index of the reserve
        #[ink(message)]
        pub fn minttotreasury(&mut self, amount: Balance, index: u32) {
            //TODO need only lendingpool
            if amount == 0 {
                return ();
            }
            self.insatoken.mint(self.reserve_asset_address, amount);
        }

        // * @dev Transfers aTokens in the event of a borrow being liquidated, in case the liquidators reclaims the aToken
        // * - Only callable by the LendingPool
        // * @param from The address getting liquidated, current owner of the aTokens
        // * @param to The recipient
        // * @param value The amount of tokens getting transferred
        #[ink(message)]
        pub fn transferonliquidation(&mut self, from: AccountId, to: AccountId, value: Balance) {
            //TODO only lengdingpool
            self._transfer(from, to, value, false);
        }

        //    * @dev Transfers the aTokens between two users. Validates the transfer
        //    * (ie checks for valid HF after the transfer) if required
        //    * @param from The source address
        //    * @param to The destination address
        //    * @param amount The amount getting transferred
        //    * @param validate `true` if the transfer needs to be validated
        fn _transfer(&mut self, from: AccountId, to: AccountId, amount: Balance, validate: bool) {
            //index should from lending pool
            let index = 1;
            let frombalancebefore = self.insatoken.balance_of(from) * index;
            let tobalancebefore = self.insatoken.balance_of(to) * index;
            self.insatoken.transfer_from_to(from, to, amount / index);
            //if validate then update lendingpool status
        }

        // * @dev Calculates the balance of the user: principal balance + interest generated by the principal
        // * @param user The user whose balance is calculated
        // * @return The balance of the user
        #[ink(message)]
        pub fn balanceof(&self, user: AccountId) -> Balance {
            //TODO index should come from lending pool
            let index = 1;
            self.insatoken.balance_of(user) * index
        }

        // * @dev Returns the scaled balance of the user and the scaled total supply.
        // * @param user The address of the user
        // * @return The scaled balance of the user
        // * @return The scaled balance and the scaled total supply
        #[ink(message)]
        pub fn getscaledbalanceandsupply(&self, user: AccountId) -> (Balance, Balance) {
            (
                self.insatoken.balance_of(user),
                self.insatoken.total_supply(),
            )
        }

        //    * @dev calculates the total supply of the specific aToken
        //    * since the balance of every single user increases over time, the total supply
        //    * does that too.
        //    * @return the current total supply
        #[ink(message)]
        pub fn totalsupply(&self) -> Balance {
            let curr_totalsupply = self.insatoken.total_supply();
            if curr_totalsupply == 0 {
                return 0;
            }
            //TODO index should from lending pool
            let index = 1;
            curr_totalsupply * index
        }

        // * @dev Returns the scaled total supply of the variable debt token. Represents sum(debt/index)
        // * @return the scaled total supply
        #[ink(message)]
        pub fn scaledtotalsuplly(&self) -> Balance {
            self.insatoken.total_supply()
        }

        // * @dev Transfers the underlying asset to `target`. Used by the LendingPool to transfer
        // * assets in borrow(), withdraw() and flashLoan()
        // * @param target The recipient of the aTokens
        // * @param amount The amount getting transferred
        // * @return The amount transferred
        #[ink(message)]
        pub fn transferunderlyingto(&mut self, target: AccountId, amount: Balance) -> Balance {
            //TODO only lendingpool
            self.underasset.transfer(target, amount);
            amount
        }
    }
}
