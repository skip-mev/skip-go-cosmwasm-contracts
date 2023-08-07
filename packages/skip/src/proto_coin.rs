use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as CosmosSdkCoin;
use cosmwasm_schema::cw_serde;
use ibc_proto::cosmos::base::v1beta1::Coin as IbcCoin;
use osmosis_std::types::cosmos::base::v1beta1::Coin as OsmosisStdCoin;

// Skip wrapper coin type that is used to wrap cosmwasm_std::Coin
// and be able to implement type conversions on the wrapped type.
#[cw_serde]
pub struct ProtoCoin(pub cosmwasm_std::Coin);

// Converts a skip coin to a cosmos_sdk_proto coin
impl From<ProtoCoin> for CosmosSdkCoin {
    fn from(coin: ProtoCoin) -> Self {
        // Convert the skip coin to a cosmos_sdk_proto coin and return it
        CosmosSdkCoin {
            denom: coin.0.denom.clone(),
            amount: coin.0.amount.to_string(),
        }
    }
}

// Converts a skip coin to an ibc_proto coin
impl From<ProtoCoin> for IbcCoin {
    fn from(coin: ProtoCoin) -> Self {
        // Convert the skip coin to an ibc_proto coin and return it
        IbcCoin {
            denom: coin.0.denom,
            amount: coin.0.amount.to_string(),
        }
    }
}

// Converts a skip coin to an osmosis_std coin
impl From<ProtoCoin> for OsmosisStdCoin {
    fn from(coin: ProtoCoin) -> Self {
        // Convert the skip coin to an osmosis coin and return it
        OsmosisStdCoin {
            denom: coin.0.denom,
            amount: coin.0.amount.to_string(),
        }
    }
}
