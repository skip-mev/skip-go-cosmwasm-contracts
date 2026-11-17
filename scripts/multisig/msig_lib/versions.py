"""Extract and compare cosmos-sdk / x/tx / wasmd versions. These three modules determine amino-json sign bytes."""

TRACKED = {
    "github.com/cosmos/cosmos-sdk": "cosmos_sdk",
    "cosmossdk.io/x/tx": "x_tx",
    "github.com/CosmWasm/wasmd": "wasmd",
}
# Modules the chain must report (missing → cannot compare, fail)
REQUIRED_ON_CHAIN = ("cosmos_sdk",)


def parse_build_deps(deps: list) -> dict:
    """Parse build_deps. Uses the pre-replace version so the local CLI ("path@ver" / "path@ver => repl@ver2")
    and LCD node_info ({path, version}) are compared on the same basis.
    (SDK node_info Module.Version is the pre-replace value: cmtservice/service.go)"""
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
