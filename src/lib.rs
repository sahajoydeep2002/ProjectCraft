/*!
Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::fungible_token::core::FungibleTokenCore;
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::resolver::FungibleTokenResolver;
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{env, log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}

const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' class='svg-icon' style='width: 1em; height: 1em;vertical-align: middle;fill: currentColor;overflow: hidden;' viewBox='0 0 1024 1024' version='1.1'%3e%3cpath d='M70.6 267.1c11.3-19.6 36.4-26.3 55.9-15l372.8 215.3 369.1-213.1c19.6-11.3 44.6-4.6 55.9 15 0.2 0.3 0.3 0.5 0.4 0.8-3.5-7-8.9-13-16.2-17.2L521.7 29.5c-7.5-6.1-16.9-9.3-26.4-9.2-9.5-0.1-18.9 3.1-26.4 9.2l-383 221.1c-10.1 5.8-16.8 15.4-19.3 25.9 0.9-3.1 2.2-6.3 4-9.4z' fill='%23242424'/%3e%3cpath d='M925.5 271.5c9.6 19.2 2.7 42.9-16.2 53.8L543.4 536.5v429c0 22.6-18.3 41-41 41-22.6 0-41-18.3-41-41V540.1l-375.9-217c-11.9-6.9-19.1-18.8-20.3-31.6v446.1c-1.5 15.5 5.9 31.1 20.3 39.4l387.4 223.7c7.5 6.1 16.9 9.3 26.4 9.2 9.5 0.1 18.9-3.1 26.4-9.2l376-217.1c16.6-5.2 28.7-20.7 28.7-39.1V290.9c0-7.1-1.8-13.7-4.9-19.4z' fill='%23242424'/%3e%3c/svg%3e";

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: ValidAccountId, total_supply: U128) -> Self {
        Self::internal_new(
            owner_id,
            total_supply,
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "BUILD".to_string(),
                symbol: "BUILD".to_string(),
                icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
                reference: None,
                reference_hash: None,
                decimals: 5,
            },
        )
    }

    fn internal_new(
        owner_id: ValidAccountId,
        total_supply: U128,
        metadata: FungibleTokenMetadata,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(b"a".to_vec()),
            metadata: LazyOption::new(b"m".to_vec(), Some(&metadata)),
        };
        this.token.internal_register_account(owner_id.as_ref());
        this.token
            .internal_deposit(owner_id.as_ref(), total_supply.into());
        this
    }

    #[payable]
    pub fn mint(&mut self, amount: U128) {
        let caller = env::predecessor_account_id();
        log!("calling mint!");
        self.register_if_needed(&caller);
        self.token.internal_deposit(&caller, amount.into());
    }

    fn register_if_needed(&mut self, account_id: &AccountId) {
        let registered = self.token.accounts.contains_key(&account_id);
        if !registered {
            self.token.internal_register_account(&account_id);
        }
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

#[near_bindgen]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: ValidAccountId, amount: U128, memo: Option<String>) {
        self.register_if_needed(&receiver_id.clone().into());
        self.token.ft_transfer(receiver_id, amount, memo)
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.register_if_needed(&receiver_id.clone().into());
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: ValidAccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near_bindgen]
impl FungibleTokenResolver for Contract {
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: ValidAccountId,
        receiver_id: ValidAccountId,
        amount: U128,
    ) -> U128 {
        let sender_id: AccountId = sender_id.into();
        let (used_amount, burned_amount) =
            self.token
                .internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
        if burned_amount > 0 {
            self.on_tokens_burned(sender_id, burned_amount);
        }
        used_amount.into()
    }
}

// near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use near_sdk::MockedBlockchain;

    use super::*;

    const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new(accounts(1).into(), TOTAL_SUPPLY.into());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(2).into(), TOTAL_SUPPLY.into());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(
            contract.ft_balance_of(accounts(2)).0,
            (TOTAL_SUPPLY - transfer_amount)
        );
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
