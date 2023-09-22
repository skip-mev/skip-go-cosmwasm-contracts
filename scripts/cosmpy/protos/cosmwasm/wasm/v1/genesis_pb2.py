"""Generated protocol buffer code."""
from google.protobuf.internal import builder as _builder
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
_sym_db = _symbol_database.Default()
from ....gogoproto import gogo_pb2 as gogoproto_dot_gogo__pb2
from ....cosmwasm.wasm.v1 import types_pb2 as cosmwasm_dot_wasm_dot_v1_dot_types__pb2
from ....cosmwasm.wasm.v1 import tx_pb2 as cosmwasm_dot_wasm_dot_v1_dot_tx__pb2
DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(b'\n\x1ecosmwasm/wasm/v1/genesis.proto\x12\x10cosmwasm.wasm.v1\x1a\x14gogoproto/gogo.proto\x1a\x1ccosmwasm/wasm/v1/types.proto\x1a\x19cosmwasm/wasm/v1/tx.proto"\xc1\x04\n\x0cGenesisState\x12.\n\x06params\x18\x01 \x01(\x0b2\x18.cosmwasm.wasm.v1.ParamsB\x04\xc8\xde\x1f\x00\x12>\n\x05codes\x18\x02 \x03(\x0b2\x16.cosmwasm.wasm.v1.CodeB\x17\xc8\xde\x1f\x00\xea\xde\x1f\x0fcodes,omitempty\x12J\n\tcontracts\x18\x03 \x03(\x0b2\x1a.cosmwasm.wasm.v1.ContractB\x1b\xc8\xde\x1f\x00\xea\xde\x1f\x13contracts,omitempty\x12J\n\tsequences\x18\x04 \x03(\x0b2\x1a.cosmwasm.wasm.v1.SequenceB\x1b\xc8\xde\x1f\x00\xea\xde\x1f\x13sequences,omitempty\x12T\n\x08gen_msgs\x18\x05 \x03(\x0b2&.cosmwasm.wasm.v1.GenesisState.GenMsgsB\x1a\xc8\xde\x1f\x00\xea\xde\x1f\x12gen_msgs,omitempty\x1a\xd2\x01\n\x07GenMsgs\x124\n\nstore_code\x18\x01 \x01(\x0b2\x1e.cosmwasm.wasm.v1.MsgStoreCodeH\x00\x12H\n\x14instantiate_contract\x18\x02 \x01(\x0b2(.cosmwasm.wasm.v1.MsgInstantiateContractH\x00\x12@\n\x10execute_contract\x18\x03 \x01(\x0b2$.cosmwasm.wasm.v1.MsgExecuteContractH\x00B\x05\n\x03sum"|\n\x04Code\x12\x1b\n\x07code_id\x18\x01 \x01(\x04B\n\xe2\xde\x1f\x06CodeID\x123\n\tcode_info\x18\x02 \x01(\x0b2\x1a.cosmwasm.wasm.v1.CodeInfoB\x04\xc8\xde\x1f\x00\x12\x12\n\ncode_bytes\x18\x03 \x01(\x0c\x12\x0e\n\x06pinned\x18\x04 \x01(\x08"\xe9\x01\n\x08Contract\x12\x18\n\x10contract_address\x18\x01 \x01(\t\x12;\n\rcontract_info\x18\x02 \x01(\x0b2\x1e.cosmwasm.wasm.v1.ContractInfoB\x04\xc8\xde\x1f\x00\x125\n\x0econtract_state\x18\x03 \x03(\x0b2\x17.cosmwasm.wasm.v1.ModelB\x04\xc8\xde\x1f\x00\x12O\n\x15contract_code_history\x18\x04 \x03(\x0b2*.cosmwasm.wasm.v1.ContractCodeHistoryEntryB\x04\xc8\xde\x1f\x00"4\n\x08Sequence\x12\x19\n\x06id_key\x18\x01 \x01(\x0cB\t\xe2\xde\x1f\x05IDKey\x12\r\n\x05value\x18\x02 \x01(\x04B(Z&github.com/CosmWasm/wasmd/x/wasm/typesb\x06proto3')
_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, globals())
_builder.BuildTopDescriptorsAndMessages(DESCRIPTOR, 'cosmwasm.wasm.v1.genesis_pb2', globals())
if _descriptor._USE_C_DESCRIPTORS == False:
    DESCRIPTOR._options = None
    DESCRIPTOR._serialized_options = b'Z&github.com/CosmWasm/wasmd/x/wasm/types'
    _GENESISSTATE.fields_by_name['params']._options = None
    _GENESISSTATE.fields_by_name['params']._serialized_options = b'\xc8\xde\x1f\x00'
    _GENESISSTATE.fields_by_name['codes']._options = None
    _GENESISSTATE.fields_by_name['codes']._serialized_options = b'\xc8\xde\x1f\x00\xea\xde\x1f\x0fcodes,omitempty'
    _GENESISSTATE.fields_by_name['contracts']._options = None
    _GENESISSTATE.fields_by_name['contracts']._serialized_options = b'\xc8\xde\x1f\x00\xea\xde\x1f\x13contracts,omitempty'
    _GENESISSTATE.fields_by_name['sequences']._options = None
    _GENESISSTATE.fields_by_name['sequences']._serialized_options = b'\xc8\xde\x1f\x00\xea\xde\x1f\x13sequences,omitempty'
    _GENESISSTATE.fields_by_name['gen_msgs']._options = None
    _GENESISSTATE.fields_by_name['gen_msgs']._serialized_options = b'\xc8\xde\x1f\x00\xea\xde\x1f\x12gen_msgs,omitempty'
    _CODE.fields_by_name['code_id']._options = None
    _CODE.fields_by_name['code_id']._serialized_options = b'\xe2\xde\x1f\x06CodeID'
    _CODE.fields_by_name['code_info']._options = None
    _CODE.fields_by_name['code_info']._serialized_options = b'\xc8\xde\x1f\x00'
    _CONTRACT.fields_by_name['contract_info']._options = None
    _CONTRACT.fields_by_name['contract_info']._serialized_options = b'\xc8\xde\x1f\x00'
    _CONTRACT.fields_by_name['contract_state']._options = None
    _CONTRACT.fields_by_name['contract_state']._serialized_options = b'\xc8\xde\x1f\x00'
    _CONTRACT.fields_by_name['contract_code_history']._options = None
    _CONTRACT.fields_by_name['contract_code_history']._serialized_options = b'\xc8\xde\x1f\x00'
    _SEQUENCE.fields_by_name['id_key']._options = None
    _SEQUENCE.fields_by_name['id_key']._serialized_options = b'\xe2\xde\x1f\x05IDKey'
    _GENESISSTATE._serialized_start = 132
    _GENESISSTATE._serialized_end = 709
    _GENESISSTATE_GENMSGS._serialized_start = 499
    _GENESISSTATE_GENMSGS._serialized_end = 709
    _CODE._serialized_start = 711
    _CODE._serialized_end = 835
    _CONTRACT._serialized_start = 838
    _CONTRACT._serialized_end = 1071
    _SEQUENCE._serialized_start = 1073
    _SEQUENCE._serialized_end = 1125