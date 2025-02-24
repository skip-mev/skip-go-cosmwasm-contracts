use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as CosmosSdkCoin;
use cosmwasm_schema::cw_serde;
use ibc_proto::cosmos::base::v1beta1::Coin as IbcCoin;

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

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::Uint128;

    #[test]
    fn test_from_skip_proto_coin_to_cosmos_sdk_proto_coin() {
        let skip_coin = ProtoCoin(cosmwasm_std::Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        });

        let cosmos_sdk_proto_coin: CosmosSdkCoin = skip_coin.into();

        assert_eq!(cosmos_sdk_proto_coin.denom, "uatom");
        assert_eq!(cosmos_sdk_proto_coin.amount, "100");
    }

    #[test]
    fn test_from_skip_proto_coin_to_ibc_proto_coin() {
        let skip_coin = ProtoCoin(cosmwasm_std::Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        });

        let ibc_proto_coin: IbcCoin = skip_coin.into();

        assert_eq!(ibc_proto_coin.denom, "uatom");
        assert_eq!(ibc_proto_coin.amount, "100");
    }
}
