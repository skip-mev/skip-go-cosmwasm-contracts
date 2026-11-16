"""tx를 사람이 읽을 수 있게 출력하고, 서명자가 준 기대값과 대조한다."""
import json
from dataclasses import dataclass, field
from decimal import Decimal

from . import txfile as tf


@dataclass
class Review:
    lines: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    errors: list[str] = field(default_factory=list)


def _json(obj) -> str:
    return json.dumps(obj, indent=2, sort_keys=True)


def _indent(text: str, n: int = 8) -> str:
    pad = " " * n
    return "\n".join(pad + line for line in text.splitlines())


def _salt_repr(raw: bytes) -> str:
    try:
        text = raw.decode("ascii")
        if text.isprintable():
            return f'"{text}" (hex {raw.hex()})'
    except UnicodeDecodeError:
        pass
    return f"hex {raw.hex()}"


def review(tx: dict, meta: dict, expect_checksums: list[str] | None = None,
           online: dict | None = None, gas_price: Decimal | None = None,
           fee_denom: str | None = None, expect_multisig: str | None = None) -> Review:
    """online: {"code_checksums": {code_id: hex}, "addresses": {msg_index: addr}} (선택).
    expect_multisig: 서명자가 독립적으로 알고 있는 multisig 주소. meta 값은 코디네이터가 만든 것이라 신뢰하지 않는다."""
    r = Review()
    multisig = meta.get("multisig", "?")
    online = online or {}

    r.lines.append(f"chain     {meta.get('chain_id', '?')} ({meta.get('chain', '?')})")
    if expect_multisig is None:
        r.lines.append(f"multisig  {multisig}   (from meta, NOT independently verified)")
        r.warnings.append("multisig address not verified: pass --expect-multisig (or MSIG_MULTISIG)")
    elif expect_multisig == multisig:
        r.lines.append(f"multisig  {multisig}   ✔ matches --expect-multisig")
    else:
        r.lines.append(f"multisig  {multisig}")
        r.errors.append(f"meta multisig {multisig} != expected {expect_multisig}")
    r.lines.append(f"account   {meta.get('account_number', '?')}   sequence {meta.get('sequence', '?')}")
    r.lines.append("")

    msgs = tf.messages(tx)
    if not msgs:
        r.errors.append("tx has no messages")

    for i, m in enumerate(msgs, start=1):
        t = m.get("@type", "?")
        r.lines.append(f"[{i}] {t}")
        sender = tf.signer_of(m)
        if sender is not None:
            mark = "(= multisig)" if sender == multisig else "!! NOT multisig"
            r.lines.append(f"    sender      {sender} {mark}")
            if sender != multisig:
                r.warnings.append(f"msg {i}: signer {sender} is not the multisig {multisig}")

        if t == tf.STORE:
            raw = tf.decode_wasm(m["wasm_byte_code"])
            digest = tf.wasm_sha256(m)
            r.lines.append(f"    wasm sha256 {digest}")
            r.lines.append(f"    wasm size   {len(raw):,} bytes")
            perm = m.get("instantiate_permission")
            r.lines.append(f"    instantiate {json.dumps(perm) if perm else 'default (chain param)'}")
        elif t in (tf.INSTANTIATE, tf.INSTANTIATE2):
            code_id = str(m.get("code_id"))
            r.lines.append(f"    code_id     {code_id}")
            if code_id in online.get("code_checksums", {}):
                r.lines.append(f"    checksum    {online['code_checksums'][code_id]} (on-chain)")
            r.lines.append(f"    label       {m.get('label')!r}")
            admin = m.get("admin") or ""
            admin_note = "(= multisig)" if admin == multisig else ("(none, immutable)" if not admin else "")
            r.lines.append(f"    admin       {admin or '-'} {admin_note}".rstrip())
            if t == tf.INSTANTIATE2:
                r.lines.append(f"    salt        {_salt_repr(tf.decode_salt(m.get('salt', '')))}")
                r.lines.append(f"    fix_msg     {m.get('fix_msg', False)}")
                if i in online.get("addresses", {}):
                    r.lines.append(f"    address     {online['addresses'][i]} (predicted)")
            if m.get("funds"):
                r.lines.append(f"    funds       {m['funds']}")
            r.lines.append("    msg")
            r.lines.append(_indent(_json(tf.decode_contract_msg(m.get("msg")))))
        elif t in (tf.EXECUTE, tf.MIGRATE):
            r.lines.append(f"    contract    {m.get('contract')}")
            if t == tf.MIGRATE:
                r.lines.append(f"    code_id     {m.get('code_id')}")
            if m.get("funds"):
                r.lines.append(f"    funds       {m['funds']}")
            r.lines.append("    msg")
            r.lines.append(_indent(_json(tf.decode_contract_msg(m.get("msg")))))
        elif t in (tf.UPDATE_ADMIN, tf.CLEAR_ADMIN):
            r.lines.append(f"    contract    {m.get('contract')}")
            if t == tf.UPDATE_ADMIN:
                r.lines.append(f"    new_admin   {m.get('new_admin')}")
        else:
            body = {k: v for k, v in m.items() if k != "@type"}
            r.lines.append(_indent(_json(body), 4))
        r.lines.append("")

    body = tx.get("body", {})
    gas, amount = tf.tx_fee(tx)
    fee_str = ", ".join(f"{c['amount']}{c['denom']}" for c in amount) or "0"
    r.lines.append(f"fee       {fee_str}   gas {gas:,}")
    r.lines.append(f"memo      {body.get('memo', '')!r}")
    if body.get("memo"):
        r.warnings.append(f"memo is not empty: {body['memo']!r}")
    if str(body.get("timeout_height", "0")) not in ("0", ""):
        r.warnings.append(f"timeout_height is set: {body['timeout_height']}")
    if fee_denom:
        for c in amount:
            if c["denom"] != fee_denom:
                r.warnings.append(f"unexpected fee denom {c['denom']} (chain fee denom is {fee_denom})")
    if gas_price and amount:
        limit = Decimal(gas) * gas_price * 3
        total = sum(Decimal(c["amount"]) for c in amount if not fee_denom or c["denom"] == fee_denom)
        if total > limit:
            r.warnings.append(f"fee {total}{fee_denom or ''} exceeds 3x expected ({limit:.0f})")

    _check_checksums(tx, expect_checksums, r)
    return r


def _check_checksums(tx: dict, expect: list[str] | None, r: Review) -> None:
    actual = tf.store_checksums(tx)
    if not actual:
        if expect:
            r.errors.append("--expect-checksum given but tx has no MsgStoreCode")
        return
    if not expect:
        r.warnings.append("MsgStoreCode present but no --expect-checksum given: wasm content is NOT verified")
        return

    want = sorted(c.lower().removeprefix("0x") for c in expect)
    got = sorted(actual)
    if want == got:
        r.lines.append(f"checksum  ✔ all {len(got)} wasm match --expect-checksum")
        return
    missing = [c for c in want if c not in got]
    extra = [c for c in got if c not in want]
    for c in extra:
        r.errors.append(f"wasm sha256 {c} in tx is NOT in --expect-checksum")
    for c in missing:
        r.errors.append(f"expected checksum {c} not found in tx")
    if not missing and not extra:
        r.errors.append(f"checksum count mismatch: expected {len(want)}, tx has {len(got)}")


def print_review(r: Review, out) -> None:
    for line in r.lines:
        print(line, file=out)
    for w in r.warnings:
        print(f"WARNING   {w}", file=out)
    for e in r.errors:
        print(f"ERROR     {e}", file=out)
