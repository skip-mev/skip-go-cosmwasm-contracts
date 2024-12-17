use crate::{
    asset::{Asset, Snip20ReceiveMsg},
    error::SkipError,
};

use std::{convert::TryFrom, num::ParseIntError};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, ContractInfo, Decimal, Uint128};

///////////////
/// MIGRATE ///
///////////////

// The MigrateMsg struct defines the migration parameters used.
#[cw_serde]
pub struct MigrateMsg {
    pub entry_point_contract_address: String,
}

///////////////////
/// INSTANTIATE ///
///////////////////

// The InstantiateMsg struct defines the initialization parameters for the
// Osmosis Poolmanager and Astroport swap adapter contracts.
#[cw_serde]
pub struct InstantiateMsg {
    pub entry_point_contract_address: String,
}

/////////////////////////
///      EXECUTE      ///
/////////////////////////

// The ExecuteMsg enum defines the execution message that the swap adapter contracts can handle.
// Only the Swap message is callable by external users.
#[cw_serde]
pub enum ExecuteMsg {
    Receive(Snip20ReceiveMsg),
    Swap { operations: Vec<SwapOperation> },
    TransferFundsBack { swapper: Addr, return_denom: String },
}

#[cw_serde]
pub enum Cw20HookMsg {
    Swap { operations: Vec<SwapOperation> },
}

/////////////////////////
///       QUERY       ///
/////////////////////////

// The QueryMsg enum defines the queries the swap adapter contracts provide.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // SimulateSwapExactAssetOut returns the asset in necessary to receive the specified asset out
    #[returns(Asset)]
    SimulateSwapExactAssetOut {
        asset_out: Asset,
        swap_operations: Vec<SwapOperation>,
    },
    // SimulateSwapExactAssetIn returns the asset out received from the specified asset in
    #[returns(Asset)]
    SimulateSwapExactAssetIn {
        asset_in: Asset,
        swap_operations: Vec<SwapOperation>,
    },
    // SimulateSwapExactAssetOutWithSpotPrice returns the asset in necessary to receive the specified asset out with metadata
    #[returns(SimulateSwapExactAssetOutResponse)]
    SimulateSwapExactAssetOutWithMetadata {
        asset_out: Asset,
        swap_operations: Vec<SwapOperation>,
        include_spot_price: bool,
    },
    // SimulateSwapExactAssetInWithSpotPrice returns the asset out received from the specified asset in with metadata
    #[returns(SimulateSwapExactAssetInResponse)]
    SimulateSwapExactAssetInWithMetadata {
        asset_in: Asset,
        swap_operations: Vec<SwapOperation>,
        include_spot_price: bool,
    },
    // SimulateSmartSwapExactAssetIn returns the asset out received from the specified asset in over multiple routes
    #[returns(Asset)]
    SimulateSmartSwapExactAssetIn { asset_in: Asset, routes: Vec<Route> },
    // SimulateSmartSwapExactAssetInWithMetadata returns the asset out received from the specified asset in over multiple routes with metadata
    #[returns(SimulateSmartSwapExactAssetInResponse)]
    SimulateSmartSwapExactAssetInWithMetadata {
        asset_in: Asset,
        routes: Vec<Route>,
        include_spot_price: bool,
    },
}

// The SimulateSwapExactAssetInResponse struct defines the response for the
// SimulateSwapExactAssetIn query.
#[cw_serde]
pub struct SimulateSwapExactAssetInResponse {
    pub asset_out: Asset,
    pub spot_price: Option<Decimal>,
}

// The SimulateSwapExactAssetOutResponse struct defines the response for the
// SimulateSwapExactAssetOut query.
#[cw_serde]
pub struct SimulateSwapExactAssetOutResponse {
    pub asset_in: Asset,
    pub spot_price: Option<Decimal>,
}

// The SimulateSmartSwapExactAssetInResponse struct defines the response for the
// SimulateSmartSwapExactAssetIn query.
#[cw_serde]
pub struct SimulateSmartSwapExactAssetInResponse {
    pub asset_out: Asset,
    pub spot_price: Option<Decimal>,
}

////////////////////
/// COMMON TYPES ///
////////////////////

// Swap venue object that contains the name of the swap venue and adapter contract address.
#[cw_serde]
pub struct SwapVenue {
    pub name: String,
    pub adapter_contract: ContractInfo,
}

#[cw_serde]
pub struct Route {
    pub offer_asset: Asset,
    pub operations: Vec<SwapOperation>,
}

impl Route {
    pub fn ask_denom(&self) -> Result<String, SkipError> {
        match self.operations.last() {
            Some(op) => Ok(op.denom_out.clone()),
            None => Err(SkipError::SwapOperationsEmpty),
        }
    }
}

pub fn get_ask_denom_for_routes(routes: &[Route]) -> Result<String, SkipError> {
    match routes.last() {
        Some(route) => route.ask_denom(),
        None => Err(SkipError::RoutesEmpty),
    }
}

// Standard swap operation type that contains the pool, denom in, and denom out
// for the swap operation. The type is converted into the respective swap venues
// expected format in each adapter contract.
#[cw_serde]
pub struct SwapOperation {
    pub pool: String,
    pub denom_in: String,
    pub denom_out: String,
    pub interface: Option<Binary>,
}

// Converts a vector of skip swap operation to vector of osmosis swap
// amount in/out routes, returning an error if any of the swap operations
// fail to convert. This only happens if the given String for pool in the
// swap operation is not a valid u64, which is the pool_id type for Osmosis.
pub fn convert_swap_operations<T>(
    swap_operations: Vec<SwapOperation>,
) -> Result<Vec<T>, ParseIntError>
where
    T: TryFrom<SwapOperation, Error = ParseIntError>,
{
    swap_operations.into_iter().map(T::try_from).collect()
}

// Swap object to get the exact amount of a given asset with the given vector of swap operations
#[cw_serde]
pub struct SwapExactAssetOut {
    pub swap_venue_name: String,
    pub operations: Vec<SwapOperation>,
    pub refund_address: Option<String>,
}

// Swap object that swaps the remaining asset recevied
// from the contract call minus fee swap (if present)
#[cw_serde]
pub struct SwapExactAssetIn {
    pub swap_venue_name: String,
    pub operations: Vec<SwapOperation>,
}

// Swap object that swaps the remaining asset recevied
// over multiple routes from the contract call minus fee swap (if present)
#[cw_serde]
pub struct SmartSwapExactAssetIn {
    pub swap_venue_name: String,
    pub routes: Vec<Route>,
}

impl SmartSwapExactAssetIn {
    pub fn amount(&self) -> Uint128 {
        self.routes
            .iter()
            .map(|route| route.offer_asset.amount())
            .sum()
    }

    pub fn ask_denom(&self) -> Result<String, SkipError> {
        match self.routes.last() {
            Some(route) => route.ask_denom(),
            None => Err(SkipError::RoutesEmpty),
        }
    }

    pub fn largest_route_index(&self) -> Result<usize, SkipError> {
        match self
            .routes
            .iter()
            .enumerate()
            .max_by_key(|(_, route)| route.offer_asset.amount())
            .map(|(index, _)| index)
        {
            Some(idx) => Ok(idx),
            None => Err(SkipError::RoutesEmpty),
        }
    }
}

// Converts a SwapExactAssetOut used in the entry point contract
// to a swap adapter Swap execute message
impl From<SwapExactAssetOut> for ExecuteMsg {
    fn from(swap: SwapExactAssetOut) -> Self {
        ExecuteMsg::Swap {
            operations: swap.operations,
        }
    }
}

// Converts a SwapExactAssetIn used in the entry point contract
// to a swap adapter Swap execute message
impl From<SwapExactAssetIn> for ExecuteMsg {
    fn from(swap: SwapExactAssetIn) -> Self {
        ExecuteMsg::Swap {
            operations: swap.operations,
        }
    }
}

#[cw_serde]
pub enum Swap {
    SwapExactAssetIn(SwapExactAssetIn),
    SwapExactAssetOut(SwapExactAssetOut),
    SmartSwapExactAssetIn(SmartSwapExactAssetIn),
}

////////////////////////
/// COMMON FUNCTIONS ///
////////////////////////

// Query the contract's balance and transfer the funds back to the swapper
/*
pub fn execute_transfer_funds_back(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    swapper: Addr,
    return_denom: String,
) -> Result<Response, SkipError> {
    // Ensure the caller is the contract itself
    if info.sender != env.contract.address {
        return Err(SkipError::Unauthorized);
    }
    // Create the transfer funds back message
    let transfer_funds_back_msg: CosmosMsg = match deps.api.addr_validate(&return_denom) {
        Ok(contract_addr) => Asset::new(
            deps.api,
            contract_addr.as_str(),
            Cw20Contract(contract_addr.clone()).balance(&deps.querier, &env.contract.address)?,
        )
        .transfer(swapper.as_str()),
        Err(_) => CosmosMsg::Bank(BankMsg::Send {
            to_address: swapper.to_string(),
            amount: deps.querier.query_all_balances(env.contract.address)?,
        }),
    };

    Ok(Response::new()
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_transfer_funds_back_bank_send"))
}
*/

// Validates the swap operations
pub fn validate_swap_operations(
    swap_operations: &[SwapOperation],
    asset_in_denom: &str,
    asset_out_denom: &str,
) -> Result<(), SkipError> {
    // Verify the swap operations are not empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(SkipError::SwapOperationsEmpty);
    };

    // Verify the first swap operation denom in is the same as the asset in denom
    if first_op.denom_in != asset_in_denom {
        return Err(SkipError::SwapOperationsAssetInDenomMismatch);
    }

    // Verify the last swap operation denom out is the same as the asset out denom
    if last_op.denom_out != asset_out_denom {
        return Err(SkipError::SwapOperationsAssetOutDenomMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn test_validate_swap_operations() {
        // TEST CASE 1: Valid Swap Operations
        let swap_operations = vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "uosmo".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "untrn".to_string(),
                interface: None,
            },
        ];

        let asset_in_denom = "uatom";
        let asset_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, asset_in_denom, asset_out_denom);

        assert!(result.is_ok());

        // TEST CASE 2: Empty Swap Operations
        let swap_operations: Vec<SwapOperation> = vec![];

        let asset_in_denom = "uatom";
        let asset_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, asset_in_denom, asset_out_denom);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SkipError::SwapOperationsEmpty);

        // TEST CASE 3: First Swap Operation Denom In Mismatch
        let swap_operations = vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "untrn".to_string(),
                interface: None,
            },
        ];

        let asset_in_denom = "uatom";
        let asset_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, asset_in_denom, asset_out_denom);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            SkipError::SwapOperationsAssetInDenomMismatch
        );

        // TEST CASE 4: Last Swap Operation Denom Out Mismatch
        let swap_operations = vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "uosmo".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            },
        ];

        let asset_in_denom = "uatom";
        let asset_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, asset_in_denom, asset_out_denom);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            SkipError::SwapOperationsAssetOutDenomMismatch
        );
    }
}
