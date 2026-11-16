import json
import urllib.error
import urllib.request
from decimal import Decimal

from .errors import ChainError
from .versions import parse_node_build_deps

TIMEOUT = 30


def get_json(base: str, path: str) -> dict:
    req = urllib.request.Request(base + path, headers={"User-Agent": "msig-tool"})
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            return json.load(resp)
    except urllib.error.HTTPError as e:
        body = e.read().decode(errors="replace")[:300]
        raise ChainError(f"GET {base}{path} -> HTTP {e.code}: {body}") from e
    except urllib.error.URLError as e:
        raise ChainError(f"GET {base}{path} failed: {e.reason}") from e


def _find_account_fields(obj):
    # vesting/module 계정 등은 base_account 아래에 중첩된다
    if isinstance(obj, dict):
        if "account_number" in obj and "sequence" in obj:
            return obj["account_number"], obj["sequence"]
        for v in obj.values():
            found = _find_account_fields(v)
            if found:
                return found
    return None


def account(lcd: str, address: str) -> tuple[str, str]:
    data = get_json(lcd, f"/cosmos/auth/v1beta1/accounts/{address}")
    found = _find_account_fields(data.get("account", {}))
    if not found:
        raise ChainError(f"account {address}: account_number/sequence not found (unfunded account?)")
    return str(found[0]), str(found[1])


def node_versions(lcd: str) -> dict:
    data = get_json(lcd, "/cosmos/base/tendermint/v1beta1/node_info")
    deps = data.get("application_version", {}).get("build_deps", [])
    return parse_node_build_deps(deps)


def gas_price(lcd: str, query_path: str) -> Decimal:
    data = get_json(lcd, query_path)
    price = data.get("price", {}).get("amount")
    if price is None:
        raise ChainError(f"gas price response has no price.amount: {data}")
    return Decimal(price)


def code_checksum(lcd: str, code_id: str) -> str:
    # code-info는 wasm 본문 없이 checksum만 준다; 구버전 wasmd는 /code/{id}로 대체
    try:
        return str(get_json(lcd, f"/cosmwasm/wasm/v1/code-info/{code_id}")["checksum"]).lower()
    except (ChainError, KeyError):
        data = get_json(lcd, f"/cosmwasm/wasm/v1/code/{code_id}")
        return str(data.get("code_info", {}).get("data_hash", "")).lower()
