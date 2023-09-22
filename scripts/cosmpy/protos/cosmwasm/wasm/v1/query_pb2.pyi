from gogoproto import gogo_pb2 as _gogo_pb2
from cosmwasm.wasm.v1 import types_pb2 as _types_pb2
from google.api import annotations_pb2 as _annotations_pb2
from cosmos.base.query.v1beta1 import pagination_pb2 as _pagination_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union
DESCRIPTOR: _descriptor.FileDescriptor

class CodeInfoResponse(_message.Message):
    __slots__ = ['code_id', 'creator', 'data_hash', 'instantiate_permission']
    CODE_ID_FIELD_NUMBER: _ClassVar[int]
    CREATOR_FIELD_NUMBER: _ClassVar[int]
    DATA_HASH_FIELD_NUMBER: _ClassVar[int]
    INSTANTIATE_PERMISSION_FIELD_NUMBER: _ClassVar[int]
    code_id: int
    creator: str
    data_hash: bytes
    instantiate_permission: _types_pb2.AccessConfig

    def __init__(self, code_id: _Optional[int]=..., creator: _Optional[str]=..., data_hash: _Optional[bytes]=..., instantiate_permission: _Optional[_Union[_types_pb2.AccessConfig, _Mapping]]=...) -> None:
        ...

class QueryAllContractStateRequest(_message.Message):
    __slots__ = ['address', 'pagination']
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    address: str
    pagination: _pagination_pb2.PageRequest

    def __init__(self, address: _Optional[str]=..., pagination: _Optional[_Union[_pagination_pb2.PageRequest, _Mapping]]=...) -> None:
        ...

class QueryAllContractStateResponse(_message.Message):
    __slots__ = ['models', 'pagination']
    MODELS_FIELD_NUMBER: _ClassVar[int]
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    models: _containers.RepeatedCompositeFieldContainer[_types_pb2.Model]
    pagination: _pagination_pb2.PageResponse

    def __init__(self, models: _Optional[_Iterable[_Union[_types_pb2.Model, _Mapping]]]=..., pagination: _Optional[_Union[_pagination_pb2.PageResponse, _Mapping]]=...) -> None:
        ...

class QueryCodeRequest(_message.Message):
    __slots__ = ['code_id']
    CODE_ID_FIELD_NUMBER: _ClassVar[int]
    code_id: int

    def __init__(self, code_id: _Optional[int]=...) -> None:
        ...

class QueryCodeResponse(_message.Message):
    __slots__ = ['code_info', 'data']
    CODE_INFO_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    code_info: CodeInfoResponse
    data: bytes

    def __init__(self, code_info: _Optional[_Union[CodeInfoResponse, _Mapping]]=..., data: _Optional[bytes]=...) -> None:
        ...

class QueryCodesRequest(_message.Message):
    __slots__ = ['pagination']
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    pagination: _pagination_pb2.PageRequest

    def __init__(self, pagination: _Optional[_Union[_pagination_pb2.PageRequest, _Mapping]]=...) -> None:
        ...

class QueryCodesResponse(_message.Message):
    __slots__ = ['code_infos', 'pagination']
    CODE_INFOS_FIELD_NUMBER: _ClassVar[int]
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    code_infos: _containers.RepeatedCompositeFieldContainer[CodeInfoResponse]
    pagination: _pagination_pb2.PageResponse

    def __init__(self, code_infos: _Optional[_Iterable[_Union[CodeInfoResponse, _Mapping]]]=..., pagination: _Optional[_Union[_pagination_pb2.PageResponse, _Mapping]]=...) -> None:
        ...

class QueryContractHistoryRequest(_message.Message):
    __slots__ = ['address', 'pagination']
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    address: str
    pagination: _pagination_pb2.PageRequest

    def __init__(self, address: _Optional[str]=..., pagination: _Optional[_Union[_pagination_pb2.PageRequest, _Mapping]]=...) -> None:
        ...

class QueryContractHistoryResponse(_message.Message):
    __slots__ = ['entries', 'pagination']
    ENTRIES_FIELD_NUMBER: _ClassVar[int]
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    entries: _containers.RepeatedCompositeFieldContainer[_types_pb2.ContractCodeHistoryEntry]
    pagination: _pagination_pb2.PageResponse

    def __init__(self, entries: _Optional[_Iterable[_Union[_types_pb2.ContractCodeHistoryEntry, _Mapping]]]=..., pagination: _Optional[_Union[_pagination_pb2.PageResponse, _Mapping]]=...) -> None:
        ...

class QueryContractInfoRequest(_message.Message):
    __slots__ = ['address']
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    address: str

    def __init__(self, address: _Optional[str]=...) -> None:
        ...

class QueryContractInfoResponse(_message.Message):
    __slots__ = ['address', 'contract_info']
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    CONTRACT_INFO_FIELD_NUMBER: _ClassVar[int]
    address: str
    contract_info: _types_pb2.ContractInfo

    def __init__(self, address: _Optional[str]=..., contract_info: _Optional[_Union[_types_pb2.ContractInfo, _Mapping]]=...) -> None:
        ...

class QueryContractsByCodeRequest(_message.Message):
    __slots__ = ['code_id', 'pagination']
    CODE_ID_FIELD_NUMBER: _ClassVar[int]
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    code_id: int
    pagination: _pagination_pb2.PageRequest

    def __init__(self, code_id: _Optional[int]=..., pagination: _Optional[_Union[_pagination_pb2.PageRequest, _Mapping]]=...) -> None:
        ...

class QueryContractsByCodeResponse(_message.Message):
    __slots__ = ['contracts', 'pagination']
    CONTRACTS_FIELD_NUMBER: _ClassVar[int]
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    contracts: _containers.RepeatedScalarFieldContainer[str]
    pagination: _pagination_pb2.PageResponse

    def __init__(self, contracts: _Optional[_Iterable[str]]=..., pagination: _Optional[_Union[_pagination_pb2.PageResponse, _Mapping]]=...) -> None:
        ...

class QueryContractsByCreatorRequest(_message.Message):
    __slots__ = ['creator_address', 'pagination']
    CREATOR_ADDRESS_FIELD_NUMBER: _ClassVar[int]
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    creator_address: str
    pagination: _pagination_pb2.PageRequest

    def __init__(self, creator_address: _Optional[str]=..., pagination: _Optional[_Union[_pagination_pb2.PageRequest, _Mapping]]=...) -> None:
        ...

class QueryContractsByCreatorResponse(_message.Message):
    __slots__ = ['contract_addresses', 'pagination']
    CONTRACT_ADDRESSES_FIELD_NUMBER: _ClassVar[int]
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    contract_addresses: _containers.RepeatedScalarFieldContainer[str]
    pagination: _pagination_pb2.PageResponse

    def __init__(self, contract_addresses: _Optional[_Iterable[str]]=..., pagination: _Optional[_Union[_pagination_pb2.PageResponse, _Mapping]]=...) -> None:
        ...

class QueryParamsRequest(_message.Message):
    __slots__ = []

    def __init__(self) -> None:
        ...

class QueryParamsResponse(_message.Message):
    __slots__ = ['params']
    PARAMS_FIELD_NUMBER: _ClassVar[int]
    params: _types_pb2.Params

    def __init__(self, params: _Optional[_Union[_types_pb2.Params, _Mapping]]=...) -> None:
        ...

class QueryPinnedCodesRequest(_message.Message):
    __slots__ = ['pagination']
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    pagination: _pagination_pb2.PageRequest

    def __init__(self, pagination: _Optional[_Union[_pagination_pb2.PageRequest, _Mapping]]=...) -> None:
        ...

class QueryPinnedCodesResponse(_message.Message):
    __slots__ = ['code_ids', 'pagination']
    CODE_IDS_FIELD_NUMBER: _ClassVar[int]
    PAGINATION_FIELD_NUMBER: _ClassVar[int]
    code_ids: _containers.RepeatedScalarFieldContainer[int]
    pagination: _pagination_pb2.PageResponse

    def __init__(self, code_ids: _Optional[_Iterable[int]]=..., pagination: _Optional[_Union[_pagination_pb2.PageResponse, _Mapping]]=...) -> None:
        ...

class QueryRawContractStateRequest(_message.Message):
    __slots__ = ['address', 'query_data']
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    QUERY_DATA_FIELD_NUMBER: _ClassVar[int]
    address: str
    query_data: bytes

    def __init__(self, address: _Optional[str]=..., query_data: _Optional[bytes]=...) -> None:
        ...

class QueryRawContractStateResponse(_message.Message):
    __slots__ = ['data']
    DATA_FIELD_NUMBER: _ClassVar[int]
    data: bytes

    def __init__(self, data: _Optional[bytes]=...) -> None:
        ...

class QuerySmartContractStateRequest(_message.Message):
    __slots__ = ['address', 'query_data']
    ADDRESS_FIELD_NUMBER: _ClassVar[int]
    QUERY_DATA_FIELD_NUMBER: _ClassVar[int]
    address: str
    query_data: bytes

    def __init__(self, address: _Optional[str]=..., query_data: _Optional[bytes]=...) -> None:
        ...

class QuerySmartContractStateResponse(_message.Message):
    __slots__ = ['data']
    DATA_FIELD_NUMBER: _ClassVar[int]
    data: bytes

    def __init__(self, data: _Optional[bytes]=...) -> None:
        ...