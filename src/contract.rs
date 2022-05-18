use cosmwasm_std::{
    entry_point, to_binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,Binary,
    StdResult, Uint128,CosmosMsg,WasmMsg,BankMsg,QueryRequest,WasmQuery};

use cw2::set_contract_version;
use cw20::{ Cw20ExecuteMsg, Cw20QueryMsg};
use crate::oracle::QueryMsg as OracleQueryMsg;

use crate::error::{ContractError};
use crate::msg::{ ExecuteMsg, InstantiateMsg};
use crate::state::{State,CONFIG};


const CONTRACT_NAME: &str = "SWAP_CONTRACT";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
   
    let state = State {
        owner : msg.owner,
        oracle_address:msg.oracle_address,
        token_address:msg.token_address
    };
    CONFIG.save(deps.storage,&state)?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
    ExecuteMsg::BuyLemons{} =>execute_buy_lemons(deps,env,info),
    ExecuteMsg::WithdrawAmount { amount }=>execute_withdraw_amount(deps,env,info,amount)
    }
}

fn execute_buy_lemons(
    deps: DepsMut,
    env:Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let  state = CONFIG.load(deps.storage)?;
    
    let deposit_amount = info
        .funds
        .iter()
        .find(|c| c.denom == "uluna".to_string())
        .map(|c| Uint128::from(c.amount))
        .unwrap_or_else(Uint128::zero);
    
    let lemon_price:Uint128 = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: state.oracle_address.to_string(),
        msg: to_binary(&OracleQueryMsg::GetPrice { })?,
    }))?;

    let buyable_token_amount = deposit_amount/lemon_price;

    let availabe_token_amount:Uint128 = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: state.oracle_address.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance { address: env.contract.address.to_string() })?,
    }))?;

    if availabe_token_amount<buyable_token_amount{
        return Err(ContractError::NotEnoughTokens {})
    }

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: state.token_address.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: buyable_token_amount,
            })?,
        })
    ))
}

fn execute_withdraw_amount(
    deps: DepsMut,
    env:Env,
    info: MessageInfo,
    amount:Uint128
) -> Result<Response, ContractError> {
 let  state = CONFIG.load(deps.storage)?;
 
 if state.owner !=info.sender.to_string(){
     return Err(ContractError::Unauthorized {  })
 }

 Ok(Response::new().add_message(
     CosmosMsg::Bank(BankMsg::Send {
            to_address: state.owner.to_string(),
            amount:vec![Coin{
                    denom:"uluna".to_string(),
                    amount:amount
                }]
        })
        ))

}



#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{ CosmosMsg, Coin};

    #[test]
    fn instantiate_contract() {
        let mut deps = mock_dependencies(&[]);
    
        let instantiate_msg = InstantiateMsg {
            owner:"owner".to_string(),
            token_address:"token_address".to_string(),
            oracle_address:"oracle_address".to_string() };
        
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());
            
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::WithdrawAmount { amount: Uint128::new(100) };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(),1);
        assert_eq!(res.messages[0].msg,CosmosMsg::Bank(BankMsg::Send {
            to_address: "owner".to_string(),
            amount:vec![Coin{
                    denom:"uluna".to_string(),
                    amount:Uint128::new(100)
                }]
        }))
    }

}
 