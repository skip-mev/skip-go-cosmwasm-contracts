# Neutron IBC Transfer Adapter Contract

The Neutron IBC Transfer adapter contract is responsible for:
1. Dispatching the IBC transfer with the appropriate IBC fees.
2. Failing the entire transaction if the IBC transfer errors on the swap chain (sending the caller back their original funds).
3. Refunding the caller on the swap chain if the IBC transfer errors or times out once it reaches the destination chain, including refunding unused IBC fees.

WARNING: Do not send funds directly to the contract without calling one of its functions. Funds sent directly to the contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for accidentally sent funds.

## InstantiateMsg

Instantiates a new Neutron IBC Transfer adapter contract.

``` json
{}
```

## ExecuteMsg

### `ibc_transfer`

Dispatches an ICS-20 IBC Transfer given the parameters provided in the contract call.

``` json
{
    "ibc_transfer": {
        "info": {
            "source_channel": "channel-1",
            "receiver": "cosmos...",
            "fee": {
                "recv_fee": [],
                "ack_fee": [
                    {
                        "denom": "untrn",
                        "amount": "100000"
                    }
                ],
                "timeout_fee": [
                    {
                        "denom": "untrn",
                        "amount": "100000"
                    }
                ]
            },
            "memo": "",
            "recover_address": "neutron..."
        },
        "coin": {
            "denom": "uatom",
            "amount": "1000000"
        },
        "timeout_timestamp": 1000000000000
    }
}
```

## QueryMsg

### `in_progress_recover_address`

Returns the in progress recover address associated with the given `channel_id` and `sequence_id` (which make up a unique identifier mapped to in progress ibc transfers in the sub msg reply handler).

Query:
``` json
{
    "in_progress_recover_address": {
        "channel_id": "channel-1",
        "sequence_id": 420
    }
}
```

Response:
``` json
"neutron..."
```