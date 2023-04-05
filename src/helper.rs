use cosmwasm_std::{MessageInfo, Uint128};

use crate::{state::Pool, ContractError};

pub fn check_denom(info: &MessageInfo, pool: &Pool) -> Result<(), ContractError> {
    //token check
    match info.funds.len() {
        0 => Err(ContractError::MustSendCoin {}),
        1 => Ok(()),
        _ => Err(ContractError::InvalidOneTypeCoin {}),
    }?;

    let coin = &info.funds[0];
    //denom_check
    match coin.denom == pool.denom {
        true => Ok(()),
        false => Err(ContractError::InvalidDenom {}),
    }
}

pub fn check_game_contract(info: &MessageInfo, pool: &Pool) -> Result<(), ContractError> {
    match pool.game_contract == info.sender {
        true => Ok(()),
        false => Err(ContractError::InvalidContractAddress {}),
    }
}

pub fn check_enough_pool(pool: &Pool, amount: Uint128) -> Result<(), ContractError> {
    use std::cmp::Ordering::*;
    match pool.balance.cmp(&amount) {
        Less => Err(ContractError::NotEnoughPool {}),
        Equal | Greater => Ok(()),
    }
}
