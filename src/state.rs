use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Pool {
    pub denom: String,
    pub balance: Uint128,
    pub lp_contract: Addr,
    pub game_contract: Addr,
}

pub fn save_pool(storage: &mut dyn Storage, pool: &Pool) -> StdResult<()> {
    POOL.save(storage, pool)
}

//Pool 스토리지 읽어오는 함수
pub fn load_pool(storage: &dyn Storage) -> StdResult<Pool> {
    POOL.load(storage)
}

pub const POOL: Item<Pool> = Item::new("pool");
pub const FEE: Map<&Addr, Uint128> = Map::new("fee");
