use crate::{
    error::{ContractError, ContractResult},
    state::{
        ACK_ID_TO_RECOVER_ADDRESS, ENTRY_POINT_CONTRACT_ADDRESS, IN_PROGRESS_CHANNEL_ID,
        IN_PROGRESS_RECOVER_ADDRESS,
    },
};
use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, SubMsg, SubMsgResult,
};
use cw2::set_contract_version;
use ibc_proto::ibc::applications::transfer::v1::{MsgTransfer, MsgTransferResponse};
use prost::Message;
use serde_cw_value::Value;
use skip::{
    ibc::{AckID, ExecuteMsg, IbcInfo, IbcLifecycleComplete, InstantiateMsg, MigrateMsg, QueryMsg},
    proto_coin::ProtoCoin,
    sudo::{OsmosisSudoMsg as SudoMsg, SudoType},
};

const IBC_MSG_TRANSFER_TYPE_URL: &str = "/ibc.applications.transfer.v1.MsgTransfer";
const REPLY_ID: u64 = 1;

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

///////////////////
/// INSTANTIATE ///
///////////////////

// Contract name and version used for migration.
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

///////////////
/// EXECUTE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::IbcTransfer {
            info: ibc_info,
            coin,
            timeout_timestamp,
        } => execute_ibc_transfer(deps, env, info, ibc_info, coin, timeout_timestamp),
    }
}

// Converts the given info and coin into an ibc transfer message,
// saves necessary info in case the ibc transfer fails to send funds back to
// a recovery address, and then emits the ibc transfer message as a sub message
fn execute_ibc_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ibc_info: IbcInfo,
    coin: Coin,
    timeout_timestamp: u64,
) -> ContractResult<Response> {
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Error if ibc_info.fee is not None since Osmosis does not support fees
    if ibc_info.fee.is_some() {
        return Err(ContractError::IbcFeesNotSupported);
    }

    // Save in progress recover address to storage, to be used in sudo handler
    IN_PROGRESS_RECOVER_ADDRESS.save(
        deps.storage,
        &ibc_info.recover_address, // This address is verified in entry point
    )?;

    // Save in progress channel id to storage, to be used in sudo handler
    IN_PROGRESS_CHANNEL_ID.save(deps.storage, &ibc_info.source_channel)?;

    // Verify memo is valid json and add the necessary key/value pair to trigger the ibc hooks callback logic.
    let memo = verify_and_create_memo(ibc_info.memo, env.contract.address.to_string())?;

    // Create osmosis ibc transfer message
    let msg = MsgTransfer {
        source_port: "transfer".to_string(),
        source_channel: ibc_info.source_channel,
        token: Some(ProtoCoin(coin).into()),
        sender: env.contract.address.to_string(),
        receiver: ibc_info.receiver,
        timeout_height: None,
        timeout_timestamp,
        memo,
    };

    // Create stargate message from osmosis ibc transfer message
    let msg = CosmosMsg::Stargate {
        type_url: IBC_MSG_TRANSFER_TYPE_URL.to_string(),
        value: msg.encode_to_vec().into(),
    };

    // Create sub message from osmosis ibc transfer message to receive a reply
    let sub_msg = SubMsg::reply_on_success(msg, REPLY_ID);

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "execute_ibc_transfer"))
}

/////////////
/// REPLY ///
/////////////

// Handles the reply from the ibc transfer sub message
// Upon success, maps the sub msg AckID (channel_id, sequence_id)
// to the in progress ibc transfer struct, and saves it to storage.
// Now that the map entry is stored, it also removes the in progress
// ibc transfer from storage.
#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> ContractResult<Response> {
    // Error if the reply id is not the same as the one used in the sub message dispatched
    // This should never happen since we are using a constant reply id, but added in case
    // the wasm module doesn't behave as expected.
    if reply.id != REPLY_ID {
        unreachable!()
    }

    // Get the sub message response from the reply and error if it does not exist
    // This should never happen since sub msg was set to reply on success only,
    // but added in case the wasm module doesn't behave as expected.
    let SubMsgResult::Ok(sub_msg_response) = reply.result else {
        unreachable!()
    };

    // Parse the response from the sub message
    let resp: MsgTransferResponse = MsgTransferResponse::decode(
        sub_msg_response
            .data
            .ok_or(ContractError::MissingResponseData)?
            .as_slice(),
    )?;

    // Get and delete the in progress recover address from storage
    let in_progress_recover_address = IN_PROGRESS_RECOVER_ADDRESS.load(deps.storage)?;
    IN_PROGRESS_RECOVER_ADDRESS.remove(deps.storage);

    // Get and delete the in progress channel id from storage
    let in_progress_channel_id = IN_PROGRESS_CHANNEL_ID.load(deps.storage)?;
    IN_PROGRESS_CHANNEL_ID.remove(deps.storage);

    // Set ack_id to be the channel id and sequence id from the response as a tuple
    let ack_id: AckID = (&in_progress_channel_id, resp.sequence);

    // Error if unique ack_id (channel id, sequence id) already exists in storage
    if ACK_ID_TO_RECOVER_ADDRESS.has(deps.storage, ack_id) {
        return Err(ContractError::AckIDAlreadyExists {
            channel_id: ack_id.0.into(),
            sequence_id: ack_id.1,
        });
    }

    // Set the in progress recover address to storage, keyed by channel id and sequence id
    ACK_ID_TO_RECOVER_ADDRESS.save(deps.storage, ack_id, &in_progress_recover_address)?;

    Ok(Response::new().add_attribute("action", "sub_msg_reply_success"))
}

////////////
/// SUDO ///
////////////

// Handles the ibc callback from the ibc hooks module
// Upon success, removes the in progress ibc transfer from storage and returns immediately.
// Upon error or timeout, sends the attempted ibc transferred funds back to the user's recover address.
#[entry_point]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> ContractResult<Response> {
    // Get the channel id, sequence id, and sudo type from the sudo message
    let (channel, sequence, sudo_type) = match msg {
        SudoMsg::IbcLifecycleComplete(IbcLifecycleComplete::IbcAck {
            channel,
            sequence,
            ack: _,
            success,
        }) => {
            // Remove the AckID <> in progress ibc transfer from storage
            // and return immediately if the ibc transfer was successful
            // since no further action is needed.
            if success {
                let ack_id: AckID = (&channel, sequence);
                ACK_ID_TO_RECOVER_ADDRESS.remove(deps.storage, ack_id);

                return Ok(Response::new().add_attribute("action", SudoType::Response));
            }

            (channel, sequence, SudoType::Error)
        }
        SudoMsg::IbcLifecycleComplete(IbcLifecycleComplete::IbcTimeout { channel, sequence }) => {
            (channel, sequence, SudoType::Timeout)
        }
    };

    // Get and remove the AckID <> in progress recover address from storage
    let ack_id: AckID = (&channel, sequence);
    let to_address = ACK_ID_TO_RECOVER_ADDRESS.load(deps.storage, ack_id)?;
    ACK_ID_TO_RECOVER_ADDRESS.remove(deps.storage, ack_id);

    // Get all coins from contract's balance, which will be the the
    // failed ibc transfer coin and any leftover dust on the contract
    let amount = deps.querier.query_all_balances(env.contract.address)?;

    // If amount is empty, return a no funds to refund error
    if amount.is_empty() {
        return Err(ContractError::NoFundsToRefund);
    }

    // Create bank send message to send funds back to user's recover address
    let bank_send_msg = BankMsg::Send { to_address, amount };

    Ok(Response::new()
        .add_message(bank_send_msg)
        .add_attribute("action", sudo_type))
}

////////////////////////
/// HELPER FUNCTIONS ///
////////////////////////

// Verifies the given memo is empty or valid json, and then adds the necessary
// key/value pair to trigger the ibc hooks callback logic.
fn verify_and_create_memo(memo: String, contract_address: String) -> ContractResult<String> {
    // If the memo given is empty, then set it to "{}" to avoid json parsing errors. Then,
    // get Value object from json string, erroring if the memo was not null while not being valid json
    let mut memo: Value = serde_json_wasm::from_str(if memo.is_empty() { "{}" } else { &memo })?;

    // Transform the Value object into a Value map representation of the json string
    // and insert the necessary key value pair into the memo map to trigger
    // the ibc hooks callback logic. That key value pair is:
    // { "ibc_callback": <CALLBACK_CONTRACT_ADDRESS> }
    //
    // If the "ibc_callback" key was already set, this will override
    // the value with the current contract address.
    if let Value::Map(ref mut memo) = memo {
        memo.insert(
            Value::String("ibc_callback".to_string()),
            Value::String(contract_address),
        )
    } else {
        unreachable!()
    };

    // Transform the memo Value map back into a json string
    let memo = serde_json_wasm::to_string(&memo)?;

    Ok(memo)
}

/////////////
/// QUERY ///
/////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::InProgressRecoverAddress {
            channel_id,
            sequence_id,
        } => to_json_binary(
            &ACK_ID_TO_RECOVER_ADDRESS.load(deps.storage, (&channel_id, sequence_id))?,
        ),
    }
    .map_err(From::from)
}
