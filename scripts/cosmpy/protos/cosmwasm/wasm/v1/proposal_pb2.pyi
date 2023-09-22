from gogoproto import gogo_pb2 as _gogo_pb2
from cosmos.base.v1beta1 import coin_pb2 as _coin_pb2
from cosmwasm.wasm.v1 import types_pb2 as _types_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union
DESCRIPTOR: _descriptor.FileDescriptor

class AccessConfigUpdate(_message.Message):
    __slots__ = ['code_id', 'instantiate_permission']
    CODE_ID_FIELD_NUMBER: _ClassVar[int]
    INSTANTIATE_PERMISSION_FIELD_NUMBER: _ClassVar[int]
    code_id: int
    instantiate_permission: _types_pb2.AccessConfig

    def __init__(self, code_id: _Optional[int]=..., instantiate_permission: _Optional[_Union[_types_pb2.AccessConfig, _Mapping]]=...) -> None:
        ...

class ClearAdminProposal(_message.Message):
    __slots__ = ['contract', 'description', 'title']
    CONTRACT_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    contract: str
    description: str
    title: str

    def __init__(self, title: _Optional[str]=..., description: _Optional[str]=..., contract: _Optional[str]=...) -> None:
        ...

class ExecuteContractProposal(_message.Message):
    __slots__ = ['contract', 'description', 'funds', 'msg', 'run_as', 'title']
    CONTRACT_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    FUNDS_FIELD_NUMBER: _ClassVar[int]
    MSG_FIELD_NUMBER: _ClassVar[int]
    RUN_AS_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    contract: str
    description: str
    funds: _containers.RepeatedCompositeFieldContainer[_coin_pb2.Coin]
    msg: bytes
    run_as: str
    title: str

    def __init__(self, title: _Optional[str]=..., description: _Optional[str]=..., run_as: _Optional[str]=..., contract: _Optional[str]=..., msg: _Optional[bytes]=..., funds: _Optional[_Iterable[_Union[_coin_pb2.Coin, _Mapping]]]=...) -> None:
        ...

class InstantiateContractProposal(_message.Message):
    __slots__ = ['admin', 'code_id', 'description', 'funds', 'label', 'msg', 'run_as', 'title']
    ADMIN_FIELD_NUMBER: _ClassVar[int]
    CODE_ID_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    FUNDS_FIELD_NUMBER: _ClassVar[int]
    LABEL_FIELD_NUMBER: _ClassVar[int]
    MSG_FIELD_NUMBER: _ClassVar[int]
    RUN_AS_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    admin: str
    code_id: int
    description: str
    funds: _containers.RepeatedCompositeFieldContainer[_coin_pb2.Coin]
    label: str
    msg: bytes
    run_as: str
    title: str

    def __init__(self, title: _Optional[str]=..., description: _Optional[str]=..., run_as: _Optional[str]=..., admin: _Optional[str]=..., code_id: _Optional[int]=..., label: _Optional[str]=..., msg: _Optional[bytes]=..., funds: _Optional[_Iterable[_Union[_coin_pb2.Coin, _Mapping]]]=...) -> None:
        ...

class MigrateContractProposal(_message.Message):
    __slots__ = ['code_id', 'contract', 'description', 'msg', 'title']
    CODE_ID_FIELD_NUMBER: _ClassVar[int]
    CONTRACT_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    MSG_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    code_id: int
    contract: str
    description: str
    msg: bytes
    title: str

    def __init__(self, title: _Optional[str]=..., description: _Optional[str]=..., contract: _Optional[str]=..., code_id: _Optional[int]=..., msg: _Optional[bytes]=...) -> None:
        ...

class PinCodesProposal(_message.Message):
    __slots__ = ['code_ids', 'description', 'title']
    CODE_IDS_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    code_ids: _containers.RepeatedScalarFieldContainer[int]
    description: str
    title: str

    def __init__(self, title: _Optional[str]=..., description: _Optional[str]=..., code_ids: _Optional[_Iterable[int]]=...) -> None:
        ...

class StoreAndInstantiateContractProposal(_message.Message):
    __slots__ = ['admin', 'builder', 'code_hash', 'description', 'funds', 'instantiate_permission', 'label', 'msg', 'run_as', 'source', 'title', 'unpin_code', 'wasm_byte_code']
    ADMIN_FIELD_NUMBER: _ClassVar[int]
    BUILDER_FIELD_NUMBER: _ClassVar[int]
    CODE_HASH_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    FUNDS_FIELD_NUMBER: _ClassVar[int]
    INSTANTIATE_PERMISSION_FIELD_NUMBER: _ClassVar[int]
    LABEL_FIELD_NUMBER: _ClassVar[int]
    MSG_FIELD_NUMBER: _ClassVar[int]
    RUN_AS_FIELD_NUMBER: _ClassVar[int]
    SOURCE_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    UNPIN_CODE_FIELD_NUMBER: _ClassVar[int]
    WASM_BYTE_CODE_FIELD_NUMBER: _ClassVar[int]
    admin: str
    builder: str
    code_hash: bytes
    description: str
    funds: _containers.RepeatedCompositeFieldContainer[_coin_pb2.Coin]
    instantiate_permission: _types_pb2.AccessConfig
    label: str
    msg: bytes
    run_as: str
    source: str
    title: str
    unpin_code: bool
    wasm_byte_code: bytes

    def __init__(self, title: _Optional[str]=..., description: _Optional[str]=..., run_as: _Optional[str]=..., wasm_byte_code: _Optional[bytes]=..., instantiate_permission: _Optional[_Union[_types_pb2.AccessConfig, _Mapping]]=..., unpin_code: bool=..., admin: _Optional[str]=..., label: _Optional[str]=..., msg: _Optional[bytes]=..., funds: _Optional[_Iterable[_Union[_coin_pb2.Coin, _Mapping]]]=..., source: _Optional[str]=..., builder: _Optional[str]=..., code_hash: _Optional[bytes]=...) -> None:
        ...

class StoreCodeProposal(_message.Message):
    __slots__ = ['builder', 'code_hash', 'description', 'instantiate_permission', 'run_as', 'source', 'title', 'unpin_code', 'wasm_byte_code']
    BUILDER_FIELD_NUMBER: _ClassVar[int]
    CODE_HASH_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    INSTANTIATE_PERMISSION_FIELD_NUMBER: _ClassVar[int]
    RUN_AS_FIELD_NUMBER: _ClassVar[int]
    SOURCE_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    UNPIN_CODE_FIELD_NUMBER: _ClassVar[int]
    WASM_BYTE_CODE_FIELD_NUMBER: _ClassVar[int]
    builder: str
    code_hash: bytes
    description: str
    instantiate_permission: _types_pb2.AccessConfig
    run_as: str
    source: str
    title: str
    unpin_code: bool
    wasm_byte_code: bytes

    def __init__(self, title: _Optional[str]=..., description: _Optional[str]=..., run_as: _Optional[str]=..., wasm_byte_code: _Optional[bytes]=..., instantiate_permission: _Optional[_Union[_types_pb2.AccessConfig, _Mapping]]=..., unpin_code: bool=..., source: _Optional[str]=..., builder: _Optional[str]=..., code_hash: _Optional[bytes]=...) -> None:
        ...

class SudoContractProposal(_message.Message):
    __slots__ = ['contract', 'description', 'msg', 'title']
    CONTRACT_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    MSG_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    contract: str
    description: str
    msg: bytes
    title: str

    def __init__(self, title: _Optional[str]=..., description: _Optional[str]=..., contract: _Optional[str]=..., msg: _Optional[bytes]=...) -> None:
        ...

class UnpinCodesProposal(_message.Message):
    __slots__ = ['code_ids', 'description', 'title']
    CODE_IDS_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    code_ids: _containers.RepeatedScalarFieldContainer[int]
    description: str
    title: str

    def __init__(self, title: _Optional[str]=..., description: _Optional[str]=..., code_ids: _Optional[_Iterable[int]]=...) -> None:
        ...

class UpdateAdminProposal(_message.Message):
    __slots__ = ['contract', 'description', 'new_admin', 'title']
    CONTRACT_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    NEW_ADMIN_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    contract: str
    description: str
    new_admin: str
    title: str

    def __init__(self, title: _Optional[str]=..., description: _Optional[str]=..., new_admin: _Optional[str]=..., contract: _Optional[str]=...) -> None:
        ...

class UpdateInstantiateConfigProposal(_message.Message):
    __slots__ = ['access_config_updates', 'description', 'title']
    ACCESS_CONFIG_UPDATES_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    access_config_updates: _containers.RepeatedCompositeFieldContainer[AccessConfigUpdate]
    description: str
    title: str

    def __init__(self, title: _Optional[str]=..., description: _Optional[str]=..., access_config_updates: _Optional[_Iterable[_Union[AccessConfigUpdate, _Mapping]]]=...) -> None:
        ...