use crate::{
    error::{ContractError, ContractResult},
    state::{ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER, IN_PROGRESS_IBC_TRANSFER},
};
use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, SubMsg, SubMsgResult,
};
use neutron_proto::neutron::transfer::{MsgTransfer, MsgTransferResponse};
use neutron_sdk::sudo::msg::{RequestPacket, TransferSudoMsg};
use prost::Message;
use skip::{
    ibc::{
        AckID, ExecuteMsg, IbcInfo, InstantiateMsg,
        NeutronInProgressIbcTransfer as InProgressIbcTransfer, NeutronQueryMsg as QueryMsg,
    },
    proto_coin::ProtoCoin,
    sudo::SudoType,
};

const REPLY_ID: u64 = 1;

///////////////////
/// INSTANTIATE ///
///////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> ContractResult<Response> {
    Ok(Response::new().add_attribute("action", "instantiate"))
}

///////////////
/// EXECUTE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::IbcTransfer {
            info,
            coin,
            timeout_timestamp,
        } => execute_ibc_transfer(deps, env, info, coin, timeout_timestamp),
    }
}

// Converts the given info and coin into a neutron ibc transfer message,
// saves necessary info in case the ibc transfer fails to send funds back to
// a recovery address, and then emits the neutron ibc transfer message as a sub message
fn execute_ibc_transfer(
    deps: DepsMut,
    env: Env,
    info: IbcInfo,
    coin: Coin,
    timeout_timestamp: u64,
) -> ContractResult<Response> {
    // Create neutron ibc transfer message
    let msg = MsgTransfer {
        source_port: "transfer".to_string(),
        source_channel: info.source_channel,
        token: Some(ProtoCoin(coin.clone()).into()),
        sender: env.contract.address.to_string(),
        receiver: info.receiver,
        timeout_height: None,
        timeout_timestamp,
        memo: info.memo,
        fee: Some(info.fee.clone().into()),
    };

    // Save in progress ibc transfer data (recover address and coin) to storage, to be used in sudo handler
    IN_PROGRESS_IBC_TRANSFER.save(
        deps.storage,
        &InProgressIbcTransfer {
            recover_address: info.recover_address, // This address is verified in entry point
            coin,
            ack_fee: info.fee.ack_fee,
            timeout_fee: info.fee.timeout_fee,
        },
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

    // Get the in progress ibc transfer from storage
    let in_progress_ibc_transfer = IN_PROGRESS_IBC_TRANSFER.load(deps.storage)?;

    // Error if unique ack_id (channel id, sequence id) already exists in storage
    if ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER.has(deps.storage, ack_id) {
        return Err(ContractError::AckIDAlreadyExists {
            channel_id: ack_id.0.into(),
            sequence_id: ack_id.1,
        });
    }

    // Set the in progress ibc transfer to storage, keyed by channel id and sequence id
    ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER.save(deps.storage, ack_id, &in_progress_ibc_transfer)?;

    // Delete the in progress ibc transfer from storage
    IN_PROGRESS_IBC_TRANSFER.remove(deps.storage);

    Ok(Response::new().add_attribute("action", "sub_msg_reply_success"))
}

////////////
/// SUDO ///
////////////

// Handles the sudo acknowledgement from the neutron transfer module upon receiving
// a packet acknowledge form the receiving chain of the ibc transfer
#[entry_point]
pub fn sudo(deps: DepsMut, _env: Env, msg: TransferSudoMsg) -> ContractResult<Response> {
    // Get request and sudo type from sudo message
    let (req, sudo_type) = match msg {
        TransferSudoMsg::Response { request, .. } => (request, SudoType::Response),
        TransferSudoMsg::Error { request, .. } => (request, SudoType::Error),
        TransferSudoMsg::Timeout { request } => (request, SudoType::Timeout),
    };

    // Get ack id (channel id, sequence id) from request packet
    let ack_id = get_ack_id(&req)?;

    // Get in progress ibc transfer from storage
    let in_progress_ibc_transfer = ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER.load(deps.storage, ack_id)?;

    // Create bank transfer message to send funds back to user's recover address
    // based on the sudo type:
    // - Response: send the refunded timeout fee back to the user's recover address
    // - Error: send the failed ibc transferred coin + refunded timeout fee back to the user's recover address
    // - Timeout: send the failed ibc transferred coin + refunded ack fee back to the user's recover address
    let amount = match sudo_type {
        SudoType::Response => in_progress_ibc_transfer.timeout_fee,
        SudoType::Error => {
            // Create a single vector of coins to bank send
            // that merges the failed ibc transfer coin with
            // the timeout fee vector of coins
            add_coin_to_vec_coins(
                in_progress_ibc_transfer.coin,
                in_progress_ibc_transfer.timeout_fee,
            )?
        }
        SudoType::Timeout => {
            // Create a single vector of coins to bank send
            // that merges the failed ibc transfer coin with
            // the ack fee vector of coins
            add_coin_to_vec_coins(
                in_progress_ibc_transfer.coin,
                in_progress_ibc_transfer.ack_fee,
            )?
        }
    };

    // Create bank send message to send funds back to user's recover address
    // This will error if the contract balances are insufficient for the stored
    // amount of coins to send (refunded fee + ibc transfer coin if failed).
    // This error should never happen since the contract should have enough
    // funds if Neutron sudo call works as expected. If it does, the failure
    // will be stored in Neutron's ContractManager module, allowing us to query
    // and investigate the failure.
    let bank_send_msg = BankMsg::Send {
        to_address: in_progress_ibc_transfer.recover_address,
        amount,
    };

    // Remove ack id <> in progress ibc transfer entry from storage
    ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER.remove(deps.storage, ack_id);

    Ok(Response::new()
        .add_message(bank_send_msg)
        .add_attribute("action", sudo_type))
}

////////////////////////
/// HELPER FUNCTIONS ///
////////////////////////

// Helper function that adds a coin to a vector of coins, and returns the vector.
fn add_coin_to_vec_coins(coin_to_add: Coin, mut coins: Vec<Coin>) -> ContractResult<Vec<Coin>> {
    // Iterate through the coins vector and add the coin_to_add to the coin in the vector
    match coins
        .iter_mut()
        .find(|coin| coin.denom == coin_to_add.denom)
    {
        Some(coin) => {
            coin.amount = coin.amount.checked_add(coin_to_add.amount)?;
        }
        None => {
            coins.push(coin_to_add);
        }
    }

    Ok(coins)
}

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
        QueryMsg::InProgressIbcTransfer {
            channel_id,
            sequence_id,
        } => to_binary(
            &ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER.load(deps.storage, (&channel_id, sequence_id))?,
        ),
    }
    .map_err(From::from)
}
