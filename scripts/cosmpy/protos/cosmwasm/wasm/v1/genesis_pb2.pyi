from gogoproto import gogo_pb2 as _gogo_pb2
from cosmwasm.wasm.v1 import types_pb2 as _types_pb2
from cosmwasm.wasm.v1 import tx_pb2 as _tx_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union
DESCRIPTOR: _descriptor.FileDescriptor

class Code(_message.Message):
    __slots__ = ['code_bytes', 'code_id', 'code_info', 'pinned']
    CODE_BYTES_FIELD_NUMBER: _ClassVar[int]
    CODE_ID_FIELD_NUMBER: _ClassVar[int]
    CODE_INFO_FIELD_NUMBER: _ClassVar[int]
    PINNED_FIELD_NUMBER: _ClassVar[int]
    code_bytes: bytes
    code_id: int
    code_info: _types_pb2.CodeInfo
    pinned: bool

    def __init__(self, code_id: _Optional[int]=..., code_info: _Optional[_Union[_types_pb2.CodeInfo, _Mapping]]=..., code_bytes: _Optional[bytes]=..., pinned: bool=...) -> None:
        ...

class Contract(_message.Message):
    __slots__ = ['contract_address', 'contract_code_history', 'contract_info', 'contract_state']
    CONTRACT_ADDRESS_FIELD_NUMBER: _ClassVar[int]
    CONTRACT_CODE_HISTORY_FIELD_NUMBER: _ClassVar[int]
    CONTRACT_INFO_FIELD_NUMBER: _ClassVar[int]
    CONTRACT_STATE_FIELD_NUMBER: _ClassVar[int]
    contract_address: str
    contract_code_history: _containers.RepeatedCompositeFieldContainer[_types_pb2.ContractCodeHistoryEntry]
    contract_info: _types_pb2.ContractInfo
    contract_state: _containers.RepeatedCompositeFieldContainer[_types_pb2.Model]

    def __init__(self, contract_address: _Optional[str]=..., contract_info: _Optional[_Union[_types_pb2.ContractInfo, _Mapping]]=..., contract_state: _Optional[_Iterable[_Union[_types_pb2.Model, _Mapping]]]=..., contract_code_history: _Optional[_Iterable[_Union[_types_pb2.ContractCodeHistoryEntry, _Mapping]]]=...) -> None:
        ...

class GenesisState(_message.Message):
    __slots__ = ['codes', 'contracts', 'gen_msgs', 'params', 'sequences']

    class GenMsgs(_message.Message):
        __slots__ = ['execute_contract', 'instantiate_contract', 'store_code']
        EXECUTE_CONTRACT_FIELD_NUMBER: _ClassVar[int]
        INSTANTIATE_CONTRACT_FIELD_NUMBER: _ClassVar[int]
        STORE_CODE_FIELD_NUMBER: _ClassVar[int]
        execute_contract: _tx_pb2.MsgExecuteContract
        instantiate_contract: _tx_pb2.MsgInstantiateContract
        store_code: _tx_pb2.MsgStoreCode

        def __init__(self, store_code: _Optional[_Union[_tx_pb2.MsgStoreCode, _Mapping]]=..., instantiate_contract: _Optional[_Union[_tx_pb2.MsgInstantiateContract, _Mapping]]=..., execute_contract: _Optional[_Union[_tx_pb2.MsgExecuteContract, _Mapping]]=...) -> None:
            ...
    CODES_FIELD_NUMBER: _ClassVar[int]
    CONTRACTS_FIELD_NUMBER: _ClassVar[int]
    GEN_MSGS_FIELD_NUMBER: _ClassVar[int]
    PARAMS_FIELD_NUMBER: _ClassVar[int]
    SEQUENCES_FIELD_NUMBER: _ClassVar[int]
    codes: _containers.RepeatedCompositeFieldContainer[Code]
    contracts: _containers.RepeatedCompositeFieldContainer[Contract]
    gen_msgs: _containers.RepeatedCompositeFieldContainer[GenesisState.GenMsgs]
    params: _types_pb2.Params
    sequences: _containers.RepeatedCompositeFieldContainer[Sequence]

    def __init__(self, params: _Optional[_Union[_types_pb2.Params, _Mapping]]=..., codes: _Optional[_Iterable[_Union[Code, _Mapping]]]=..., contracts: _Optional[_Iterable[_Union[Contract, _Mapping]]]=..., sequences: _Optional[_Iterable[_Union[Sequence, _Mapping]]]=..., gen_msgs: _Optional[_Iterable[_Union[GenesisState.GenMsgs, _Mapping]]]=...) -> None:
        ...

class Sequence(_message.Message):
    __slots__ = ['id_key', 'value']
    ID_KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    id_key: bytes
    value: int

    def __init__(self, id_key: _Optional[bytes]=..., value: _Optional[int]=...) -> None:
        ...