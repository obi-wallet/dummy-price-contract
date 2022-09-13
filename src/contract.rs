#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{Asset, AssetInfo, ExecuteMsg, InstantiateMsg, QueryMsg, SimulationResponse};
use crate::state::{State, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:dummyprice";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        asset_prices: msg.asset_prices,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {}
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Simulation { offer_asset } => to_binary(&query_simulation(deps, offer_asset)?),
        QueryMsg::ReverseSimulation { ask_asset } => to_binary(&query_reverse_simulation(deps, ask_asset)?),
    }
}

fn query_simulation(deps: Deps, offer_asset: Asset) -> StdResult<SimulationResponse> {
    let state: State = STATE.load(deps.storage)?;
    let this_price = state
        .asset_prices
        .into_iter()
        .find(|item| match offer_asset.info.clone() {
            AssetInfo::NativeToken { denom } => denom == item.denom,
            AssetInfo::Token { contract_addr } => contract_addr == item.denom,
        });
    match this_price {
        None => Err(StdError::generic_err("Unrecognized asset")),
        Some(asset_price) => Ok(SimulationResponse {
            commission_amount: asset_price.price / Uint128::from(100u128),
            return_amount: asset_price.price - (asset_price.price / Uint128::from(100u128)),
            spread_amount: Uint128::from(100u128),
        }),
    }
}

fn query_reverse_simulation(deps: Deps, offer_asset: Asset) -> StdResult<SimulationResponse> {
    let state: State = STATE.load(deps.storage)?;
    let this_price = state
        .asset_prices
        .into_iter()
        .find(|item| match offer_asset.info.clone() {
            AssetInfo::NativeToken { denom } => denom == item.denom,
            AssetInfo::Token { contract_addr } => contract_addr == item.denom,
        });
    match this_price {
        None => Err(StdError::generic_err("Unrecognized asset")),
        Some(asset_price) => Ok(SimulationResponse {
            commission_amount: Uint128::from(1_000_000_000_000u128) / asset_price.price * Uint128::from(1_000u128) / Uint128::from(100u128),
            return_amount: Uint128::from(1_000_000_000_000u128) / asset_price.price * Uint128::from(1_000u128) - (Uint128::from(1_000_000_000_000u128) / asset_price.price * Uint128::from(1_000u128) / Uint128::from(100u128)),
            spread_amount: Uint128::from(100u128),
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::msg::AssetPrice;

    use super::*;
    use cosmwasm_std::coins;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let price_vec = vec![AssetPrice {
            denom: "ujunox".to_owned(),
            price: Uint128::from(157u128),
        }];
        let msg = InstantiateMsg {
            asset_prices: price_vec,
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn mock_simulation() {
        let mut deps = mock_dependencies();

        let price_vec = vec![
            AssetPrice {
                denom: "ujunox".to_owned(),
                price: Uint128::from(137_000_000u128),
            },
            AssetPrice {
                denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                    .to_owned(),
                price: Uint128::from(30_000_000u128),
            },
            // not a real contract
            AssetPrice {
                denom: "juno1utkr0ep06rkxgsesq6uryug93daklyd6wneesmtvxjkz0xjlte9qdj2s8q".to_owned(),
                price: Uint128::from(1_000u128),
            },
        ];
        let msg = InstantiateMsg {
            asset_prices: price_vec,
        };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // query the prices
        let query_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                    .to_owned(),
            },
            amount: Uint128::from(1_000_000u128), // is irrelevant to the response here,
                                                  // but very relevant on mainnet use with real contract
        };
        let query = query_simulation(deps.as_ref(), query_asset).unwrap();
        assert_eq!(query.commission_amount, Uint128::from(300_000u128));
        assert_eq!(query.return_amount, Uint128::from(29_700_000u128));
        assert_eq!(query.spread_amount, Uint128::from(100u128));

        // query reverse (juno)
        let query_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: "ujunox"
                    .to_owned(),
            },
            amount: Uint128::from(1_000_000u128), // is irrelevant to the response here,
                                                  // but very relevant on mainnet use with real contract
        };
        let query = query_reverse_simulation(deps.as_ref(), query_asset).unwrap();
        assert_eq!(query.commission_amount, Uint128::from(72_990u128));
        assert_eq!(query.return_amount, Uint128::from(7_226_010u128));
        assert_eq!(query.spread_amount, Uint128::from(100u128));
    }
}
