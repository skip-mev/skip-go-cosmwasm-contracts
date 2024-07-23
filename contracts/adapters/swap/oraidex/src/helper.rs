use cosmwasm_std::{Api, Coin, StdError, Uint128};
use cw20::Cw20Coin;
use oraiswap::asset::AssetInfo;
use oraiswap_v3::{percentage::Percentage, FeeTier, PoolKey};
use skip::asset::Asset;

use crate::error::ContractError;

pub fn denom_to_asset_info(api: &dyn Api, denom: &str) -> AssetInfo {
    if let Ok(contract_addr) = api.addr_validate(denom) {
        AssetInfo::Token { contract_addr }
    } else {
        AssetInfo::NativeToken {
            denom: denom.to_string(),
        }
    }
}

pub fn denom_to_asset(api: &dyn Api, denom: &str, amount: Uint128) -> Asset {
    if let Ok(contract_addr) = api.addr_validate(denom) {
        Asset::Cw20(Cw20Coin {
            address: contract_addr.to_string(),
            amount,
        })
    } else {
        Asset::Native(Coin {
            denom: denom.to_string(),
            amount,
        })
    }
}

pub fn convert_pool_id_to_v3_pool_key(pool_id: &str) -> Result<PoolKey, ContractError> {
    //poolID:  tokenX-tokenY-fee-tickSpace
    let parts: Vec<&str> = pool_id.split('-').collect();

    if parts.len() != 4 {
        return Err(ContractError::Std(StdError::generic_err(
            "Invalid v3 pool_id, require exactly 4 fields",
        )));
    }

    let token_x = String::from(parts[0]);
    let token_y = String::from(parts[1]);

    let fee = match parts[2].parse::<u64>() {
        Ok(value) => Percentage(value),
        Err(_) => {
            return Err(ContractError::Std(StdError::generic_err(
                "Invalid fee in v3 pool",
            )))
        }
    };
    let tick_spacing = match parts[3].parse::<u16>() {
        Ok(value) => value,
        Err(_) => {
            return Err(ContractError::Std(StdError::generic_err(
                "Invalid tick_spacing in v3 pool",
            )));
        }
    };

    // Create and return the PoolKey instance
    Ok(PoolKey {
        token_x,
        token_y,
        fee_tier: FeeTier { fee, tick_spacing },
    })
}
