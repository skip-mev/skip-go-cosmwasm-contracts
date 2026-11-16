#!/usr/bin/env python3
"""msig: Cosmos SDK multisig tx tool (create / merge / show / sign / combine / broadcast / address).

서명·인코딩은 체인 CLI(gaiad 등)에 위임하고, 이 도구는 tx 생성 보조와
'서명 전에 사람이 내용을 확인하고 기대 checksum과 대조'하는 일만 한다.
"""
import argparse
import binascii
import gzip
import hashlib
import json
import os
import re
import sys
import tempfile
import time
import zlib
from decimal import Decimal, InvalidOperation
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from msig_lib import lcd  # noqa: E402
from msig_lib import txfile as tf  # noqa: E402
from msig_lib.cli import ChainCli  # noqa: E402
from msig_lib.config import ChainConfig, load_chain  # noqa: E402
from msig_lib.errors import ChainError, CheckFailed, CliError, MsigError, UsageError  # noqa: E402
from msig_lib.render import print_review, review  # noqa: E402
from msig_lib.versions import diff_exact, diff_minor  # noqa: E402

DEFAULT_WORK_DIR = Path(__file__).resolve().parent / "work"


def info(msg: str = "") -> None:
    print(msg, file=sys.stderr)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def slug(text: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", text.lower()).strip("_")[:48] or "tx"


def make_cli(cfg: ChainConfig, args) -> ChainCli:
    return ChainCli(cfg.bin, args.keyring_backend, args.home)


def resolve_multisig(value: str | None, cfg: ChainConfig, cli: ChainCli) -> str:
    value = value or os.environ.get("MSIG_MULTISIG")
    if not value:
        raise UsageError("multisig not given: use --multisig <address|key name> or MSIG_MULTISIG")
    if value.startswith(cfg.bech32_prefix + "1"):
        return value
    return cli.key_address(value)


def check_versions_exact(cfg: ChainConfig, cli: ChainCli, skip: bool) -> None:
    if skip:
        info("WARNING   version check skipped (--skip-version-check)")
        return
    diffs = diff_exact(cli.versions(), lcd.node_versions(cfg.lcd))
    if diffs:
        raise CheckFailed(
            f"{cli.bin} build differs from {cfg.chain_id} node; sign bytes may not match:\n  "
            + "\n  ".join(diffs)
            + "\nInstall the chain's exact version (or --skip-version-check)."
        )


def resolve_gas_price(cfg: ChainConfig, override: str | None) -> Decimal:
    if override:
        return Decimal(override)
    if cfg.gas_price_query:
        try:
            return max(lcd.gas_price(cfg.lcd, cfg.gas_price_query) * cfg.gas_price_multiplier, cfg.gas_price_fixed)
        except ChainError as e:
            if not cfg.gas_price_fixed:
                raise
            info(f"WARNING   gas price query failed, using fixed {cfg.gas_price_fixed}: {e}")
    return cfg.gas_price_fixed


META_KEYS = ("chain", "chain_id", "multisig", "account_number", "sequence")


def load_tx_and_meta(tx_path: Path, meta_override: Path | None = None) -> tuple[dict, dict]:
    tx = tf.read_json(tx_path)
    mpath = meta_override or tf.meta_path(tx_path)
    if not mpath.exists():
        raise UsageError(f"meta file not found: {mpath} (pass --meta)")
    meta = tf.read_json(mpath)
    missing = [k for k in META_KEYS if k not in meta]
    if missing:
        raise UsageError(f"{mpath}: missing keys {missing}")
    return tx, meta


def prepare_out(path: Path, force: bool) -> Path:
    # 네트워크 호출·파일 쓰기 전에 이름과 덮어쓰기 여부를 먼저 확인
    tf.meta_path(path)
    if path.exists() and not force:
        raise UsageError(f"{path} exists (use --force or -o)")
    return path


# ---------------------------------------------------------------- create

def cmd_create(args) -> None:
    cfg = load_chain(args.chain)

    if args.kind == "store":
        wasm = Path(args.wasm)
        if not wasm.exists():
            raise UsageError(f"wasm not found: {wasm}")
        gas_kind, name = "store", f"store_{wasm.stem}"
    elif args.kind == "instantiate2":
        try:
            init_msg = json.dumps(json.loads(args.msg), separators=(",", ":"))
        except json.JSONDecodeError as e:
            raise UsageError(f"--msg is not valid JSON: {e}") from e
        salt_hex = args.salt_hex or args.salt.encode().hex()
        gas_kind, name = "instantiate", f"instantiate2_{slug(args.label)}"
    else:
        # 구분자 `--` 하나만 제거 (그 뒤의 `--`는 CLI 인자일 수 있음)
        tx_args = args.tx_args[1:] if args.tx_args[:1] == ["--"] else list(args.tx_args)
        if not tx_args:
            raise UsageError("pass-through needs tx arguments, e.g.: create cosmoshub tx -- wasm migrate <contract> <code_id> '{}'")
        gas_kind, name = "default", "tx_" + slug("_".join(tx_args[:2]))
    out = prepare_out(Path(args.out) if args.out else Path(args.work_dir) / f"{name}{tf.TX_SUFFIX}", args.force)

    cli = make_cli(cfg, args)
    multisig = resolve_multisig(args.multisig, cfg, cli)
    check_versions_exact(cfg, cli, args.skip_version_check)

    if args.kind == "store":
        perm = ["--instantiate-everybody"] if args.instantiate_everybody else ["--instantiate-anyof-addresses", multisig]
        tx_args = ["wasm", "store", str(wasm), *perm]
    elif args.kind == "instantiate2":
        admin = ["--no-admin"] if args.no_admin else ["--admin", multisig if args.admin == "multisig" else args.admin]
        # salt 인자는 wasmd CLI 기본 디코더(hex)로 전달
        tx_args = ["wasm", "instantiate2", str(args.code_id), init_msg, salt_hex, "--label", args.label, *admin]

    gas = args.gas or cfg.default_gas(gas_kind)
    gas_price = resolve_gas_price(cfg, args.gas_price)
    fee = cfg.fee_amount(gas, gas_price)

    if args.account_number and args.sequence:
        acc_num, seq = args.account_number, args.sequence
    else:
        acc_num, chain_seq = lcd.account(cfg.lcd, multisig)
        acc_num = args.account_number or acc_num
        seq = args.sequence or chain_seq

    tx = cli.generate([
        *tx_args, "--from", multisig, "--generate-only", "--chain-id", cfg.chain_id,
        "--node", cfg.rpc, "--gas", str(gas), "--fees", f"{fee}{cfg.fee_denom}",
    ])

    meta = {"chain": cfg.name, "chain_id": cfg.chain_id, "multisig": multisig,
            "account_number": str(acc_num), "sequence": str(seq)}
    tf.write_json(out, tx)
    tf.write_json(tf.meta_path(out), meta)

    print_review(review(tx, meta, gas_price=gas_price, fee_denom=cfg.fee_denom, expect_multisig=multisig), sys.stdout)
    info("")
    info(f"tx    {out}  sha256 {sha256_file(out)}")
    info(f"meta  {tf.meta_path(out)}")
    if tf.store_checksums(tx):
        info("Share the wasm checksum(s) with signers through a channel separate from these files.")


# ---------------------------------------------------------------- merge

def cmd_merge(args) -> None:
    out = prepare_out(Path(args.out), args.force)
    paths = [Path(p) for p in args.txs]
    txs = [tf.read_json(p) for p in paths]
    metas = [tf.read_json(tf.meta_path(p)) for p in paths]
    merged, meta = tf.merge(txs), tf.merge_meta(metas)
    tf.write_json(out, merged)
    tf.write_json(tf.meta_path(out), meta)
    print_review(review(merged, meta, expect_multisig=meta.get("multisig")), sys.stdout)
    info("")
    info(f"tx    {out}  sha256 {sha256_file(out)}  ({len(tf.messages(merged))} msgs)")


# ---------------------------------------------------------------- show / sign

def online_context(tx: dict, cfg: ChainConfig, args) -> dict:
    ctx = {"code_checksums": {}, "addresses": {}}
    try:
        cli = make_cli(cfg, args)
    except UsageError:
        cli = None
    for i, m in enumerate(tf.messages(tx), start=1):
        if m.get("@type") not in (tf.INSTANTIATE, tf.INSTANTIATE2):
            continue
        code_id = str(m.get("code_id"))
        if code_id not in ctx["code_checksums"]:
            ctx["code_checksums"][code_id] = lcd.code_checksum(cfg.lcd, code_id)
        if m.get("@type") == tf.INSTANTIATE2 and not m.get("fix_msg") and cli:
            salt_hex = tf.decode_salt(m.get("salt", "")).hex()
            ctx["addresses"][i] = cli.build_address(ctx["code_checksums"][code_id], m["sender"], salt_hex)
    return ctx


def expected_multisig(args, cfg: ChainConfig) -> str | None:
    value = args.expect_multisig or os.environ.get("MSIG_MULTISIG")
    if not value:
        return None
    if value.startswith(cfg.bech32_prefix + "1"):
        return value
    return make_cli(cfg, args).key_address(value)


def build_review(args, tx: dict, meta: dict):
    cfg = load_chain(meta["chain"])
    if cfg.chain_id != meta["chain_id"]:
        raise CheckFailed(f"meta chain_id {meta['chain_id']} != chains/{cfg.name}.toml {cfg.chain_id}")
    online = online_context(tx, cfg, args) if args.online else None
    return review(tx, meta, expect_checksums=args.expect_checksum, online=online,
                  gas_price=cfg.gas_price_fixed or None, fee_denom=cfg.fee_denom,
                  expect_multisig=expected_multisig(args, cfg))


def cmd_show(args) -> None:
    tx, meta = load_tx_and_meta(Path(args.tx), Path(args.meta) if args.meta else None)
    r = build_review(args, tx, meta)
    print_review(r, sys.stdout)
    if r.errors:
        raise CheckFailed(f"{len(r.errors)} check(s) failed")


def cmd_sign(args) -> None:
    tx_path = Path(args.tx)
    tx, meta = load_tx_and_meta(tx_path, Path(args.meta) if args.meta else None)
    r = build_review(args, tx, meta)
    cfg = load_chain(meta["chain"])
    cli = make_cli(cfg, args)
    print_review(r, sys.stdout)
    if r.errors:
        raise CheckFailed(f"refusing to sign: {len(r.errors)} check(s) failed")

    if args.online:
        for d in diff_minor(cli.versions(), lcd.node_versions(cfg.lcd)):
            info(f"WARNING   version line differs ({d}); combine will reject the signature if sign bytes differ")
    else:
        info("NOTE      offline: CLI version not compared with chain (combine verifies signatures)")

    out = tf.sig_path(tx_path, args.signer or args.key)
    if out.exists() and not args.force:
        raise UsageError(f"{out} exists (use --force)")

    if not args.yes:
        if not sys.stdin.isatty():
            raise UsageError("no TTY for confirmation; review the output and pass --yes")
        try:
            answer = input("\nSign this transaction? [y/N] ").strip().lower()
        except EOFError:
            answer = ""
        if answer not in ("y", "yes"):
            raise CheckFailed("signing declined")

    cli.sign(tx_path, meta["multisig"], args.key, meta["chain_id"], meta["account_number"],
             meta["sequence"], out, ledger=args.ledger)
    info(f"signature  {out}  sha256 {sha256_file(out)}")


# ---------------------------------------------------------------- combine

def cli_error_line(proc) -> str:
    # cobra는 에러 뒤에 usage 전체를 출력하므로 "Error:" 줄만 뽑는다
    lines = [l.strip() for l in (proc.stderr + "\n" + proc.stdout).splitlines() if l.strip()]
    for line in lines:
        if line.startswith("Error:"):
            return line
    return lines[0] if lines else f"exit {proc.returncode}"


def cmd_combine(args) -> None:
    tx_path = Path(args.tx)
    tx, meta = load_tx_and_meta(tx_path, Path(args.meta) if args.meta else None)
    cfg = load_chain(meta["chain"])
    cli = make_cli(cfg, args)
    check_versions_exact(cfg, cli, args.skip_version_check)

    key = args.multisig_key or os.environ.get("MSIG_MULTISIG_KEY")
    if not key:
        raise UsageError("--multisig-key (or MSIG_MULTISIG_KEY) is required")
    key_addr = cli.key_address(key)
    if key_addr != meta["multisig"]:
        raise CheckFailed(f"key '{key}' is {key_addr}, but tx multisig is {meta['multisig']}")

    out = tf.signed_path(tx_path)
    if out.exists() and not args.force:
        raise UsageError(f"{out} exists (use --force)")

    valid, seen = [], set()
    with tempfile.TemporaryDirectory() as tmp:
        for s in (Path(p) for p in args.sigs):
            pubkeys = tf.sig_pubkeys(tf.read_json(s))
            # 같은 서명자의 파일이 여러 개면 SDK가 같은 슬롯을 덮어쓰므로 1개로 센다
            if pubkeys and all(pk in seen for pk in pubkeys):
                info(f"DUP   {s.name}  — same signer as an earlier file, ignored")
                continue
            proc = cli.multisign(tx_path, key, [s], meta["chain_id"], meta["account_number"],
                                 meta["sequence"], Path(tmp) / "probe.json")
            if proc.returncode == 0:
                info(f"OK    {s.name}")
                valid.append(s)
                seen.update(pubkeys)
            else:
                info(f"FAIL  {s.name}  — {cli_error_line(proc)}")

    threshold = cli.multisig_threshold(key)
    if len(seen) < threshold:
        raise CheckFailed(f"{len(seen)} distinct valid signer(s), threshold is {threshold}")

    proc = cli.multisign(tx_path, key, valid, meta["chain_id"], meta["account_number"], meta["sequence"], out)
    if proc.returncode != 0:
        raise CliError(f"multisign failed:\n{cli_error_line(proc)}")

    signed = tf.read_json(out)
    if tf.body_and_fee(signed) != tf.body_and_fee(tx):
        out.unlink()
        raise CheckFailed("signed tx body/fee differs from the unsigned tx")
    info(f"signed  {out}  ({len(seen)} signer(s), threshold {threshold})")


# ---------------------------------------------------------------- broadcast

def event_values(tx_resp: dict, event_type: str, key: str) -> list[str]:
    values = []
    for ev in tx_resp.get("events", []):
        if ev.get("type") == event_type:
            values += [a.get("value") for a in ev.get("attributes", []) if a.get("key") == key]
    return values


def cmd_broadcast(args) -> None:
    signed = Path(args.signed)
    if not str(signed).endswith(".signed.json"):
        raise UsageError("expected a *.signed.json file")
    meta_file = Path(args.meta) if args.meta else Path(str(signed)[: -len(".signed.json")] + ".meta.json")
    meta = tf.read_json(meta_file)
    cfg = load_chain(meta["chain"])
    cli = make_cli(cfg, args)

    res = cli.broadcast(signed, cfg.rpc)
    txhash = res.get("txhash")
    if res.get("code", 0) != 0:
        raise ChainError(f"broadcast rejected (code {res.get('code')}): {res.get('raw_log')}")
    info(f"txhash  {txhash}  waiting for inclusion...")

    deadline = time.time() + args.timeout
    tx_resp = None
    while time.time() < deadline:
        tx_resp = cli.query_tx(txhash, cfg.rpc)
        if tx_resp:
            break
        time.sleep(3)
    if not tx_resp:
        raise ChainError(f"tx {txhash} not found within {args.timeout}s")
    if int(tx_resp.get("code", 0)) != 0:
        raise ChainError(f"tx failed (code {tx_resp.get('code')}): {tx_resp.get('raw_log')}")

    result = {
        "txhash": txhash,
        "height": tx_resp.get("height"),
        "code_ids": event_values(tx_resp, "store_code", "code_id"),
        "code_checksums": event_values(tx_resp, "store_code", "code_checksum"),
        "contracts": event_values(tx_resp, "instantiate", "_contract_address"),
    }
    print(json.dumps(result, indent=2))
    tf.write_json(Path(str(signed)[: -len(".signed.json")] + ".result.json"), result)

    missing = [a for a in (args.expect_address or []) if a not in result["contracts"]]
    if missing:
        raise CheckFailed(f"expected contract address(es) not created: {missing}")
    # 저장된 코드가 서명한 tx의 wasm과 같은지 확인
    unsigned_path = Path(str(signed)[: -len(".signed.json")] + ".json")
    if unsigned_path.exists():
        signed_sums = sorted(tf.store_checksums(tf.read_json(unsigned_path)))
        chain_sums = sorted(c.lower() for c in result["code_checksums"])
        if signed_sums and signed_sums != chain_sums:
            raise CheckFailed(f"on-chain code checksum {chain_sums} != signed wasm {signed_sums}")


# ---------------------------------------------------------------- address

def cmd_address(args) -> None:
    cfg = load_chain(args.chain)
    cli = make_cli(cfg, args)
    creator = resolve_multisig(args.creator, cfg, cli)
    salt_hex = args.salt_hex or args.salt.encode().hex()
    print(cli.build_address(args.checksum.lower().removeprefix("0x"), creator, salt_hex))


# ---------------------------------------------------------------- main

class Parser(argparse.ArgumentParser):
    # argparse 기본 exit 2는 '검증 실패'(2)와 겹치므로 사용법 오류는 1로 통일
    def error(self, message):
        self.print_usage(sys.stderr)
        self.exit(1, f"{self.prog}: error: {message}\n")


def build_parser() -> argparse.ArgumentParser:
    p = Parser(prog="msig", description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--keyring-backend", help="CLI keyring backend (env MSIG_KEYRING_BACKEND)")
    p.add_argument("--home", help="CLI home dir (env MSIG_HOME)")
    p.add_argument("--work-dir", default=os.environ.get("MSIG_WORK_DIR") or str(DEFAULT_WORK_DIR),
                   help="where create writes txs (env MSIG_WORK_DIR; shared runs: runs/<date>-<chain>-<purpose>)")
    sub = p.add_subparsers(dest="cmd", required=True)

    # create
    c = sub.add_parser("create", help="(coordinator) build an unsigned tx + meta")
    c.add_argument("chain")
    ck = c.add_subparsers(dest="kind", required=True)

    def create_common(sp):
        sp.add_argument("--multisig", help="multisig address or keyring key name (env MSIG_MULTISIG)")
        sp.add_argument("--gas", type=int)
        sp.add_argument("--gas-price", help="override gas price (decimal)")
        sp.add_argument("--account-number")
        sp.add_argument("--sequence", help="override sequence (pre-build consecutive txs)")
        sp.add_argument("-o", "--out")
        sp.add_argument("--force", action="store_true")
        sp.add_argument("--skip-version-check", action="store_true")

    cs = ck.add_parser("store", help="MsgStoreCode")
    cs.add_argument("wasm")
    cs.add_argument("--instantiate-everybody", action="store_true",
                    help="default: only the multisig may instantiate this code")
    create_common(cs)

    ci = ck.add_parser("instantiate2", help="MsgInstantiateContract2")
    ci.add_argument("--code-id", required=True, type=int)
    ci.add_argument("--msg", required=True, help="instantiate msg JSON")
    ci.add_argument("--label", required=True)
    sg = ci.add_mutually_exclusive_group(required=True)
    sg.add_argument("--salt", help="salt as ascii")
    sg.add_argument("--salt-hex")
    ag = ci.add_mutually_exclusive_group(required=True)
    ag.add_argument("--admin", help="'multisig' or an address")
    ag.add_argument("--no-admin", action="store_true")
    create_common(ci)

    ct = ck.add_parser("tx", help="pass-through: arguments after -- go to `<bin> tx`")
    ct.add_argument("tx_args", nargs=argparse.REMAINDER)
    create_common(ct)

    m = sub.add_parser("merge", help="(coordinator) merge messages of several unsigned txs into one")
    m.add_argument("txs", nargs="+")
    m.add_argument("-o", "--out", required=True)
    m.add_argument("--force", action="store_true")

    def review_common(sp):
        sp.add_argument("tx")
        sp.add_argument("--meta")
        sp.add_argument("--expect-checksum", action="append", metavar="HEX",
                        help="expected wasm sha256 (repeat for multiple MsgStoreCode)")
        sp.add_argument("--online", action="store_true", help="query chain for code checksums / predicted addresses")
        sp.add_argument("--expect-multisig", metavar="ADDR|KEY",
                        help="multisig you expect (independent of meta.json; env MSIG_MULTISIG)")

    s = sub.add_parser("show", help="print a tx in readable form")
    review_common(s)

    sg2 = sub.add_parser("sign", help="(signer) review, verify, and sign with your key")
    review_common(sg2)
    sg2.add_argument("--from", dest="key", required=True, help="your keyring key name")
    sg2.add_argument("--signer", help="name used in the signature file name (default: key name)")
    sg2.add_argument("--ledger", action="store_true")
    sg2.add_argument("--yes", action="store_true", help="skip the confirmation prompt")
    sg2.add_argument("--force", action="store_true")

    cb = sub.add_parser("combine", help="(coordinator) verify and combine signatures")
    cb.add_argument("tx")
    cb.add_argument("sigs", nargs="+")
    cb.add_argument("--meta")
    cb.add_argument("--multisig-key", help="multisig key name in keyring (env MSIG_MULTISIG_KEY)")
    cb.add_argument("--skip-version-check", action="store_true")
    cb.add_argument("--force", action="store_true")

    b = sub.add_parser("broadcast", help="(coordinator) broadcast and wait for inclusion")
    b.add_argument("signed")
    b.add_argument("--meta")
    b.add_argument("--expect-address", action="append", metavar="ADDR")
    b.add_argument("--timeout", type=int, default=90)

    a = sub.add_parser("address", help="predict an instantiate2 address")
    a.add_argument("chain")
    a.add_argument("--checksum", required=True)
    ag2 = a.add_mutually_exclusive_group(required=True)
    ag2.add_argument("--salt", help="salt as ascii")
    ag2.add_argument("--salt-hex")
    a.add_argument("--creator", help="creator address or key name (default: MSIG_MULTISIG)")
    return p


COMMANDS = {"create": cmd_create, "merge": cmd_merge, "show": cmd_show, "sign": cmd_sign,
            "combine": cmd_combine, "broadcast": cmd_broadcast, "address": cmd_address}


def main(argv=None) -> int:
    args = build_parser().parse_args(argv)
    try:
        COMMANDS[args.cmd](args)
        return 0
    except MsigError as e:
        info(f"error: {e}")
        return e.exit_code
    except (json.JSONDecodeError, KeyError, ValueError, TypeError, AttributeError, EOFError, InvalidOperation,
            binascii.Error, gzip.BadGzipFile, zlib.error, OSError) as e:
        # 손상된 tx/meta/sig 파일이나 CLI 출력 → traceback 대신 사용법·환경 오류로 종료
        info(f"error: invalid input ({type(e).__name__}): {e}")
        return 1
    except KeyboardInterrupt:
        info("aborted")
        return 130


if __name__ == "__main__":
    sys.exit(main())
