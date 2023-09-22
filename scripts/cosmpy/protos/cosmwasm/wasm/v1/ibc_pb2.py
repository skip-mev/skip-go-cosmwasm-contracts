"""Generated protocol buffer code."""
from google.protobuf.internal import builder as _builder
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
_sym_db = _symbol_database.Default()
from ....gogoproto import gogo_pb2 as gogoproto_dot_gogo__pb2
DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(b'\n\x1acosmwasm/wasm/v1/ibc.proto\x12\x10cosmwasm.wasm.v1\x1a\x14gogoproto/gogo.proto"\xb2\x01\n\nMsgIBCSend\x12*\n\x07channel\x18\x02 \x01(\tB\x19\xf2\xde\x1f\x15yaml:"source_channel"\x121\n\x0etimeout_height\x18\x04 \x01(\x04B\x19\xf2\xde\x1f\x15yaml:"timeout_height"\x127\n\x11timeout_timestamp\x18\x05 \x01(\x04B\x1c\xf2\xde\x1f\x18yaml:"timeout_timestamp"\x12\x0c\n\x04data\x18\x06 \x01(\x0c"@\n\x12MsgIBCCloseChannel\x12*\n\x07channel\x18\x02 \x01(\tB\x19\xf2\xde\x1f\x15yaml:"source_channel"B,Z&github.com/CosmWasm/wasmd/x/wasm/types\xc8\xe1\x1e\x00b\x06proto3')
_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, globals())
_builder.BuildTopDescriptorsAndMessages(DESCRIPTOR, 'cosmwasm.wasm.v1.ibc_pb2', globals())
if _descriptor._USE_C_DESCRIPTORS == False:
    DESCRIPTOR._options = None
    DESCRIPTOR._serialized_options = b'Z&github.com/CosmWasm/wasmd/x/wasm/types\xc8\xe1\x1e\x00'
    _MSGIBCSEND.fields_by_name['channel']._options = None
    _MSGIBCSEND.fields_by_name['channel']._serialized_options = b'\xf2\xde\x1f\x15yaml:"source_channel"'
    _MSGIBCSEND.fields_by_name['timeout_height']._options = None
    _MSGIBCSEND.fields_by_name['timeout_height']._serialized_options = b'\xf2\xde\x1f\x15yaml:"timeout_height"'
    _MSGIBCSEND.fields_by_name['timeout_timestamp']._options = None
    _MSGIBCSEND.fields_by_name['timeout_timestamp']._serialized_options = b'\xf2\xde\x1f\x18yaml:"timeout_timestamp"'
    _MSGIBCCLOSECHANNEL.fields_by_name['channel']._options = None
    _MSGIBCCLOSECHANNEL.fields_by_name['channel']._serialized_options = b'\xf2\xde\x1f\x15yaml:"source_channel"'
    _MSGIBCSEND._serialized_start = 71
    _MSGIBCSEND._serialized_end = 249
    _MSGIBCCLOSECHANNEL._serialized_start = 251
    _MSGIBCCLOSECHANNEL._serialized_end = 315