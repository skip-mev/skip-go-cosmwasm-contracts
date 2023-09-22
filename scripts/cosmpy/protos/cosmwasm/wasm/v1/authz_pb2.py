"""Generated protocol buffer code."""
from google.protobuf.internal import builder as _builder
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
_sym_db = _symbol_database.Default()
from ....gogoproto import gogo_pb2 as gogoproto_dot_gogo__pb2
from ....cosmos_proto import cosmos_pb2 as cosmos__proto_dot_cosmos__pb2
from ....cosmos.base.v1beta1 import coin_pb2 as cosmos_dot_base_dot_v1beta1_dot_coin__pb2
from google.protobuf import any_pb2 as google_dot_protobuf_dot_any__pb2
DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(b'\n\x1ccosmwasm/wasm/v1/authz.proto\x12\x10cosmwasm.wasm.v1\x1a\x14gogoproto/gogo.proto\x1a\x19cosmos_proto/cosmos.proto\x1a\x1ecosmos/base/v1beta1/coin.proto\x1a\x19google/protobuf/any.proto"j\n\x1eContractExecutionAuthorization\x125\n\x06grants\x18\x01 \x03(\x0b2\x1f.cosmwasm.wasm.v1.ContractGrantB\x04\xc8\xde\x1f\x00:\x11\xca\xb4-\rAuthorization"j\n\x1eContractMigrationAuthorization\x125\n\x06grants\x18\x01 \x03(\x0b2\x1f.cosmwasm.wasm.v1.ContractGrantB\x04\xc8\xde\x1f\x00:\x11\xca\xb4-\rAuthorization"\x9f\x01\n\rContractGrant\x12\x10\n\x08contract\x18\x01 \x01(\t\x12<\n\x05limit\x18\x02 \x01(\x0b2\x14.google.protobuf.AnyB\x17\xca\xb4-\x13ContractAuthzLimitX\x12>\n\x06filter\x18\x03 \x01(\x0b2\x14.google.protobuf.AnyB\x18\xca\xb4-\x14ContractAuthzFilterX";\n\rMaxCallsLimit\x12\x11\n\tremaining\x18\x01 \x01(\x04:\x17\xca\xb4-\x13ContractAuthzLimitX"\x86\x01\n\rMaxFundsLimit\x12\\\n\x07amounts\x18\x01 \x03(\x0b2\x19.cosmos.base.v1beta1.CoinB0\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins:\x17\xca\xb4-\x13ContractAuthzLimitX"\x9f\x01\n\rCombinedLimit\x12\x17\n\x0fcalls_remaining\x18\x01 \x01(\x04\x12\\\n\x07amounts\x18\x02 \x03(\x0b2\x19.cosmos.base.v1beta1.CoinB0\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins:\x17\xca\xb4-\x13ContractAuthzLimitX"2\n\x16AllowAllMessagesFilter:\x18\xca\xb4-\x14ContractAuthzFilterX"C\n\x19AcceptedMessageKeysFilter\x12\x0c\n\x04keys\x18\x01 \x03(\t:\x18\xca\xb4-\x14ContractAuthzFilterX"\\\n\x16AcceptedMessagesFilter\x12(\n\x08messages\x18\x01 \x03(\x0cB\x16\xfa\xde\x1f\x12RawContractMessage:\x18\xca\xb4-\x14ContractAuthzFilterXB,Z&github.com/CosmWasm/wasmd/x/wasm/types\xc8\xe1\x1e\x00b\x06proto3')
_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, globals())
_builder.BuildTopDescriptorsAndMessages(DESCRIPTOR, 'cosmwasm.wasm.v1.authz_pb2', globals())
if _descriptor._USE_C_DESCRIPTORS == False:
    DESCRIPTOR._options = None
    DESCRIPTOR._serialized_options = b'Z&github.com/CosmWasm/wasmd/x/wasm/types\xc8\xe1\x1e\x00'
    _CONTRACTEXECUTIONAUTHORIZATION.fields_by_name['grants']._options = None
    _CONTRACTEXECUTIONAUTHORIZATION.fields_by_name['grants']._serialized_options = b'\xc8\xde\x1f\x00'
    _CONTRACTEXECUTIONAUTHORIZATION._options = None
    _CONTRACTEXECUTIONAUTHORIZATION._serialized_options = b'\xca\xb4-\rAuthorization'
    _CONTRACTMIGRATIONAUTHORIZATION.fields_by_name['grants']._options = None
    _CONTRACTMIGRATIONAUTHORIZATION.fields_by_name['grants']._serialized_options = b'\xc8\xde\x1f\x00'
    _CONTRACTMIGRATIONAUTHORIZATION._options = None
    _CONTRACTMIGRATIONAUTHORIZATION._serialized_options = b'\xca\xb4-\rAuthorization'
    _CONTRACTGRANT.fields_by_name['limit']._options = None
    _CONTRACTGRANT.fields_by_name['limit']._serialized_options = b'\xca\xb4-\x13ContractAuthzLimitX'
    _CONTRACTGRANT.fields_by_name['filter']._options = None
    _CONTRACTGRANT.fields_by_name['filter']._serialized_options = b'\xca\xb4-\x14ContractAuthzFilterX'
    _MAXCALLSLIMIT._options = None
    _MAXCALLSLIMIT._serialized_options = b'\xca\xb4-\x13ContractAuthzLimitX'
    _MAXFUNDSLIMIT.fields_by_name['amounts']._options = None
    _MAXFUNDSLIMIT.fields_by_name['amounts']._serialized_options = b'\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins'
    _MAXFUNDSLIMIT._options = None
    _MAXFUNDSLIMIT._serialized_options = b'\xca\xb4-\x13ContractAuthzLimitX'
    _COMBINEDLIMIT.fields_by_name['amounts']._options = None
    _COMBINEDLIMIT.fields_by_name['amounts']._serialized_options = b'\xc8\xde\x1f\x00\xaa\xdf\x1f(github.com/cosmos/cosmos-sdk/types.Coins'
    _COMBINEDLIMIT._options = None
    _COMBINEDLIMIT._serialized_options = b'\xca\xb4-\x13ContractAuthzLimitX'
    _ALLOWALLMESSAGESFILTER._options = None
    _ALLOWALLMESSAGESFILTER._serialized_options = b'\xca\xb4-\x14ContractAuthzFilterX'
    _ACCEPTEDMESSAGEKEYSFILTER._options = None
    _ACCEPTEDMESSAGEKEYSFILTER._serialized_options = b'\xca\xb4-\x14ContractAuthzFilterX'
    _ACCEPTEDMESSAGESFILTER.fields_by_name['messages']._options = None
    _ACCEPTEDMESSAGESFILTER.fields_by_name['messages']._serialized_options = b'\xfa\xde\x1f\x12RawContractMessage'
    _ACCEPTEDMESSAGESFILTER._options = None
    _ACCEPTEDMESSAGESFILTER._serialized_options = b'\xca\xb4-\x14ContractAuthzFilterX'
    _CONTRACTEXECUTIONAUTHORIZATION._serialized_start = 158
    _CONTRACTEXECUTIONAUTHORIZATION._serialized_end = 264
    _CONTRACTMIGRATIONAUTHORIZATION._serialized_start = 266
    _CONTRACTMIGRATIONAUTHORIZATION._serialized_end = 372
    _CONTRACTGRANT._serialized_start = 375
    _CONTRACTGRANT._serialized_end = 534
    _MAXCALLSLIMIT._serialized_start = 536
    _MAXCALLSLIMIT._serialized_end = 595
    _MAXFUNDSLIMIT._serialized_start = 598
    _MAXFUNDSLIMIT._serialized_end = 732
    _COMBINEDLIMIT._serialized_start = 735
    _COMBINEDLIMIT._serialized_end = 894
    _ALLOWALLMESSAGESFILTER._serialized_start = 896
    _ALLOWALLMESSAGESFILTER._serialized_end = 946
    _ACCEPTEDMESSAGEKEYSFILTER._serialized_start = 948
    _ACCEPTEDMESSAGEKEYSFILTER._serialized_end = 1015
    _ACCEPTEDMESSAGESFILTER._serialized_start = 1017
    _ACCEPTEDMESSAGESFILTER._serialized_end = 1109