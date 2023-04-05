use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Must Send Coin")]
    MustSendCoin {},

    #[error("Must Send One Coin")]
    InvalidOneTypeCoin {},

    #[error("Denom Invalid")]
    InvalidDenom {},

    #[error("Only Game Contract use function")]
    InvalidContractAddress {},

    #[error("Pool is smaller than Borrow amount")]
    NotEnoughPool {},

    #[error("Invalid LP Allowance")]
    InvalidLPAllowance {},

    #[error("Expires is Invalid you must setting expires")]
    InvalidExpires {},
}
