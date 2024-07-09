use cosmwasm_std::coin;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env,
};

use skip::asset::Asset;
use skip::swap::{QueryMsg, SwapOperation};
use skip_api_swap_adapter_pryzm::contract;

#[test]
fn test_simulate() {
    let mut deps = mock_dependencies();
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSwapExactAssetIn {
            asset_in: Asset::Native(coin(1000, "uatom")),
            swap_operations: vec![SwapOperation {
                pool: "icstaking:uatom:channel-0".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "c:uatom".to_string(),
                interface: None,
            }],
        },
    );
    assert!(res.is_ok())
}
