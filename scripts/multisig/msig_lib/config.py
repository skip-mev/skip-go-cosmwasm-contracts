import math
import re
import tomllib
from dataclasses import dataclass, field
from decimal import Decimal
from pathlib import Path

from .errors import UsageError

CHAINS_DIR = Path(__file__).resolve().parent.parent / "chains"
# chain 이름은 코디네이터가 준 meta.json에서도 오므로 경로 탈출을 막는다
CHAIN_NAME_RE = re.compile(r"^[a-z0-9][a-z0-9_-]*$")


@dataclass
class ChainConfig:
    name: str
    chain_id: str
    bin: str
    bech32_prefix: str
    fee_denom: str
    rpc: str
    lcd: str
    gas_price_query: str | None = None
    gas_price_multiplier: Decimal = Decimal("1")
    gas_price_fixed: Decimal = Decimal("0")
    gas: dict = field(default_factory=dict)

    def default_gas(self, kind: str) -> int:
        return int(self.gas.get(kind, self.gas.get("default", 500000)))

    def fee_amount(self, gas: int, gas_price: Decimal) -> int:
        return math.ceil(Decimal(gas) * gas_price)


def load_chain(name: str, chains_dir: Path = CHAINS_DIR) -> ChainConfig:
    if not isinstance(name, str) or not CHAIN_NAME_RE.match(name):
        raise UsageError(f"invalid chain name {name!r} (allowed: lowercase letters, digits, '-', '_')")
    path = chains_dir / f"{name}.toml"
    if not path.exists():
        available = sorted(p.stem for p in chains_dir.glob("*.toml"))
        raise UsageError(f"unknown chain '{name}' (available: {', '.join(available) or 'none'})")
    with path.open("rb") as f:
        raw = tomllib.load(f)

    required = ["chain_id", "bin", "bech32_prefix", "fee_denom", "rpc", "lcd"]
    missing = [k for k in required if k not in raw]
    if missing:
        raise UsageError(f"{path}: missing keys {missing}")

    gp = raw.get("gas_price", {})
    if "query" not in gp and "fixed" not in gp:
        raise UsageError(f"{path}: [gas_price] needs 'query' or 'fixed'")

    return ChainConfig(
        name=name,
        chain_id=raw["chain_id"],
        bin=raw["bin"],
        bech32_prefix=raw["bech32_prefix"],
        fee_denom=raw["fee_denom"],
        rpc=raw["rpc"],
        lcd=raw["lcd"].rstrip("/"),
        gas_price_query=gp.get("query"),
        gas_price_multiplier=Decimal(str(gp.get("multiplier", "1"))),
        gas_price_fixed=Decimal(str(gp.get("fixed", "0"))),
        gas=raw.get("gas", {}),
    )
