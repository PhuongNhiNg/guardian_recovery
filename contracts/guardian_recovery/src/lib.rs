#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Map, Symbol, Vec};

/// Guardian-based social recovery for Stellar accounts.
///
/// The owner of an account designates a set of N trusted guardians and an
/// M-of-N approval threshold. If the owner loses access to their secret key,
/// a candidate new owner can start a recovery flow; once M guardians
/// approve, the recovery can be executed and the candidate becomes the
/// authoritative owner of the account. The original owner can cancel any
/// pending recovery at any time while they still hold their secret key.
#[contract]
pub struct GuardianRecovery;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Error {
    /// The owner has no guardian configuration stored.
    NotInitialized = 1,
    /// The guardian list is empty.
    InvalidGuardianSet = 2,
    /// The threshold is 0 or exceeds the guardian count.
    InvalidThreshold = 3,
    /// The caller is not a registered guardian.
    NotAGuardian = 4,
    /// No recovery is currently pending for the given account.
    RecoveryNotFound = 5,
    /// The recovery has already been executed.
    RecoveryAlreadyExecuted = 6,
    /// Not enough guardian approvals have been collected.
    ThresholdNotReached = 7,
    /// This guardian has already approved the pending recovery.
    AlreadyApproved = 8,
    /// The supplied new_owner does not match the pending recovery.
    NewOwnerMismatch = 9,
}

#[contracttype]
#[derive(Clone)]
pub struct GuardianConfig {
    pub guardians: Vec<Address>,
    pub threshold: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct Recovery {
    pub new_owner: Address,
    pub approvals: Map<Address, bool>,
    pub executed: bool,
}

#[contractimpl]
impl GuardianRecovery {
    /// Register or update the guardian set and M-of-N threshold for
    /// `owner`. The owner must authorize this call. `guardians` must be
    /// non-empty and `threshold` must satisfy `1 <= threshold <= N`.
    pub fn set_guardians(
        env: Env,
        owner: Address,
        guardians: Vec<Address>,
        threshold: u32,
    ) -> Result<(), Error> {
        owner.require_auth();

        if guardians.is_empty() {
            return Err(Error::InvalidGuardianSet);
        }
        if threshold == 0 || threshold > guardians.len() {
            return Err(Error::InvalidThreshold);
        }

        let key = Symbol::new(&env, "cfg");
        let mut configs: Map<Address, GuardianConfig> = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(Map::new(&env));

        configs.set(
            owner,
            GuardianConfig {
                guardians,
                threshold,
            },
        );
        env.storage().instance().set(&key, &configs);

        Ok(())
    }

    /// Begin a recovery flow. `new_owner` declares that the account
    /// `old_owner` should be rotated to their public key. The candidate
    /// authorizes the call. Any previous pending recovery for `old_owner`
    /// is reset.
    pub fn initiate_recovery(
        env: Env,
        new_owner: Address,
        old_owner: Address,
    ) -> Result<(), Error> {
        new_owner.require_auth();

        Self::require_configured(&env, &old_owner)?;

        let key = Symbol::new(&env, "rec");
        let mut recs: Map<Address, Recovery> = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(Map::new(&env));

        recs.set(
            old_owner,
            Recovery {
                new_owner,
                approvals: Map::new(&env),
                executed: false,
            },
        );
        env.storage().instance().set(&key, &recs);

        Ok(())
    }

    /// Record `guardian`'s approval for the pending recovery of
    /// `old_owner`. The guardian must be in the owner's configured set and
    /// may only approve once per recovery.
    pub fn approve_recovery(
        env: Env,
        guardian: Address,
        old_owner: Address,
    ) -> Result<(), Error> {
        guardian.require_auth();

        let cfg = Self::require_configured(&env, &old_owner)?;
        if !vec_contains(&cfg.guardians, &guardian) {
            return Err(Error::NotAGuardian);
        }

        let key = Symbol::new(&env, "rec");
        let mut recs: Map<Address, Recovery> = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(Map::new(&env));

        let mut recovery = recs
            .get(old_owner.clone())
            .ok_or(Error::RecoveryNotFound)?;

        if recovery.executed {
            return Err(Error::RecoveryAlreadyExecuted);
        }
        if recovery.approvals.get(guardian.clone()).unwrap_or(false) {
            return Err(Error::AlreadyApproved);
        }

        recovery.approvals.set(guardian, true);
        recs.set(old_owner, recovery);
        env.storage().instance().set(&key, &recs);

        Ok(())
    }

    /// Finalize the recovery once the approval threshold is met. The
    /// supplied `new_owner` must match the candidate stored when the
    /// recovery was initiated. After a successful call the recovery is
    /// marked executed; the candidate is now the authoritative owner.
    /// This call performs a pure state transition; it does not move any
    /// XLM or other on-chain assets.
    pub fn execute_recovery(
        env: Env,
        old_owner: Address,
        new_owner: Address,
    ) -> Result<(), Error> {
        let cfg = Self::require_configured(&env, &old_owner)?;

        let key = Symbol::new(&env, "rec");
        let mut recs: Map<Address, Recovery> = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(Map::new(&env));

        let mut recovery = recs
            .get(old_owner.clone())
            .ok_or(Error::RecoveryNotFound)?;

        if recovery.executed {
            return Err(Error::RecoveryAlreadyExecuted);
        }
        if recovery.new_owner != new_owner {
            return Err(Error::NewOwnerMismatch);
        }

        let approvals = count_true(&recovery.approvals);
        if approvals < cfg.threshold {
            return Err(Error::ThresholdNotReached);
        }

        recovery.executed = true;
        recs.set(old_owner, recovery);
        env.storage().instance().set(&key, &recs);

        Ok(())
    }

    /// Owner-initiated cancel of any pending recovery for their own
    /// account. The owner must authorize this call.
    pub fn cancel_recovery(env: Env, owner: Address) -> Result<(), Error> {
        owner.require_auth();

        let key = Symbol::new(&env, "rec");
        let mut recs: Map<Address, Recovery> = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(Map::new(&env));

        if !recs.contains_key(owner.clone()) {
            return Err(Error::RecoveryNotFound);
        }
        recs.remove(owner);
        env.storage().instance().set(&key, &recs);

        Ok(())
    }

    /// Returns the number of distinct guardian approvals recorded for the
    /// pending recovery of `old_owner`. Returns 0 if no recovery is
    /// pending.
    pub fn get_approvals(env: Env, old_owner: Address) -> u32 {
        let key = Symbol::new(&env, "rec");
        let recs: Map<Address, Recovery> = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(Map::new(&env));

        match recs.get(old_owner) {
            Some(r) => count_true(&r.approvals),
            None => 0,
        }
    }

    /// Returns the configured guardian list for `owner`. Returns an empty
    /// `Vec` if the owner has not configured guardians.
    pub fn get_guardians(env: Env, owner: Address) -> Vec<Address> {
        let key = Symbol::new(&env, "cfg");
        let configs: Map<Address, GuardianConfig> = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(Map::new(&env));
        configs
            .get(owner)
            .map(|c| c.guardians)
            .unwrap_or(Vec::new(&env))
    }

    /// Returns the configured M-of-N threshold for `owner`. Returns 0 if
    /// the owner has not configured guardians.
    pub fn get_threshold(env: Env, owner: Address) -> u32 {
        let key = Symbol::new(&env, "cfg");
        let configs: Map<Address, GuardianConfig> = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(Map::new(&env));
        configs.get(owner).map(|c| c.threshold).unwrap_or(0)
    }

    // ---- internal helpers ----

    fn require_configured(env: &Env, owner: &Address) -> Result<GuardianConfig, Error> {
        let key = Symbol::new(env, "cfg");
        let configs: Map<Address, GuardianConfig> = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(Map::new(env));
        configs.get(owner.clone()).ok_or(Error::NotInitialized)
    }
}

fn vec_contains(vec: &Vec<Address>, target: &Address) -> bool {
    for a in vec.iter() {
        if a == *target {
            return true;
        }
    }
    false
}

fn count_true(map: &Map<Address, bool>) -> u32 {
    let mut n: u32 = 0;
    for k in map.keys() {
        if map.get(k).unwrap_or(false) {
            n = n.saturating_add(1);
        }
    }
    n
}
