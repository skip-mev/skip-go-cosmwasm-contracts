use cw_storage_plus::{Item, Map};
use skip::ibc::{AckID, NeutronInProgressIbcTransfer as InProgressIbcTransfer};

pub const IN_PROGRESS_IBC_TRANSFER: Item<InProgressIbcTransfer> =
    Item::new("in_progress_ibc_transfer");
pub const ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER: Map<AckID, InProgressIbcTransfer> =
    Map::new("ack_id_to_transferer");
