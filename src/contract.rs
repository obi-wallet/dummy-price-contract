#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    Asset, AssetInfo, ExecuteMsg, InstantiateMsg, QueryMsg, ReverseSimulationResponse,
    SimulationResponse, Token1ForToken2Response, Token2ForToken1Response,
};
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
        QueryMsg::ReverseSimulation { ask_asset } => {
            to_binary(&query_reverse_simulation(deps, ask_asset)?)
        }
        QueryMsg::Token1ForToken2Price(msg) => to_binary(&juno_style_swap1(
            deps,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ujunox".to_string(),
                },
                amount: msg.token1_amount,
            },
        )?),
        QueryMsg::Token2ForToken1Price(msg) => to_binary(&juno_style_swap2(
            deps,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                        .to_string(),
                },
                amount: msg.token2_amount,
            },
        )?),
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
        Some(asset_price) => {
            let base_amount =
                asset_price.price.checked_mul(offer_asset.amount)? / Uint128::from(1_000_000u128);
            Ok(SimulationResponse {
                commission_amount: base_amount / Uint128::from(100u128),
                return_amount: base_amount.saturating_sub(base_amount / Uint128::from(100u128)),
                spread_amount: Uint128::from(100u128),
            })
        }
    }
}

fn juno_style_swap1(deps: Deps, known_asset: Asset) -> StdResult<Token1ForToken2Response> {
    Ok(Token1ForToken2Response {
        token2_amount: juno_style_swap(deps, known_asset)?,
    })
}

fn juno_style_swap2(deps: Deps, known_asset: Asset) -> StdResult<Token2ForToken1Response> {
    Ok(Token2ForToken1Response {
        token1_amount: juno_style_swap(deps, known_asset)?,
    })
}

fn juno_style_swap(deps: Deps, known_asset: Asset) -> StdResult<Uint128> {
    let state: State = STATE.load(deps.storage)?;
    // kludge sim usdc <> junox
    let usdc_price = state.asset_prices.clone().into_iter().find(|item| {
        item.denom
            == *"ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
    });
    let junox_price = state
        .asset_prices
        .into_iter()
        .find(|item| item.denom == *"ujunox");
    if usdc_price == None {
        return Err(StdError::generic_err("USDC<>DEX price not set"));
    }
    if junox_price == None {
        return Err(StdError::generic_err("USDC<>DEX price not set"));
    }
    let known_asset_denom = match known_asset.info {
        AssetInfo::NativeToken { denom } => denom,
        AssetInfo::Token { contract_addr } => contract_addr,
    };
    let base_amount = match known_asset_denom {
        val if val == *"ujunox" => {
            let dex_amount = known_asset.amount.checked_mul(junox_price.unwrap().price)?; // loop total
            dex_amount
                .checked_mul(Uint128::from(1_000_000u128))?
                .checked_div(usdc_price.unwrap().price)?
                .checked_div(Uint128::from(1_000_000u128))?
        }
        val if val
            == *"ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034" =>
        {
            let dex_amount = known_asset.amount.checked_mul(usdc_price.unwrap().price)?; // loop total
            dex_amount
                .checked_mul(Uint128::from(1_000_000u128))?
                .checked_div(junox_price.unwrap().price)?
                .checked_div(Uint128::from(1_000_000u128))?
        }
        _ => {
            return Err(StdError::GenericErr {
                msg: "invalid juno-type swap assets".to_string(),
            });
        }
    };
    Ok(base_amount)
}

fn query_reverse_simulation(deps: Deps, ask_asset: Asset) -> StdResult<ReverseSimulationResponse> {
    let state: State = STATE.load(deps.storage)?;
    let this_price = state
        .asset_prices
        .into_iter()
        .find(|item| match ask_asset.info.clone() {
            AssetInfo::NativeToken { denom } => denom == item.denom,
            AssetInfo::Token { contract_addr } => contract_addr == item.denom,
        });
    match this_price {
        None => Err(StdError::generic_err("Unrecognized asset")),
        Some(asset_price) => {
            let target_amount = asset_price
                .price
                .checked_mul(Uint128::from(1_000_000u128))?
                .checked_div(ask_asset.amount)?;
            Ok(ReverseSimulationResponse {
                commission_amount: target_amount / Uint128::from(100u128),
                offer_amount: target_amount.saturating_sub(target_amount / Uint128::from(100u128)),
                spread_amount: Uint128::from(100u128),
            })
        }
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
            amount: Uint128::from(1_000_000u128),
        };
        let query = query_simulation(deps.as_ref(), query_asset).unwrap();
        println!("query gives {:?}", query);
        assert_eq!(query.commission_amount, Uint128::from(300_000u128));
        assert_eq!(query.return_amount, Uint128::from(29_700_000u128));
        assert_eq!(query.spread_amount, Uint128::from(100u128));

        // query reverse (juno)
        let query_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: "ujunox".to_owned(),
            },
            amount: Uint128::from(1_000_000u128),
        };
        let query = query_reverse_simulation(deps.as_ref(), query_asset).unwrap();
        println!("query reverse gives {:?}", query);
        assert_eq!(query.commission_amount, Uint128::from(1_370_000u128));
        assert_eq!(query.offer_amount, Uint128::from(135_630_000u128));
        assert_eq!(query.spread_amount, Uint128::from(100u128));

        // query JunoSwap style (juno->usdc)
        let query_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: "ujunox".to_owned(),
            },
            amount: Uint128::from(2_000_000u128),
        };
        let query = juno_style_swap1(deps.as_ref(), query_asset).unwrap();
        println!("junoswap query gives {:?}", query);
        assert_eq!(query.token2_amount, Uint128::from(9_133_333u128));

        // query JunoSwap style (usdc->juno)
        let query_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                    .to_owned(),
            },
            amount: Uint128::from(20_000_000u128),
        };
        let query = juno_style_swap2(deps.as_ref(), query_asset).unwrap();
        println!("junoswap query reverse gives {:?}", query);
        assert_eq!(query.token1_amount, Uint128::from(4_379_562u128));
    }
}
