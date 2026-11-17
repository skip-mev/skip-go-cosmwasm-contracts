import base64
import copy
import gzip
import hashlib
import json
from pathlib import Path

from .errors import UsageError

STORE = "/cosmwasm.wasm.v1.MsgStoreCode"
INSTANTIATE = "/cosmwasm.wasm.v1.MsgInstantiateContract"
INSTANTIATE2 = "/cosmwasm.wasm.v1.MsgInstantiateContract2"
EXECUTE = "/cosmwasm.wasm.v1.MsgExecuteContract"
MIGRATE = "/cosmwasm.wasm.v1.MsgMigrateContract"
UPDATE_ADMIN = "/cosmwasm.wasm.v1.MsgUpdateAdmin"
CLEAR_ADMIN = "/cosmwasm.wasm.v1.MsgClearAdmin"

TX_SUFFIX = ".tx.json"


def base_name(tx_path: Path) -> str:
    s = str(tx_path)
    if not s.endswith(TX_SUFFIX):
        raise UsageError(f"tx file must end with '{TX_SUFFIX}': {tx_path}")
    return s[: -len(".json")]


def meta_path(tx_path: Path) -> Path:
    return Path(base_name(tx_path) + ".meta.json")


def sig_path(tx_path: Path, signer: str) -> Path:
    return Path(base_name(tx_path) + f".sig.{signer}.json")


def signed_path(tx_path: Path) -> Path:
    return Path(base_name(tx_path) + ".signed.json")


def read_json(path: Path) -> dict:
    try:
        return json.loads(Path(path).read_text())
    except FileNotFoundError as e:
        raise UsageError(f"file not found: {path}") from e
    except json.JSONDecodeError as e:
        raise UsageError(f"invalid JSON in {path}: {e}") from e


def write_json(path: Path, data: dict) -> None:
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(json.dumps(data, indent=2) + "\n")


def messages(tx: dict) -> list[dict]:
    return tx.get("body", {}).get("messages", [])


WASM_MAGIC = b"\x00asm"


def decode_wasm(b64: str) -> bytes:
    try:
        raw = base64.b64decode(b64, validate=True)
    except Exception as e:
        raise UsageError("wasm_byte_code is not valid base64") from e
    # The CLI gzips the wasm; the checksum is over the raw wasm
    if raw[:2] == b"\x1f\x8b":
        raw = gzip.decompress(raw)
    if raw[:4] != WASM_MAGIC:
        raise UsageError("wasm_byte_code is not a wasm binary (missing \\0asm magic)")
    return raw


def wasm_sha256(msg: dict) -> str:
    return hashlib.sha256(decode_wasm(msg["wasm_byte_code"])).hexdigest()


def decode_contract_msg(value):
    """msg field of instantiate/execute/migrate: accepts a JSON object or base64(JSON)."""
    if isinstance(value, (dict, list)):
        return value
    if isinstance(value, str):
        try:
            return json.loads(base64.b64decode(value, validate=True))
        except Exception:
            try:
                return json.loads(value)
            except Exception:
                return value
    return value


def decode_salt(value: str) -> bytes:
    # bytes fields in proto JSON are always base64 (no hex fallback; it would be ambiguous)
    try:
        return base64.b64decode(value, validate=True)
    except Exception as e:
        raise UsageError(f"salt is not valid base64: {value!r}") from e


# Field naming the signer in each msg (sender for wasm, from_address for bank, etc.)
SIGNER_FIELDS = ("sender", "from_address", "delegator_address", "signer", "authority")


def signer_of(msg: dict) -> str | None:
    for f in SIGNER_FIELDS:
        if f in msg:
            return msg[f]
    return None


def sig_pubkeys(sig_file: dict) -> list[str]:
    return [s.get("public_key", {}).get("key", "") for s in sig_file.get("signatures", [])]


def store_checksums(tx: dict) -> list[str]:
    return [wasm_sha256(m) for m in messages(tx) if m.get("@type") == STORE]


def tx_fee(tx: dict) -> tuple[int, list[dict]]:
    fee = tx.get("auth_info", {}).get("fee", {})
    return int(fee.get("gas_limit", "0")), fee.get("amount", [])


def merge(txs: list[dict]) -> dict:
    if len(txs) < 2:
        raise UsageError("merge needs at least 2 tx files")
    base = copy.deepcopy(txs[0])
    for i, tx in enumerate(txs[1:], start=2):
        if tx.get("body", {}).get("memo", "") != base["body"].get("memo", ""):
            raise UsageError(f"tx #{i}: memo differs from tx #1")
        if str(tx.get("body", {}).get("timeout_height", "0")) != str(base["body"].get("timeout_height", "0")):
            raise UsageError(f"tx #{i}: timeout_height differs from tx #1")
        if tx.get("auth_info", {}).get("signer_infos") or tx.get("signatures"):
            raise UsageError(f"tx #{i}: already signed; merge only unsigned txs")
        base["body"]["messages"].extend(copy.deepcopy(messages(tx)))

    gas_total = sum(tx_fee(tx)[0] for tx in txs)
    amounts: dict[str, int] = {}
    for tx in txs:
        for coin in tx_fee(tx)[1]:
            amounts[coin["denom"]] = amounts.get(coin["denom"], 0) + int(coin["amount"])
    base["auth_info"]["fee"]["gas_limit"] = str(gas_total)
    base["auth_info"]["fee"]["amount"] = [{"denom": d, "amount": str(a)} for d, a in sorted(amounts.items())]
    return base


def merge_meta(metas: list[dict]) -> dict:
    keys = ("chain", "chain_id", "multisig", "account_number", "sequence")
    first = metas[0]
    for i, m in enumerate(metas[1:], start=2):
        for k in keys:
            if m.get(k) != first.get(k):
                raise UsageError(f"meta #{i}: {k} '{m.get(k)}' != '{first.get(k)}'")
    return dict(first)


def body_and_fee(tx: dict) -> tuple:
    auth = tx.get("auth_info", {})
    return tx.get("body"), auth.get("fee")
