#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};
use stellar_ai_payments_shared::Error;

#[contracttype]
pub enum DataKey {
    Admin,
    Balance,
    AllowedAgent(Address),
    AgentAllowance(Address),
}

#[contract]
pub struct SponsoredFeeVaultContract;

#[contractimpl]
impl SponsoredFeeVaultContract {
    /// Initialize vault with admin and XLM asset address.
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Balance, &0i128);
    }

    /// Deposit XLM into the vault for fee sponsorship.
    pub fn deposit(env: Env, from: Address, asset: Address, amount: i128) -> Result<(), Error> {
        from.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&from, &env.current_contract_address(), &amount);

        let current: i128 = env.storage().instance().get(&DataKey::Balance).unwrap_or(0);
        env.storage().instance().set(&DataKey::Balance, &(current + amount));
        Ok(())
    }

    /// Register an agent with a per-tx fee allowance (in stroops).
    pub fn register_agent(env: Env, caller: Address, agent: Address, allowance: i128) -> Result<(), Error> {
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        env.storage().persistent().set(&DataKey::AllowedAgent(agent.clone()), &true);
        env.storage().persistent().set(&DataKey::AgentAllowance(agent), &allowance);
        Ok(())
    }

    /// Deduct fee from vault on behalf of an agent.
    pub fn deduct_fee(env: Env, agent: Address, asset: Address, fee: i128) -> Result<(), Error> {
        agent.require_auth();

        let allowed: bool = env
            .storage()
            .persistent()
            .get(&DataKey::AllowedAgent(agent.clone()))
            .unwrap_or(false);
        if !allowed {
            return Err(Error::Unauthorized);
        }

        let allowance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::AgentAllowance(agent.clone()))
            .unwrap_or(0);
        if fee > allowance {
            return Err(Error::InsufficientFunds);
        }

        let balance: i128 = env.storage().instance().get(&DataKey::Balance).unwrap_or(0);
        if fee > balance {
            return Err(Error::InsufficientFunds);
        }

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&env.current_contract_address(), &agent, &fee);

        env.storage().instance().set(&DataKey::Balance, &(balance - fee));
        Ok(())
    }

    /// Get current vault balance.
    pub fn get_balance(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::Balance).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::Address as _,
        token::StellarAssetClient,
        Env,
    };

    fn setup() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, SponsoredFeeVaultContract);
        let client = SponsoredFeeVaultContractClient::new(&env, &contract_id);
        client.initialize(&admin);

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let asset = token_id.address();
        let sac = StellarAssetClient::new(&env, &asset);
        sac.mint(&admin, &100_000_0000000);

        (env, contract_id, admin, asset)
    }

    #[test]
    fn test_deposit_increases_balance() {
        let (env, contract_id, admin, asset) = setup();
        let client = SponsoredFeeVaultContractClient::new(&env, &contract_id);
        client.deposit(&admin, &asset, &1_000_0000000);
        assert_eq!(client.get_balance(), 1_000_0000000);
    }

    #[test]
    fn test_deposit_invalid_amount() {
        let (env, contract_id, admin, asset) = setup();
        let client = SponsoredFeeVaultContractClient::new(&env, &contract_id);
        assert!(client.try_deposit(&admin, &asset, &0).is_err());
    }

    #[test]
    fn test_register_and_deduct_fee() {
        let (env, contract_id, admin, asset) = setup();
        let client = SponsoredFeeVaultContractClient::new(&env, &contract_id);
        client.deposit(&admin, &asset, &10_000_0000000);

        let agent = Address::generate(&env);
        let sac = StellarAssetClient::new(&env, &asset);
        sac.mint(&agent, &0);

        client.register_agent(&admin, &agent, &500);
        client.deduct_fee(&agent, &asset, &100);
        assert_eq!(client.get_balance(), 10_000_0000000 - 100);
    }

    #[test]
    fn test_unregistered_agent_cannot_deduct() {
        let (env, contract_id, admin, asset) = setup();
        let client = SponsoredFeeVaultContractClient::new(&env, &contract_id);
        client.deposit(&admin, &asset, &1_000_0000000);
        let agent = Address::generate(&env);
        assert!(client.try_deduct_fee(&agent, &asset, &100).is_err());
    }

    #[test]
    fn test_deduct_exceeds_allowance() {
        let (env, contract_id, admin, asset) = setup();
        let client = SponsoredFeeVaultContractClient::new(&env, &contract_id);
        client.deposit(&admin, &asset, &10_000_0000000);
        let agent = Address::generate(&env);
        client.register_agent(&admin, &agent, &100);
        assert!(client.try_deduct_fee(&agent, &asset, &200).is_err());
    }

    #[test]
    fn test_non_admin_cannot_register_agent() {
        let (env, contract_id, _admin, _asset) = setup();
        let client = SponsoredFeeVaultContractClient::new(&env, &contract_id);
        let rando = Address::generate(&env);
        let agent = Address::generate(&env);
        assert!(client.try_register_agent(&rando, &agent, &100).is_err());
    }
}

    /// Withdraw XLM from vault (admin only).
    pub fn withdraw(env: Env, caller: Address, asset: Address, amount: i128) -> Result<(), Error> {
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if caller != admin { return Err(Error::Unauthorized); }
        let balance: i128 = env.storage().instance().get(&DataKey::Balance).unwrap_or(0);
        if amount > balance { return Err(Error::InsufficientFunds); }
        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&env.current_contract_address(), &caller, &amount);
        env.storage().instance().set(&DataKey::Balance, &(balance - amount));
        Ok(())
    }

// TODO: emit events on deposit/deduct for off-chain monitoring

    /// Get the current admin address.
    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

// Future: track registered agent count for admin dashboard
