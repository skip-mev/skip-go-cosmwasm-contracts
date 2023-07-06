use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use skip::ibc::{AckID, OsmosisInProgressIbcTransfer as InProgressIbcTransfer};

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");
pub const IN_PROGRESS_IBC_TRANSFER: Item<InProgressIbcTransfer> =
    Item::new("in_progress_ibc_transfer");
pub const ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER: Map<AckID, InProgressIbcTransfer> =
    Map::new("ack_id_to_transferer");
