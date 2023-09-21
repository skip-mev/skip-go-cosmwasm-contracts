"""Generated protocol buffer code."""
from google.protobuf.internal import builder as _builder
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
_sym_db = _symbol_database.Default()
from ....cosmos.base.v1beta1 import coin_pb2 as cosmos_dot_base_dot_v1beta1_dot_coin__pb2
from ....gogoproto import gogo_pb2 as gogoproto_dot_gogo__pb2
from ....cosmwasm.wasm.v1 import types_pb2 as cosmwasm_dot_wasm_dot_v1_dot_types__pb2
DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(b'\n\x19cosmwasm/wasm/v1/tx.proto\x12\x10cosmwasm.wasm.v1\x1a\x1ecosmos/base/v1beta1/coin.proto\x1a\x14gogoproto/gogo.proto\x1a\x1ccosmwasm/wasm/v1/types.proto"\x94\x01\n\x0cMsgStoreCode\x12\x0e\n\x06sender\x18\x01 \x01(\t\x12(\n\x0ewasm_byte_code\x18\x02 \x01(\x0cB\x10\xe2\xde\x1f\x0cWASMByteCode\x12>\n\x16instantiate_permission\x18\x05 \x01(\x0b2\x1e.cosmwasm.wasm.v1.AccessConfigJ\x04\x08\x03\x10\x04J\x04\x08\x04\x10\x05"E\n\x14MsgStoreCodeResponse\x12\x1b\n\x07code_id\x18\x01 \x01(\x04B\n\xe2\xde\x1f\x06CodeID\x12\x10\n\x08checksum\x18\x02 \x01(\x0c"\xe4\x01\n\x16MsgInstantiateContract\x12\x0e\n\x06sender\x18\x01 \x01(\t\x12\r\n\x05admin\x18\x02 \x01(\t\x12\x1b\n\x07code_id\x18\x03 \x01(\x04B\n\xe2\xde\x1f\x06CodeID\x12\r\n\x05label\x18\x04 \x01(\t\x12#\n\x03msg\x18\x05 \x01(\x0cB\x16\xfa\xde\x1f\x12RawContractMessage\x12Z\n\x05funds\x18\x06 \x03(\x0b2\x19.cosmos.base.v1beta1.CoinB0\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins"\x84\x02\n\x17MsgInstantiateContract2\x12\x0e\n\x06sender\x18\x01 \x01(\t\x12\r\n\x05admin\x18\x02 \x01(\t\x12\x1b\n\x07code_id\x18\x03 \x01(\x04B\n\xe2\xde\x1f\x06CodeID\x12\r\n\x05label\x18\x04 \x01(\t\x12#\n\x03msg\x18\x05 \x01(\x0cB\x16\xfa\xde\x1f\x12RawContractMessage\x12Z\n\x05funds\x18\x06 \x03(\x0b2\x19.cosmos.base.v1beta1.CoinB0\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins\x12\x0c\n\x04salt\x18\x07 \x01(\x0c\x12\x0f\n\x07fix_msg\x18\x08 \x01(\x08"?\n\x1eMsgInstantiateContractResponse\x12\x0f\n\x07address\x18\x01 \x01(\t\x12\x0c\n\x04data\x18\x02 \x01(\x0c"@\n\x1fMsgInstantiateContract2Response\x12\x0f\n\x07address\x18\x01 \x01(\t\x12\x0c\n\x04data\x18\x02 \x01(\x0c"\xb7\x01\n\x12MsgExecuteContract\x12\x0e\n\x06sender\x18\x01 \x01(\t\x12\x10\n\x08contract\x18\x02 \x01(\t\x12#\n\x03msg\x18\x03 \x01(\x0cB\x16\xfa\xde\x1f\x12RawContractMessage\x12Z\n\x05funds\x18\x05 \x03(\x0b2\x19.cosmos.base.v1beta1.CoinB0\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins"*\n\x1aMsgExecuteContractResponse\x12\x0c\n\x04data\x18\x01 \x01(\x0c"x\n\x12MsgMigrateContract\x12\x0e\n\x06sender\x18\x01 \x01(\t\x12\x10\n\x08contract\x18\x02 \x01(\t\x12\x1b\n\x07code_id\x18\x03 \x01(\x04B\n\xe2\xde\x1f\x06CodeID\x12#\n\x03msg\x18\x04 \x01(\x0cB\x16\xfa\xde\x1f\x12RawContractMessage"*\n\x1aMsgMigrateContractResponse\x12\x0c\n\x04data\x18\x01 \x01(\x0c"E\n\x0eMsgUpdateAdmin\x12\x0e\n\x06sender\x18\x01 \x01(\t\x12\x11\n\tnew_admin\x18\x02 \x01(\t\x12\x10\n\x08contract\x18\x03 \x01(\t"\x18\n\x16MsgUpdateAdminResponse"1\n\rMsgClearAdmin\x12\x0e\n\x06sender\x18\x01 \x01(\t\x12\x10\n\x08contract\x18\x03 \x01(\t"\x17\n\x15MsgClearAdminResponse2\xc4\x05\n\x03Msg\x12S\n\tStoreCode\x12\x1e.cosmwasm.wasm.v1.MsgStoreCode\x1a&.cosmwasm.wasm.v1.MsgStoreCodeResponse\x12q\n\x13InstantiateContract\x12(.cosmwasm.wasm.v1.MsgInstantiateContract\x1a0.cosmwasm.wasm.v1.MsgInstantiateContractResponse\x12t\n\x14InstantiateContract2\x12).cosmwasm.wasm.v1.MsgInstantiateContract2\x1a1.cosmwasm.wasm.v1.MsgInstantiateContract2Response\x12e\n\x0fExecuteContract\x12$.cosmwasm.wasm.v1.MsgExecuteContract\x1a,.cosmwasm.wasm.v1.MsgExecuteContractResponse\x12e\n\x0fMigrateContract\x12$.cosmwasm.wasm.v1.MsgMigrateContract\x1a,.cosmwasm.wasm.v1.MsgMigrateContractResponse\x12Y\n\x0bUpdateAdmin\x12 .cosmwasm.wasm.v1.MsgUpdateAdmin\x1a(.cosmwasm.wasm.v1.MsgUpdateAdminResponse\x12V\n\nClearAdmin\x12\x1f.cosmwasm.wasm.v1.MsgClearAdmin\x1a\'.cosmwasm.wasm.v1.MsgClearAdminResponseB,Z&github.com/CosmWasm/wasmd/x/wasm/types\xc8\xe1\x1e\x00b\x06proto3')
_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, globals())
_builder.BuildTopDescriptorsAndMessages(DESCRIPTOR, 'cosmwasm.wasm.v1.tx_pb2', globals())
if _descriptor._USE_C_DESCRIPTORS == False:
    DESCRIPTOR._options = None
    DESCRIPTOR._serialized_options = b'Z&github.com/CosmWasm/wasmd/x/wasm/types\xc8\xe1\x1e\x00'
    _MSGSTORECODE.fields_by_name['wasm_byte_code']._options = None
    _MSGSTORECODE.fields_by_name['wasm_byte_code']._serialized_options = b'\xe2\xde\x1f\x0cWASMByteCode'
    _MSGSTORECODERESPONSE.fields_by_name['code_id']._options = None
    _MSGSTORECODERESPONSE.fields_by_name['code_id']._serialized_options = b'\xe2\xde\x1f\x06CodeID'
    _MSGINSTANTIATECONTRACT.fields_by_name['code_id']._options = None
    _MSGINSTANTIATECONTRACT.fields_by_name['code_id']._serialized_options = b'\xe2\xde\x1f\x06CodeID'
    _MSGINSTANTIATECONTRACT.fields_by_name['msg']._options = None
    _MSGINSTANTIATECONTRACT.fields_by_name['msg']._serialized_options = b'\xfa\xde\x1f\x12RawContractMessage'
    _MSGINSTANTIATECONTRACT.fields_by_name['funds']._options = None
    _MSGINSTANTIATECONTRACT.fields_by_name['funds']._serialized_options = b'\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins'
    _MSGINSTANTIATECONTRACT2.fields_by_name['code_id']._options = None
    _MSGINSTANTIATECONTRACT2.fields_by_name['code_id']._serialized_options = b'\xe2\xde\x1f\x06CodeID'
    _MSGINSTANTIATECONTRACT2.fields_by_name['msg']._options = None
    _MSGINSTANTIATECONTRACT2.fields_by_name['msg']._serialized_options = b'\xfa\xde\x1f\x12RawContractMessage'
    _MSGINSTANTIATECONTRACT2.fields_by_name['funds']._options = None
    _MSGINSTANTIATECONTRACT2.fields_by_name['funds']._serialized_options = b'\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins'
    _MSGEXECUTECONTRACT.fields_by_name['msg']._options = None
    _MSGEXECUTECONTRACT.fields_by_name['msg']._serialized_options = b'\xfa\xde\x1f\x12RawContractMessage'
    _MSGEXECUTECONTRACT.fields_by_name['funds']._options = None
    _MSGEXECUTECONTRACT.fields_by_name['funds']._serialized_options = b'\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins'
    _MSGMIGRATECONTRACT.fields_by_name['code_id']._options = None
    _MSGMIGRATECONTRACT.fields_by_name['code_id']._serialized_options = b'\xe2\xde\x1f\x06CodeID'
    _MSGMIGRATECONTRACT.fields_by_name['msg']._options = None
    _MSGMIGRATECONTRACT.fields_by_name['msg']._serialized_options = b'\xfa\xde\x1f\x12RawContractMessage'
    _MSGSTORECODE._serialized_start = 132
    _MSGSTORECODE._serialized_end = 280
    _MSGSTORECODERESPONSE._serialized_start = 282
    _MSGSTORECODERESPONSE._serialized_end = 351
    _MSGINSTANTIATECONTRACT._serialized_start = 354
    _MSGINSTANTIATECONTRACT._serialized_end = 582
    _MSGINSTANTIATECONTRACT2._serialized_start = 585
    _MSGINSTANTIATECONTRACT2._serialized_end = 845
    _MSGINSTANTIATECONTRACTRESPONSE._serialized_start = 847
    _MSGINSTANTIATECONTRACTRESPONSE._serialized_end = 910
    _MSGINSTANTIATECONTRACT2RESPONSE._serialized_start = 912
    _MSGINSTANTIATECONTRACT2RESPONSE._serialized_end = 976
    _MSGEXECUTECONTRACT._serialized_start = 979
    _MSGEXECUTECONTRACT._serialized_end = 1162
    _MSGEXECUTECONTRACTRESPONSE._serialized_start = 1164
    _MSGEXECUTECONTRACTRESPONSE._serialized_end = 1206
    _MSGMIGRATECONTRACT._serialized_start = 1208
    _MSGMIGRATECONTRACT._serialized_end = 1328
    _MSGMIGRATECONTRACTRESPONSE._serialized_start = 1330
    _MSGMIGRATECONTRACTRESPONSE._serialized_end = 1372
    _MSGUPDATEADMIN._serialized_start = 1374
    _MSGUPDATEADMIN._serialized_end = 1443
    _MSGUPDATEADMINRESPONSE._serialized_start = 1445
    _MSGUPDATEADMINRESPONSE._serialized_end = 1469
    _MSGCLEARADMIN._serialized_start = 1471
    _MSGCLEARADMIN._serialized_end = 1520
    _MSGCLEARADMINRESPONSE._serialized_start = 1522
    _MSGCLEARADMINRESPONSE._serialized_end = 1545
    _MSG._serialized_start = 1548
    _MSG._serialized_end = 2256