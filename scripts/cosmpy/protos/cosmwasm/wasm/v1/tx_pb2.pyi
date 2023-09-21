from cosmos.base.v1beta1 import coin_pb2 as _coin_pb2
from gogoproto import gogo_pb2 as _gogo_pb2
from cosmwasm.wasm.v1 import types_pb2 as _types_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union
DESCRIPTOR: _descriptor.FileDescriptor

class MsgClearAdmin(_message.Message):
    __slots__ = ['contract', 'sender']
    CONTRACT_FIELD_NUMBER: _ClassVar[int]
    SENDER_FIELD_NUMBER: _ClassVar[int]
    contract: str
    sender: str

    def __init__(self, sender: _Optional[str]=..., contract: _Optional[str]=...) -> None:
        ...

class MsgClearAdminResponse(_message.Message):
    __slots__ = []

    def __init__(self) -> None:
        ...

class MsgExecuteContract(_message.Message):
    __slots__ = ['contract', 'funds', 'msg', 'sender']
    CONTRACT_FIELD_NUMBER: _ClassVar[int]
    FUNDS_FIELD_NUMBER: _ClassVar[int]
    MSG_FIELD_NUMBER: _ClassVar[int]
    SENDER_FIELD_NUMBER: _ClassVar[int]
    contract: str
    funds: _containers.RepeatedCompositeFieldContainer[_coin_pb2.Coin]
    msg: bytes
    sender: str

    def __init__(self, sender: _Optional[str]=..., contract: _Optional[str]=..., msg: _Optional[bytes]=..., funds: _Optional[_Iterable[_Union[_coin_pb2.Coin, _Mapping]]]=...) -> None:
        ...

class MsgExecuteContractResponse(_message.Message):
    __slots__ = ['data']
    DATA_FIELD_NUMBER: _ClassVar[int]
    data: bytes

    def __init__(self, data: _Optional[bytes]=...) -> None:
        ...

class MsgInstantiateContract(_message.Message):
    __slots__ = ['admin', 'code_id', 'funds', 'label', 'msg', 'sender']
    ADMIN_FIELD_NUMBER: _ClassVar[int]
    CODE_ID_FIELD_NUMBER: _ClassVar[int]
    FUNDS_FIELD_NUMBER: _ClassVar[int]
    LABEL_FIELD_NUMBER: _ClassVar[int]
    MSG_FIELD_NUMBER: _ClassVar[int]
    SENDER_FIELD_NUMBER: _ClassVar[int]
    admin: str
    code_id: int
    funds: _containers.RepeatedCompositeFieldContainer[_coin_pb2.Coin]
    label: str
    msg: bytes
    sender: str

    def __init__(self, sender: _Optional[str]=..., admin: _Optional[str]=..., code_id: _Optional[int]=..., label: _Optional[str]=..., msg: _Optional[bytes]=..., funds: _Optional[_Iterable[_Union[_coin_pb2.Coin, _Mapping]]]=...) -> None:
        ...

class MsgInstantiateContract2(_message.Message):
    __slots__ = ['admin', 'code_id', 'fix_msg', 'funds', 'label', 'msg', 'salt', 'sender']
    ADMIN_FIELD_NUMBER: _ClassVar[int]
    CODE_ID_FIELD_NUMBER: _ClassVar[int]
    FIX_MSG_FIELD_NUMBER: _ClassVar[int]
    FUNDS_FIELD_NUMBER: _ClassVar[int]
    LABEL_FIELD_NUMBER: _ClassVar[int]
    MSG_FIELD_NUMBER: _ClassVar[int]
    SALT_FIELD_NUMBER: _ClassVar[int]
    SENDER_FIELD_NUMBER: _ClassVar[int]
    admin: str
    code_id: int
    fix_msg: bool
    funds: _containers.RepeatedCompositeFieldContainer[_coin_pb2.Coin]
    label: str
    msg: bytes
    salt: bytes
    sender: str

    def __init__(self, sender: _Optional[str]=..., admin: _Optional[str]=..., code_id: _Optional[int]=..., label: _Optional[str]=..., msg: _Optional[bytes]=..., funds: _Optional[_Iterable[_Union[_coin_pb2.Coin, _Mapping]]]=..., salt: _Optional[bytes]=..., fix_msg: bool=...) -> None:
        ...

class MsgInstantiateContract2Response(_message.Message):
    __slots__ = ['address', 'data']
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    address: str
    data: bytes

    def __init__(self, address: _Optional[str]=..., data: _Optional[bytes]=...) -> None:
        ...

class MsgInstantiateContractResponse(_message.Message):
    __slots__ = ['address', 'data']
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    address: str
    data: bytes

    def __init__(self, address: _Optional[str]=..., data: _Optional[bytes]=...) -> None:
        ...

class MsgMigrateContract(_message.Message):
    __slots__ = ['code_id', 'contract', 'msg', 'sender']
    CODE_ID_FIELD_NUMBER: _ClassVar[int]
    CONTRACT_FIELD_NUMBER: _ClassVar[int]
    MSG_FIELD_NUMBER: _ClassVar[int]
    SENDER_FIELD_NUMBER: _ClassVar[int]
    code_id: int
    contract: str
    msg: bytes
    sender: str

    def __init__(self, sender: _Optional[str]=..., contract: _Optional[str]=..., code_id: _Optional[int]=..., msg: _Optional[bytes]=...) -> None:
        ...

class MsgMigrateContractResponse(_message.Message):
    __slots__ = ['data']
    DATA_FIELD_NUMBER: _ClassVar[int]
    data: bytes

    def __init__(self, data: _Optional[bytes]=...) -> None:
        ...

class MsgStoreCode(_message.Message):
    __slots__ = ['instantiate_permission', 'sender', 'wasm_byte_code']
    INSTANTIATE_PERMISSION_FIELD_NUMBER: _ClassVar[int]
    SENDER_FIELD_NUMBER: _ClassVar[int]
    WASM_BYTE_CODE_FIELD_NUMBER: _ClassVar[int]
    instantiate_permission: _types_pb2.AccessConfig
    sender: str
    wasm_byte_code: bytes

    def __init__(self, sender: _Optional[str]=..., wasm_byte_code: _Optional[bytes]=..., instantiate_permission: _Optional[_Union[_types_pb2.AccessConfig, _Mapping]]=...) -> None:
        ...

class MsgStoreCodeResponse(_message.Message):
    __slots__ = ['checksum', 'code_id']
    CHECKSUM_FIELD_NUMBER: _ClassVar[int]
    CODE_ID_FIELD_NUMBER: _ClassVar[int]
    checksum: bytes
    code_id: int

    def __init__(self, code_id: _Optional[int]=..., checksum: _Optional[bytes]=...) -> None:
        ...

class MsgUpdateAdmin(_message.Message):
    __slots__ = ['contract', 'new_admin', 'sender']
    CONTRACT_FIELD_NUMBER: _ClassVar[int]
    NEW_ADMIN_FIELD_NUMBER: _ClassVar[int]
    SENDER_FIELD_NUMBER: _ClassVar[int]
    contract: str
    new_admin: str
    sender: str

    def __init__(self, sender: _Optional[str]=..., new_admin: _Optional[str]=..., contract: _Optional[str]=...) -> None:
        ...

class MsgUpdateAdminResponse(_message.Message):
    __slots__ = []

    def __init__(self) -> None:
        ...