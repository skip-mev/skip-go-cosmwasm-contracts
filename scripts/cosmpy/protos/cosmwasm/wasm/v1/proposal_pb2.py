"""Generated protocol buffer code."""
from google.protobuf.internal import builder as _builder
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
_sym_db = _symbol_database.Default()
from ....gogoproto import gogo_pb2 as gogoproto_dot_gogo__pb2
from ....cosmos.base.v1beta1 import coin_pb2 as cosmos_dot_base_dot_v1beta1_dot_coin__pb2
from ....cosmwasm.wasm.v1 import types_pb2 as cosmwasm_dot_wasm_dot_v1_dot_types__pb2
DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(b'\n\x1fcosmwasm/wasm/v1/proposal.proto\x12\x10cosmwasm.wasm.v1\x1a\x14gogoproto/gogo.proto\x1a\x1ecosmos/base/v1beta1/coin.proto\x1a\x1ccosmwasm/wasm/v1/types.proto"\x85\x02\n\x11StoreCodeProposal\x12\r\n\x05title\x18\x01 \x01(\t\x12\x13\n\x0bdescription\x18\x02 \x01(\t\x12\x0e\n\x06run_as\x18\x03 \x01(\t\x12(\n\x0ewasm_byte_code\x18\x04 \x01(\x0cB\x10\xe2\xde\x1f\x0cWASMByteCode\x12>\n\x16instantiate_permission\x18\x07 \x01(\x0b2\x1e.cosmwasm.wasm.v1.AccessConfig\x12\x12\n\nunpin_code\x18\x08 \x01(\x08\x12\x0e\n\x06source\x18\t \x01(\t\x12\x0f\n\x07builder\x18\n \x01(\t\x12\x11\n\tcode_hash\x18\x0b \x01(\x0cJ\x04\x08\x05\x10\x06J\x04\x08\x06\x10\x07"\x8d\x02\n\x1bInstantiateContractProposal\x12\r\n\x05title\x18\x01 \x01(\t\x12\x13\n\x0bdescription\x18\x02 \x01(\t\x12\x0e\n\x06run_as\x18\x03 \x01(\t\x12\r\n\x05admin\x18\x04 \x01(\t\x12\x1b\n\x07code_id\x18\x05 \x01(\x04B\n\xe2\xde\x1f\x06CodeID\x12\r\n\x05label\x18\x06 \x01(\t\x12#\n\x03msg\x18\x07 \x01(\x0cB\x16\xfa\xde\x1f\x12RawContractMessage\x12Z\n\x05funds\x18\x08 \x03(\x0b2\x19.cosmos.base.v1beta1.CoinB0\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins"\x91\x01\n\x17MigrateContractProposal\x12\r\n\x05title\x18\x01 \x01(\t\x12\x13\n\x0bdescription\x18\x02 \x01(\t\x12\x10\n\x08contract\x18\x04 \x01(\t\x12\x1b\n\x07code_id\x18\x05 \x01(\x04B\n\xe2\xde\x1f\x06CodeID\x12#\n\x03msg\x18\x06 \x01(\x0cB\x16\xfa\xde\x1f\x12RawContractMessage"q\n\x14SudoContractProposal\x12\r\n\x05title\x18\x01 \x01(\t\x12\x13\n\x0bdescription\x18\x02 \x01(\t\x12\x10\n\x08contract\x18\x03 \x01(\t\x12#\n\x03msg\x18\x04 \x01(\x0cB\x16\xfa\xde\x1f\x12RawContractMessage"\xe0\x01\n\x17ExecuteContractProposal\x12\r\n\x05title\x18\x01 \x01(\t\x12\x13\n\x0bdescription\x18\x02 \x01(\t\x12\x0e\n\x06run_as\x18\x03 \x01(\t\x12\x10\n\x08contract\x18\x04 \x01(\t\x12#\n\x03msg\x18\x05 \x01(\x0cB\x16\xfa\xde\x1f\x12RawContractMessage\x12Z\n\x05funds\x18\x06 \x03(\x0b2\x19.cosmos.base.v1beta1.CoinB0\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins"t\n\x13UpdateAdminProposal\x12\r\n\x05title\x18\x01 \x01(\t\x12\x13\n\x0bdescription\x18\x02 \x01(\t\x12\'\n\tnew_admin\x18\x03 \x01(\tB\x14\xf2\xde\x1f\x10yaml:"new_admin"\x12\x10\n\x08contract\x18\x04 \x01(\t"J\n\x12ClearAdminProposal\x12\r\n\x05title\x18\x01 \x01(\t\x12\x13\n\x0bdescription\x18\x02 \x01(\t\x12\x10\n\x08contract\x18\x03 \x01(\t"\x92\x01\n\x10PinCodesProposal\x12\x1f\n\x05title\x18\x01 \x01(\tB\x10\xf2\xde\x1f\x0cyaml:"title"\x12+\n\x0bdescription\x18\x02 \x01(\tB\x16\xf2\xde\x1f\x12yaml:"description"\x120\n\x08code_ids\x18\x03 \x03(\x04B\x1e\xe2\xde\x1f\x07CodeIDs\xf2\xde\x1f\x0fyaml:"code_ids""\x94\x01\n\x12UnpinCodesProposal\x12\x1f\n\x05title\x18\x01 \x01(\tB\x10\xf2\xde\x1f\x0cyaml:"title"\x12+\n\x0bdescription\x18\x02 \x01(\tB\x16\xf2\xde\x1f\x12yaml:"description"\x120\n\x08code_ids\x18\x03 \x03(\x04B\x1e\xe2\xde\x1f\x07CodeIDs\xf2\xde\x1f\x0fyaml:"code_ids""w\n\x12AccessConfigUpdate\x12\x1b\n\x07code_id\x18\x01 \x01(\x04B\n\xe2\xde\x1f\x06CodeID\x12D\n\x16instantiate_permission\x18\x02 \x01(\x0b2\x1e.cosmwasm.wasm.v1.AccessConfigB\x04\xc8\xde\x1f\x00"\xba\x01\n\x1fUpdateInstantiateConfigProposal\x12\x1f\n\x05title\x18\x01 \x01(\tB\x10\xf2\xde\x1f\x0cyaml:"title"\x12+\n\x0bdescription\x18\x02 \x01(\tB\x16\xf2\xde\x1f\x12yaml:"description"\x12I\n\x15access_config_updates\x18\x03 \x03(\x0b2$.cosmwasm.wasm.v1.AccessConfigUpdateB\x04\xc8\xde\x1f\x00"\xaa\x03\n#StoreAndInstantiateContractProposal\x12\r\n\x05title\x18\x01 \x01(\t\x12\x13\n\x0bdescription\x18\x02 \x01(\t\x12\x0e\n\x06run_as\x18\x03 \x01(\t\x12(\n\x0ewasm_byte_code\x18\x04 \x01(\x0cB\x10\xe2\xde\x1f\x0cWASMByteCode\x12>\n\x16instantiate_permission\x18\x05 \x01(\x0b2\x1e.cosmwasm.wasm.v1.AccessConfig\x12\x12\n\nunpin_code\x18\x06 \x01(\x08\x12\r\n\x05admin\x18\x07 \x01(\t\x12\r\n\x05label\x18\x08 \x01(\t\x12#\n\x03msg\x18\t \x01(\x0cB\x16\xfa\xde\x1f\x12RawContractMessage\x12Z\n\x05funds\x18\n \x03(\x0b2\x19.cosmos.base.v1beta1.CoinB0\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins\x12\x0e\n\x06source\x18\x0b \x01(\t\x12\x0f\n\x07builder\x18\x0c \x01(\t\x12\x11\n\tcode_hash\x18\r \x01(\x0cB4Z&github.com/CosmWasm/wasmd/x/wasm/types\xd8\xe1\x1e\x00\xc8\xe1\x1e\x00\xa8\xe2\x1e\x01b\x06proto3')
_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, globals())
_builder.BuildTopDescriptorsAndMessages(DESCRIPTOR, 'cosmwasm.wasm.v1.proposal_pb2', globals())
if _descriptor._USE_C_DESCRIPTORS == False:
    DESCRIPTOR._options = None
    DESCRIPTOR._serialized_options = b'Z&github.com/CosmWasm/wasmd/x/wasm/types\xd8\xe1\x1e\x00\xc8\xe1\x1e\x00\xa8\xe2\x1e\x01'
    _STORECODEPROPOSAL.fields_by_name['wasm_byte_code']._options = None
    _STORECODEPROPOSAL.fields_by_name['wasm_byte_code']._serialized_options = b'\xe2\xde\x1f\x0cWASMByteCode'
    _INSTANTIATECONTRACTPROPOSAL.fields_by_name['code_id']._options = None
    _INSTANTIATECONTRACTPROPOSAL.fields_by_name['code_id']._serialized_options = b'\xe2\xde\x1f\x06CodeID'
    _INSTANTIATECONTRACTPROPOSAL.fields_by_name['msg']._options = None
    _INSTANTIATECONTRACTPROPOSAL.fields_by_name['msg']._serialized_options = b'\xfa\xde\x1f\x12RawContractMessage'
    _INSTANTIATECONTRACTPROPOSAL.fields_by_name['funds']._options = None
    _INSTANTIATECONTRACTPROPOSAL.fields_by_name['funds']._serialized_options = b'\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins'
    _MIGRATECONTRACTPROPOSAL.fields_by_name['code_id']._options = None
    _MIGRATECONTRACTPROPOSAL.fields_by_name['code_id']._serialized_options = b'\xe2\xde\x1f\x06CodeID'
    _MIGRATECONTRACTPROPOSAL.fields_by_name['msg']._options = None
    _MIGRATECONTRACTPROPOSAL.fields_by_name['msg']._serialized_options = b'\xfa\xde\x1f\x12RawContractMessage'
    _SUDOCONTRACTPROPOSAL.fields_by_name['msg']._options = None
    _SUDOCONTRACTPROPOSAL.fields_by_name['msg']._serialized_options = b'\xfa\xde\x1f\x12RawContractMessage'
    _EXECUTECONTRACTPROPOSAL.fields_by_name['msg']._options = None
    _EXECUTECONTRACTPROPOSAL.fields_by_name['msg']._serialized_options = b'\xfa\xde\x1f\x12RawContractMessage'
    _EXECUTECONTRACTPROPOSAL.fields_by_name['funds']._options = None
    _EXECUTECONTRACTPROPOSAL.fields_by_name['funds']._serialized_options = b'\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins'
    _UPDATEADMINPROPOSAL.fields_by_name['new_admin']._options = None
    _UPDATEADMINPROPOSAL.fields_by_name['new_admin']._serialized_options = b'\xf2\xde\x1f\x10yaml:"new_admin"'
    _PINCODESPROPOSAL.fields_by_name['title']._options = None
    _PINCODESPROPOSAL.fields_by_name['title']._serialized_options = b'\xf2\xde\x1f\x0cyaml:"title"'
    _PINCODESPROPOSAL.fields_by_name['description']._options = None
    _PINCODESPROPOSAL.fields_by_name['description']._serialized_options = b'\xf2\xde\x1f\x12yaml:"description"'
    _PINCODESPROPOSAL.fields_by_name['code_ids']._options = None
    _PINCODESPROPOSAL.fields_by_name['code_ids']._serialized_options = b'\xe2\xde\x1f\x07CodeIDs\xf2\xde\x1f\x0fyaml:"code_ids"'
    _UNPINCODESPROPOSAL.fields_by_name['title']._options = None
    _UNPINCODESPROPOSAL.fields_by_name['title']._serialized_options = b'\xf2\xde\x1f\x0cyaml:"title"'
    _UNPINCODESPROPOSAL.fields_by_name['description']._options = None
    _UNPINCODESPROPOSAL.fields_by_name['description']._serialized_options = b'\xf2\xde\x1f\x12yaml:"description"'
    _UNPINCODESPROPOSAL.fields_by_name['code_ids']._options = None
    _UNPINCODESPROPOSAL.fields_by_name['code_ids']._serialized_options = b'\xe2\xde\x1f\x07CodeIDs\xf2\xde\x1f\x0fyaml:"code_ids"'
    _ACCESSCONFIGUPDATE.fields_by_name['code_id']._options = None
    _ACCESSCONFIGUPDATE.fields_by_name['code_id']._serialized_options = b'\xe2\xde\x1f\x06CodeID'
    _ACCESSCONFIGUPDATE.fields_by_name['instantiate_permission']._options = None
    _ACCESSCONFIGUPDATE.fields_by_name['instantiate_permission']._serialized_options = b'\xc8\xde\x1f\x00'
    _UPDATEINSTANTIATECONFIGPROPOSAL.fields_by_name['title']._options = None
    _UPDATEINSTANTIATECONFIGPROPOSAL.fields_by_name['title']._serialized_options = b'\xf2\xde\x1f\x0cyaml:"title"'
    _UPDATEINSTANTIATECONFIGPROPOSAL.fields_by_name['description']._options = None
    _UPDATEINSTANTIATECONFIGPROPOSAL.fields_by_name['description']._serialized_options = b'\xf2\xde\x1f\x12yaml:"description"'
    _UPDATEINSTANTIATECONFIGPROPOSAL.fields_by_name['access_config_updates']._options = None
    _UPDATEINSTANTIATECONFIGPROPOSAL.fields_by_name['access_config_updates']._serialized_options = b'\xc8\xde\x1f\x00'
    _STOREANDINSTANTIATECONTRACTPROPOSAL.fields_by_name['wasm_byte_code']._options = None
    _STOREANDINSTANTIATECONTRACTPROPOSAL.fields_by_name['wasm_byte_code']._serialized_options = b'\xe2\xde\x1f\x0cWASMByteCode'
    _STOREANDINSTANTIATECONTRACTPROPOSAL.fields_by_name['msg']._options = None
    _STOREANDINSTANTIATECONTRACTPROPOSAL.fields_by_name['msg']._serialized_options = b'\xfa\xde\x1f\x12RawContractMessage'
    _STOREANDINSTANTIATECONTRACTPROPOSAL.fields_by_name['funds']._options = None
    _STOREANDINSTANTIATECONTRACTPROPOSAL.fields_by_name['funds']._serialized_options = b'\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins'
    _STORECODEPROPOSAL._serialized_start = 138
    _STORECODEPROPOSAL._serialized_end = 399
    _INSTANTIATECONTRACTPROPOSAL._serialized_start = 402
    _INSTANTIATECONTRACTPROPOSAL._serialized_end = 671
    _MIGRATECONTRACTPROPOSAL._serialized_start = 674
    _MIGRATECONTRACTPROPOSAL._serialized_end = 819
    _SUDOCONTRACTPROPOSAL._serialized_start = 821
    _SUDOCONTRACTPROPOSAL._serialized_end = 934
    _EXECUTECONTRACTPROPOSAL._serialized_start = 937
    _EXECUTECONTRACTPROPOSAL._serialized_end = 1161
    _UPDATEADMINPROPOSAL._serialized_start = 1163
    _UPDATEADMINPROPOSAL._serialized_end = 1279
    _CLEARADMINPROPOSAL._serialized_start = 1281
    _CLEARADMINPROPOSAL._serialized_end = 1355
    _PINCODESPROPOSAL._serialized_start = 1358
    _PINCODESPROPOSAL._serialized_end = 1504
    _UNPINCODESPROPOSAL._serialized_start = 1507
    _UNPINCODESPROPOSAL._serialized_end = 1655
    _ACCESSCONFIGUPDATE._serialized_start = 1657
    _ACCESSCONFIGUPDATE._serialized_end = 1776
    _UPDATEINSTANTIATECONFIGPROPOSAL._serialized_start = 1779
    _UPDATEINSTANTIATECONFIGPROPOSAL._serialized_end = 1965
    _STOREANDINSTANTIATECONTRACTPROPOSAL._serialized_start = 1968
    _STOREANDINSTANTIATECONTRACTPROPOSAL._serialized_end = 2394