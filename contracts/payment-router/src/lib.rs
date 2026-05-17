#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, vec, Address, Env, Vec};
use stellar_ai_payments_shared::Error;

#[contracttype]
pub struct Route {
    pub destination: Address,
    pub bps: u32, // basis points (100 = 1%)
}

#[contracttype]
pub enum DataKey {
    Admin,
    Routes,
}

#[contract]
pub struct PaymentRouterContract;

#[contractimpl]
impl PaymentRouterContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Set routing table. BPS must sum to 10000 (100%).
    pub fn set_routes(env: Env, caller: Address, routes: Vec<Route>) -> Result<(), Error> {
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        let total: u32 = routes.iter().map(|r| r.bps).sum();
        if total != 10_000 {
            return Err(Error::InvalidAmount);
        }
        env.storage().instance().set(&DataKey::Routes, &routes);
        Ok(())
    }

    /// Route a payment across all configured destinations proportionally.
    pub fn route(
        env: Env,
        sender: Address,
        asset: Address,
        total_amount: i128,
    ) -> Result<(), Error> {
        sender.require_auth();
        if total_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let routes: Vec<Route> = env
            .storage()
            .instance()
            .get(&DataKey::Routes)
            .ok_or(Error::NotFound)?;

        let token_client = token::Client::new(&env, &asset);
        // Pull full amount from sender first
        token_client.transfer(&sender, &env.current_contract_address(), &total_amount);

        for route in routes.iter() {
            let share = (total_amount * route.bps as i128) / 10_000;
            if share > 0 {
                token_client.transfer(&env.current_contract_address(), &route.destination, &share);
            }
        }
        Ok(())
    }

    pub fn get_routes(env: Env) -> Vec<Route> {
        env.storage()
            .instance()
            .get(&DataKey::Routes)
            .unwrap_or(vec![&env])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::Address as _,
        token::StellarAssetClient,
        vec, Env,
    };

    fn setup() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, PaymentRouterContract);
        PaymentRouterContractClient::new(&env, &contract_id).initialize(&admin);

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let asset = token_id.address();
        let sac = StellarAssetClient::new(&env, &asset);
        let sender = Address::generate(&env);
        sac.mint(&sender, &100_000_0000000);

        (env, contract_id, admin, asset)
    }

    #[test]
    fn test_set_and_get_routes() {
        let (env, contract_id, admin, _) = setup();
        let client = PaymentRouterContractClient::new(&env, &contract_id);
        let dest1 = Address::generate(&env);
        let dest2 = Address::generate(&env);
        let routes = vec![
            &env,
            Route { destination: dest1, bps: 7000 },
            Route { destination: dest2, bps: 3000 },
        ];
        client.set_routes(&admin, &routes);
        assert_eq!(client.get_routes().len(), 2);
    }

    #[test]
    fn test_routes_must_sum_to_10000() {
        let (env, contract_id, admin, _) = setup();
        let client = PaymentRouterContractClient::new(&env, &contract_id);
        let dest = Address::generate(&env);
        let bad_routes = vec![&env, Route { destination: dest, bps: 5000 }];
        assert!(client.try_set_routes(&admin, &bad_routes).is_err());
    }

    #[test]
    fn test_route_splits_payment() {
        let (env, contract_id, admin, asset) = setup();
        let client = PaymentRouterContractClient::new(&env, &contract_id);
        let sender = Address::generate(&env);
        let sac = StellarAssetClient::new(&env, &asset);
        sac.mint(&sender, &10_000_0000000);

        let dest1 = Address::generate(&env);
        let dest2 = Address::generate(&env);
        let routes = vec![
            &env,
            Route { destination: dest1.clone(), bps: 6000 },
            Route { destination: dest2.clone(), bps: 4000 },
        ];
        client.set_routes(&admin, &routes);
        client.route(&sender, &asset, &10_000_0000000);

        let token = token::Client::new(&env, &asset);
        assert_eq!(token.balance(&dest1), 6_000_0000000);
        assert_eq!(token.balance(&dest2), 4_000_0000000);
    }

    #[test]
    fn test_non_admin_cannot_set_routes() {
        let (env, contract_id, _, _) = setup();
        let client = PaymentRouterContractClient::new(&env, &contract_id);
        let rando = Address::generate(&env);
        let dest = Address::generate(&env);
        let routes = vec![&env, Route { destination: dest, bps: 10000 }];
        assert!(client.try_set_routes(&rando, &routes).is_err());
    }

    #[test]
    fn test_route_invalid_amount() {
        let (env, contract_id, admin, asset) = setup();
        let client = PaymentRouterContractClient::new(&env, &contract_id);
        let dest = Address::generate(&env);
        let routes = vec![&env, Route { destination: dest, bps: 10000 }];
        client.set_routes(&admin, &routes);
        let sender = Address::generate(&env);
        assert!(client.try_route(&sender, &asset, &0).is_err());
    }
}

// Paused state key
// Future: add pause/unpause for emergency stops

// TODO: emit route event with split details for indexers

    /// Get the current admin address.
    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }
