#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::helper::{check_denom, check_enough_pool, check_game_contract};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::{AllowanceAndTotalSupplyResponse, LPQueryMsg};
use crate::state::POOL;
use crate::state::{load_pool, save_pool, Pool};
// version info for migration info
const CONTRACT_NAME: &str = "amg:bank";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let lp_contract = deps.api.addr_validate(&msg.lp_contract.as_str())?;
    let game_contract = deps.api.addr_validate(&msg.game_contract.as_str())?;

    let pool = Pool {
        denom: "uconst".to_string(),
        balance: Uint128::new(0),
        fee_pool: Uint128::new(0),
        lp_contract,
        borrow_balance: Uint128::new(0),
        game_contract,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    save_pool(deps.storage, &pool)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {} => deposit(deps, info),
        ExecuteMsg::Withdraw {} => withdraw(deps, env, info),
        ExecuteMsg::ProvideFee {} => provide_fee(deps, info),
        ExecuteMsg::BorrowBalance { amount } => borrow_balance(deps, info, amount),
        ExecuteMsg::PayBack {} => payback(deps, info),
    }
}

fn provide_fee(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut pool = load_pool(deps.storage)?;

    //check betting contract
    check_game_contract(&info, &pool)?;

    //checking send fee token
    check_denom(&info, &pool)?;
    //fee pool += fee_amount

    pool.fee_pool += info.funds[0].amount;
    save_pool(deps.storage, &pool)?;
    Ok(Response::new())
}

fn payback(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut pool = load_pool(deps.storage)?;

    //check betting contract
    check_game_contract(&info, &pool)?;

    //checking send fee token
    check_denom(&info, &pool)?;
    //payback amount is borrowed amount * 2
    let denom_amount = info.funds[0].amount;

    //balance + denom_amount
    pool.balance += denom_amount;

    //borrow_balance - denom_amount / 2
    pool.borrow_balance -= denom_amount
        .checked_div(Uint128::new(2))
        .unwrap_or_default();
    save_pool(deps.storage, &pool)?;
    Ok(Response::new())
}

fn borrow_balance(
    deps: DepsMut,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut pool = load_pool(deps.storage)?;

    //check betting contract
    check_game_contract(&info, &pool)?;

    check_enough_pool(&pool, amount)?;

    pool.balance -= amount;
    save_pool(deps.storage, &pool)?;

    let bank_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![coin(amount.u128(), "uconst")],
    });

    Ok(Response::new().add_message(bank_msg))
}

fn deposit(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut pool = load_pool(deps.storage)?;
    check_denom(&info, &pool)?;

    let deposit_amount = info.funds[0].amount;

    let pool_balance = pool.balance;

    let lp_total_supply: Uint128 = deps.querier.query(
        &WasmQuery::Smart {
            contract_addr: pool.lp_contract.to_string(),
            msg: to_binary(&LPQueryMsg::TotalSupply {})?,
        }
        .into(),
    )?;
    // amount = 30, pool.balance = 1000
    // ratio = 0.03
    let ratio = match pool_balance.is_zero() {
        true => Decimal::new(Uint128::one()),
        false => Decimal::from_ratio(deposit_amount, pool_balance),
    };
    let lp_mint_amount = match lp_total_supply.is_zero() {
        true => deposit_amount.u128(),
        false => (ratio * lp_total_supply).u128(),
    };

    pool.balance += deposit_amount;
    save_pool(deps.storage, &pool)?;
    // POOL.save(deps.storage, &pool)?;
    let mint_msg = Cw20ExecuteMsg::Mint {
        recipient: info.sender.to_string(),
        amount: Uint128::new(lp_mint_amount),
    };
    let response = Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pool.lp_contract.to_string(),
            msg: to_binary(&mint_msg)?,
            funds: vec![],
        }))
        .add_attribute("method", "deposit")
        .add_attribute("denom", info.funds[0].denom.clone())
        .add_attribute("deposit_amount", deposit_amount);
    Ok(response)
}

fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let mut pool = POOL.load(deps.storage)?;
    let lp_contract_address = &pool.lp_contract;

    let res: AllowanceAndTotalSupplyResponse = deps.querier.query(
        &WasmQuery::Smart {
            contract_addr: lp_contract_address.to_string(),
            msg: to_binary(&LPQueryMsg::AllowanceAndTotalSuuply {
                owner: info.sender.to_string(),
                spender: env.contract.address.to_string(),
            })?,
        }
        .into(),
    )?;
    let AllowanceAndTotalSupplyResponse {
        total_supply,
        allowance,
        expires,
    } = res;
    let request_widthdraw_amount = allowance;

    if allowance.is_zero() {
        return Err(ContractError::InvalidLPAllowance {});
    }
    //expires handling todo!
    if expires.is_expired(&env.block) {
        return Err(ContractError::InvalidExpires {});
    }

    let withdraw_balance = match total_supply.u128() {
        0 => {
            let ratio = Decimal::new(Uint128::new(1));
            (pool.balance * ratio).u128()
        }
        _ => {
            let ratio = Decimal::from_ratio(request_widthdraw_amount, total_supply);
            (pool.balance * ratio).u128()
        }
    };

    pool.balance = pool
        .balance
        .checked_sub(Uint128::new(withdraw_balance))
        .unwrap_or_default();

    POOL.save(deps.storage, &pool)?;

    let withdraw_coin = coin(withdraw_balance, pool.denom.as_str());

    let bank_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![withdraw_coin],
    });
    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: lp_contract_address.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::BurnFrom {
            owner: info.sender.to_string(),
            amount: request_widthdraw_amount,
        })?,
        funds: vec![],
    });
    Ok(Response::new()
        .add_message(burn_msg)
        .add_message(bank_msg)
        .add_attribute("method", "withdraw")
        .add_attribute("lp_amount", request_widthdraw_amount.to_string())
        .add_attribute(
            format!("{}_amount", pool.denom.as_str()),
            withdraw_balance.to_string(),
        ))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPool {} => to_binary(&get_pool(deps)?),
    }
}

fn get_pool(deps: Deps) -> StdResult<Pool> {
    let pool = POOL.load(deps.storage)?;
    Ok(pool)
}
