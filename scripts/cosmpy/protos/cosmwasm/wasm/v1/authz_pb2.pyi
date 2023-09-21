from gogoproto import gogo_pb2 as _gogo_pb2
from cosmos_proto import cosmos_pb2 as _cosmos_pb2
from cosmos.base.v1beta1 import coin_pb2 as _coin_pb2
from google.protobuf import any_pb2 as _any_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union
DESCRIPTOR: _descriptor.FileDescriptor

class AcceptedMessageKeysFilter(_message.Message):
    __slots__ = ['keys']
    KEYS_FIELD_NUMBER: _ClassVar[int]
    keys: _containers.RepeatedScalarFieldContainer[str]

    def __init__(self, keys: _Optional[_Iterable[str]]=...) -> None:
        ...

class AcceptedMessagesFilter(_message.Message):
    __slots__ = ['messages']
    MESSAGES_FIELD_NUMBER: _ClassVar[int]
    messages: _containers.RepeatedScalarFieldContainer[bytes]

    def __init__(self, messages: _Optional[_Iterable[bytes]]=...) -> None:
        ...

class AllowAllMessagesFilter(_message.Message):
    __slots__ = []

    def __init__(self) -> None:
        ...

class CombinedLimit(_message.Message):
    __slots__ = ['amounts', 'calls_remaining']
    AMOUNTS_FIELD_NUMBER: _ClassVar[int]
    CALLS_REMAINING_FIELD_NUMBER: _ClassVar[int]
    amounts: _containers.RepeatedCompositeFieldContainer[_coin_pb2.Coin]
    calls_remaining: int

    def __init__(self, calls_remaining: _Optional[int]=..., amounts: _Optional[_Iterable[_Union[_coin_pb2.Coin, _Mapping]]]=...) -> None:
        ...

class ContractExecutionAuthorization(_message.Message):
    __slots__ = ['grants']
    GRANTS_FIELD_NUMBER: _ClassVar[int]
    grants: _containers.RepeatedCompositeFieldContainer[ContractGrant]

    def __init__(self, grants: _Optional[_Iterable[_Union[ContractGrant, _Mapping]]]=...) -> None:
        ...

class ContractGrant(_message.Message):
    __slots__ = ['contract', 'filter', 'limit']
    CONTRACT_FIELD_NUMBER: _ClassVar[int]
    FILTER_FIELD_NUMBER: _ClassVar[int]
    LIMIT_FIELD_NUMBER: _ClassVar[int]
    contract: str
    filter: _any_pb2.Any
    limit: _any_pb2.Any

    def __init__(self, contract: _Optional[str]=..., limit: _Optional[_Union[_any_pb2.Any, _Mapping]]=..., filter: _Optional[_Union[_any_pb2.Any, _Mapping]]=...) -> None:
        ...

class ContractMigrationAuthorization(_message.Message):
    __slots__ = ['grants']
    GRANTS_FIELD_NUMBER: _ClassVar[int]
    grants: _containers.RepeatedCompositeFieldContainer[ContractGrant]

    def __init__(self, grants: _Optional[_Iterable[_Union[ContractGrant, _Mapping]]]=...) -> None:
        ...

class MaxCallsLimit(_message.Message):
    __slots__ = ['remaining']
    REMAINING_FIELD_NUMBER: _ClassVar[int]
    remaining: int

    def __init__(self, remaining: _Optional[int]=...) -> None:
        ...

class MaxFundsLimit(_message.Message):
    __slots__ = ['amounts']
    AMOUNTS_FIELD_NUMBER: _ClassVar[int]
    amounts: _containers.RepeatedCompositeFieldContainer[_coin_pb2.Coin]

    def __init__(self, amounts: _Optional[_Iterable[_Union[_coin_pb2.Coin, _Mapping]]]=...) -> None:
        ...