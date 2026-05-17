#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, String,
};
use stellar_ai_payments_shared::{Error, PaymentRecord, PaymentStatus};

#[contracttype]
pub enum DataKey {
    Payment(String),
    Admin,
}

#[contract]
pub struct AgentEscrowContract;

#[contractimpl]
impl AgentEscrowContract {
    /// Initialize the contract with an admin address.
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Deposit funds into escrow for a payment.
    pub fn deposit(
        env: Env,
        payment_id: String,
        sender: Address,
        recipient: Address,
        asset: Address,
        amount: i128,
        ttl_seconds: u64,
    ) -> Result<(), Error> {
        sender.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let key = DataKey::Payment(payment_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::AlreadyExists);
        }

        // Transfer tokens from sender to this contract
        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&sender, &env.current_contract_address(), &amount);

        let now = env.ledger().timestamp();
        let record = PaymentRecord {
            id: payment_id,
            sender,
            recipient,
            amount,
            asset,
            status: PaymentStatus::Pending,
            created_at: now,
            expires_at: now + ttl_seconds,
        };

        env.storage().persistent().set(&key, &record);
        Ok(())
    }

    /// Release escrowed funds to the recipient.
    pub fn release(env: Env, payment_id: String, caller: Address) -> Result<(), Error> {
        caller.require_auth();

        let key = DataKey::Payment(payment_id);
        let mut record: PaymentRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NotFound)?;

        if record.status != PaymentStatus::Pending {
            return Err(Error::Unauthorized);
        }

        // Only sender or admin can release
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if caller != record.sender && caller != admin {
            return Err(Error::Unauthorized);
        }

        let token_client = token::Client::new(&env, &record.asset);
        token_client.transfer(&env.current_contract_address(), &record.recipient, &record.amount);

        record.status = PaymentStatus::Released;
        env.storage().persistent().set(&key, &record);
        Ok(())
    }

    /// Refund escrowed funds to the sender after expiry.
    pub fn refund(env: Env, payment_id: String) -> Result<(), Error> {
        let key = DataKey::Payment(payment_id);
        let mut record: PaymentRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NotFound)?;

        if record.status != PaymentStatus::Pending {
            return Err(Error::Unauthorized);
        }

        let now = env.ledger().timestamp();
        if now < record.expires_at {
            return Err(Error::Unauthorized);
        }

        let token_client = token::Client::new(&env, &record.asset);
        token_client.transfer(&env.current_contract_address(), &record.sender, &record.amount);

        record.status = PaymentStatus::Refunded;
        env.storage().persistent().set(&key, &record);
        Ok(())
    }

    /// Get a payment record by ID.
    pub fn get_payment(env: Env, payment_id: String) -> Result<PaymentRecord, Error> {
        let key = DataKey::Payment(payment_id);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::{Client as TokenClient, StellarAssetClient},
        Env, String,
    };

    fn setup() -> (Env, Address, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let recipient = Address::generate(&env);
        let contract_id = env.register_contract(None, AgentEscrowContract);
        let client = AgentEscrowContractClient::new(&env, &contract_id);
        client.initialize(&admin);

        // Create a test token
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let asset_address = token_id.address();
        let sac = StellarAssetClient::new(&env, &asset_address);
        sac.mint(&sender, &10_000_0000000);

        (env, contract_id, sender, recipient, asset_address)
    }

    #[test]
    fn test_deposit_and_release() {
        let (env, contract_id, sender, recipient, asset) = setup();
        let client = AgentEscrowContractClient::new(&env, &contract_id);
        let pid = String::from_str(&env, "pay-001");

        client.deposit(&pid, &sender, &recipient, &asset, &1_000_0000000, &3600);
        let record = client.get_payment(&pid).unwrap();
        assert_eq!(record.status, PaymentStatus::Pending);

        client.release(&pid, &sender);
        let record = client.get_payment(&pid).unwrap();
        assert_eq!(record.status, PaymentStatus::Released);
    }

    #[test]
    fn test_deposit_invalid_amount() {
        let (env, contract_id, sender, recipient, asset) = setup();
        let client = AgentEscrowContractClient::new(&env, &contract_id);
        let pid = String::from_str(&env, "pay-002");
        let result = client.try_deposit(&pid, &sender, &recipient, &asset, &0, &3600);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_deposit_fails() {
        let (env, contract_id, sender, recipient, asset) = setup();
        let client = AgentEscrowContractClient::new(&env, &contract_id);
        let pid = String::from_str(&env, "pay-003");
        client.deposit(&pid, &sender, &recipient, &asset, &100_0000000, &3600);
        let result = client.try_deposit(&pid, &sender, &recipient, &asset, &100_0000000, &3600);
        assert!(result.is_err());
    }

    #[test]
    fn test_refund_after_expiry() {
        let (env, contract_id, sender, recipient, asset) = setup();
        let client = AgentEscrowContractClient::new(&env, &contract_id);
        let pid = String::from_str(&env, "pay-004");
        client.deposit(&pid, &sender, &recipient, &asset, &100_0000000, &10);

        // Advance ledger past expiry
        env.ledger().with_mut(|l| l.timestamp += 100);
        client.refund(&pid);
        let record = client.get_payment(&pid).unwrap();
        assert_eq!(record.status, PaymentStatus::Refunded);
    }

    #[test]
    fn test_refund_before_expiry_fails() {
        let (env, contract_id, sender, recipient, asset) = setup();
        let client = AgentEscrowContractClient::new(&env, &contract_id);
        let pid = String::from_str(&env, "pay-005");
        client.deposit(&pid, &sender, &recipient, &asset, &100_0000000, &3600);
        let result = client.try_refund(&pid);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_nonexistent_payment() {
        let (env, contract_id, ..) = setup();
        let client = AgentEscrowContractClient::new(&env, &contract_id);
        let pid = String::from_str(&env, "nonexistent");
        let result = client.try_get_payment(&pid);
        assert!(result.is_err());
    }
}
