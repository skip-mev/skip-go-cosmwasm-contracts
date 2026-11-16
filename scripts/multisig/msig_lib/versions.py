"""cosmos-sdk / x/tx / wasmd 버전 추출과 비교. amino-json sign bytes는 이 세 모듈이 결정한다."""

TRACKED = {
    "github.com/cosmos/cosmos-sdk": "cosmos_sdk",
    "cosmossdk.io/x/tx": "x_tx",
    "github.com/CosmWasm/wasmd": "wasmd",
}
# 체인이 반드시 보고해야 하는 모듈 (없으면 비교 불가로 실패)
REQUIRED_ON_CHAIN = ("cosmos_sdk",)


def parse_build_deps(deps: list) -> dict:
    """build_deps 파싱. 로컬 CLI("path@ver" / "path@ver => repl@ver2")와
    LCD node_info({path, version})를 같은 기준으로 맞추기 위해 replace 이전 버전을 쓴다.
    (SDK node_info의 Module.Version은 replace 이전 값: cmtservice/service.go)"""
    out: dict = {}
    for dep in deps:
        if isinstance(dep, dict):
            path, version = dep.get("path", ""), dep.get("version", "")
        else:
            orig = dep.partition(" => ")[0].strip()
            path, _, version = orig.partition("@")
        key = TRACKED.get(path)
        if key:
            out[key] = version
    return out


parse_cli_build_deps = parse_build_deps
parse_node_build_deps = parse_build_deps


def minor_line(version: str) -> str:
    parts = version.lstrip("v").split(".")
    return "v" + ".".join(parts[:2]) if len(parts) >= 2 else version


def diff_exact(local: dict, chain: dict) -> list[str]:
    diffs = [f"chain reports no {k} version; cannot compare" for k in REQUIRED_ON_CHAIN if k not in chain]
    diffs += [
        f"{k}: local {local.get(k, '?')} != chain {chain[k]}"
        for k in TRACKED.values()
        if k in chain and local.get(k) != chain[k]
    ]
    return diffs


def diff_minor(local: dict, chain: dict) -> list[str]:
    return [
        f"{k}: local {local.get(k, '?')} vs chain {chain.get(k, '?')}"
        for k in ("cosmos_sdk", "wasmd")
        if k in chain and minor_line(local.get(k, "")) != minor_line(chain[k])
    ]
