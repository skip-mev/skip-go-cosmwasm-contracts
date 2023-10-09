use crate::{
    error::{ContractError, ContractResult},
    state::{ACK_ID_TO_RECOVER_ADDRESS, ENTRY_POINT_CONTRACT_ADDRESS, IN_PROGRESS_RECOVER_ADDRESS},
};
use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, SubMsg, SubMsgResult,
};
use neutron_proto::neutron::transfer::{MsgTransfer, MsgTransferResponse};
use neutron_sdk::sudo::msg::{RequestPacket, TransferSudoMsg};
use prost::Message;
use skip::{
    ibc::{AckID, ExecuteMsg, IbcInfo, InstantiateMsg, QueryMsg},
    proto_coin::ProtoCoin,
    sudo::SudoType,
};

const REPLY_ID: u64 = 1;

///////////////////
/// INSTANTIATE ///
///////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
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

// Converts the given info and coin into a neutron ibc transfer message,
// saves necessary info in case the ibc transfer fails to send funds back to
// a recovery address, and then emits the neutron ibc transfer message as a sub message
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

    // Error if ibc_info.fee is not Some since they are required on Neutron.
    let ibc_fee = match ibc_info.fee {
        Some(fee) => fee,
        None => return Err(ContractError::IbcFeesRequired),
    };

    // Create neutron ibc transfer message
    let msg = MsgTransfer {
        source_port: "transfer".to_string(),
        source_channel: ibc_info.source_channel,
        token: Some(ProtoCoin(coin).into()),
        sender: env.contract.address.to_string(),
        receiver: ibc_info.receiver,
        timeout_height: None,
        timeout_timestamp,
        memo: ibc_info.memo,
        fee: Some(ibc_fee.into()),
    };

    // Save in progress recover address to storage, to be used in sudo handler
    IN_PROGRESS_RECOVER_ADDRESS.save(
        deps.storage,
        &ibc_info.recover_address, // This address is verified in entry point
    )?;

    // Create sub message from neutron ibc transfer message to receive a reply
    let sub_msg = SubMsg::reply_on_success(msg, REPLY_ID);

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "execute_ibc_transfer"))
}

/////////////
/// REPLY ///
/////////////

// Handles the reply from the neutron ibc transfer sub message
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

    // Set ack_id to be the channel id and sequence id from the response as a tuple
    let ack_id: AckID = (&resp.channel, resp.sequence_id);

    // Get and remove the in progress recover address from storage
    let in_progress_recover_address = IN_PROGRESS_RECOVER_ADDRESS.load(deps.storage)?;
    IN_PROGRESS_RECOVER_ADDRESS.remove(deps.storage);

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

// Handles the sudo acknowledgement from the neutron transfer module upon receiving
// a packet acknowledge form the receiving chain of the ibc transfer

#[entry_point]
pub fn sudo(deps: DepsMut, env: Env, msg: TransferSudoMsg) -> ContractResult<Response> {
    // Get request and sudo type from sudo message
    let (req, sudo_type) = match msg {
        TransferSudoMsg::Response { request, .. } => (request, SudoType::Response),
        TransferSudoMsg::Error { request, .. } => (request, SudoType::Error),
        TransferSudoMsg::Timeout { request } => (request, SudoType::Timeout),
    };

    // Get and remove the AckID <> in progress ibc transfer from storage
    let ack_id = get_ack_id(&req)?;
    let to_address = ACK_ID_TO_RECOVER_ADDRESS.load(deps.storage, ack_id)?;
    ACK_ID_TO_RECOVER_ADDRESS.remove(deps.storage, ack_id);

    // Get all coins from contract's balance, which will be the refunded fee,
    // the failed ibc transfer coin if response is an error or timeout,
    // and any leftover dust on the contract
    let amount = deps.querier.query_all_balances(env.contract.address)?;

    // If amount is empty, return a no funds to refund error
    if amount.is_empty() {
        return Err(ContractError::NoFundsToRefund);
    }

    // Create bank send message to send the contract's funds back
    // to the user's recover address.
    let bank_send_msg = BankMsg::Send { to_address, amount };

    Ok(Response::new()
        .add_message(bank_send_msg)
        .add_attribute("action", sudo_type))
}

////////////////////////
/// HELPER FUNCTIONS ///
////////////////////////

// Helper function to get the ack_id (channel id, sequence id) from a RequestPacket
fn get_ack_id(req: &RequestPacket) -> ContractResult<AckID> {
    // Get the channel id and sequence id from the request packet
    let channel_id = req
        .source_channel
        .as_ref()
        .ok_or(ContractError::ChannelIDNotFound)?;
    let seq_id = req.sequence.ok_or(ContractError::SequenceNotFound)?;

    // Return the ack_id as a tuple of the channel id and sequence id
    Ok((channel_id, seq_id))
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
        } => to_binary(&ACK_ID_TO_RECOVER_ADDRESS.load(deps.storage, (&channel_id, sequence_id))?),
    }
    .map_err(From::from)
}
