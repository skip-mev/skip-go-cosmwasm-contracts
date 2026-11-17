import base64
import copy
import gzip
import hashlib
import json
import sys
import tempfile
import unittest
from decimal import Decimal
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from msig_lib import txfile as tf  # noqa: E402
from msig_lib.config import load_chain  # noqa: E402
from msig_lib.errors import UsageError  # noqa: E402
from msig_lib.render import review  # noqa: E402
from msig_lib.versions import diff_exact, diff_minor, parse_cli_build_deps  # noqa: E402

MS = "cosmos1multisig"
WASM = b"\x00asm\x01\x00\x00\x00fake-contract"
WASM_SUM = hashlib.sha256(WASM).hexdigest()
META = {"chain": "cosmoshub", "chain_id": "cosmoshub-4", "multisig": MS, "account_number": "7", "sequence": "3"}


def unsigned(msgs, gas="8000000", fee="80000"):
    return {
        "body": {"messages": msgs, "memo": "", "timeout_height": "0",
                 "extension_options": [], "non_critical_extension_options": []},
        "auth_info": {"signer_infos": [], "fee": {"amount": [{"denom": "uatom", "amount": fee}],
                                                   "gas_limit": gas, "payer": "", "granter": ""}},
        "signatures": [],
    }


def store_msg(wasm=WASM, gz=True):
    data = gzip.compress(wasm) if gz else wasm
    return {"@type": tf.STORE, "sender": MS, "wasm_byte_code": base64.b64encode(data).decode(),
            "instantiate_permission": {"permission": "AnyOfAddresses", "addresses": [MS]}}


def inst2_msg(msg, salt=b"skip-ep-v3", encode_msg_b64=False):
    m = json.dumps(msg)
    return {"@type": tf.INSTANTIATE2, "sender": MS, "admin": MS, "code_id": "301",
            "label": "Skip Go Entry Point",
            "msg": base64.b64encode(m.encode()).decode() if encode_msg_b64 else msg,
            "funds": [], "salt": base64.b64encode(salt).decode(), "fix_msg": False}


class DecodeTest(unittest.TestCase):
    def test_wasm_gzip_and_plain(self):
        self.assertEqual(tf.wasm_sha256(store_msg(gz=True)), WASM_SUM)
        self.assertEqual(tf.wasm_sha256(store_msg(gz=False)), WASM_SUM)

    def test_corrupt_wasm_rejected(self):
        for bad in ("!!!", base64.b64encode(b"not wasm").decode(), base64.b64encode(gzip.compress(b"nope")).decode()):
            with self.assertRaises(UsageError, msg=bad):
                tf.decode_wasm(bad)

    def test_contract_msg_json_or_b64(self):
        body = {"swap_venues": [], "ibc_transfer_contract_address": "cosmos1cb"}
        self.assertEqual(tf.decode_contract_msg(inst2_msg(body)["msg"]), body)
        self.assertEqual(tf.decode_contract_msg(inst2_msg(body, encode_msg_b64=True)["msg"]), body)

    def test_salt(self):
        self.assertEqual(tf.decode_salt(inst2_msg({})["salt"]), b"skip-ep-v3")
        with self.assertRaises(UsageError):
            tf.decode_salt("not base64!")

    def test_signer_fields(self):
        self.assertEqual(tf.signer_of({"from_address": "a"}), "a")
        self.assertEqual(tf.signer_of({"sender": "b"}), "b")
        self.assertIsNone(tf.signer_of({"contract": "c"}))


class ChecksumTest(unittest.TestCase):
    def test_match(self):
        r = review(unsigned([store_msg()]), META, expect_checksums=[WASM_SUM])
        self.assertEqual(r.errors, [])

    def test_tampered_wasm_rejected(self):
        tampered = unsigned([store_msg(WASM + b"backdoor")])
        r = review(tampered, META, expect_checksums=[WASM_SUM])
        self.assertTrue(any("NOT in --expect-checksum" in e for e in r.errors))

    def test_missing_expect_is_warning_not_error(self):
        r = review(unsigned([store_msg()]), META)
        self.assertEqual(r.errors, [])
        self.assertTrue(any("NOT verified" in w for w in r.warnings))

    def test_extra_store_msg_rejected(self):
        tx = unsigned([store_msg(), store_msg(WASM + b"other")])
        r = review(tx, META, expect_checksums=[WASM_SUM])
        self.assertTrue(r.errors)

    def test_expected_but_absent(self):
        other = hashlib.sha256(b"other").hexdigest()
        r = review(unsigned([store_msg()]), META, expect_checksums=[WASM_SUM, other])
        self.assertTrue(any("not found in tx" in e for e in r.errors))

    def test_expect_without_store(self):
        r = review(unsigned([inst2_msg({})]), META, expect_checksums=[WASM_SUM])
        self.assertTrue(r.errors)

    def test_case_and_prefix_insensitive(self):
        r = review(unsigned([store_msg()]), META, expect_checksums=["0x" + WASM_SUM.upper()])
        self.assertEqual(r.errors, [])


class ReviewWarningTest(unittest.TestCase):
    def test_foreign_sender_and_memo(self):
        msg = store_msg()
        msg["sender"] = "cosmos1attacker"
        tx = unsigned([msg])
        tx["body"]["memo"] = "hi"
        r = review(tx, META, expect_checksums=[WASM_SUM])
        self.assertTrue(any("not the multisig" in w for w in r.warnings))
        self.assertTrue(any("memo" in w for w in r.warnings))

    def test_fee_cap_warning(self):
        r = review(unsigned([store_msg()], fee="999999999"), META, expect_checksums=[WASM_SUM],
                   gas_price=Decimal("0.01"))
        self.assertTrue(any("exceeds" in w for w in r.warnings))

    def test_instantiate_rendering(self):
        body = {"entry_point_contract_address": "cosmos1ep"}
        r = review(unsigned([inst2_msg(body, salt=b"skip-cb-v3")]), META)
        text = "\n".join(r.lines)
        self.assertIn('"skip-cb-v3"', text)
        self.assertIn("cosmos1ep", text)
        self.assertIn("(= multisig)", text)


class ExpectMultisigTest(unittest.TestCase):
    def test_match(self):
        r = review(unsigned([store_msg()]), META, expect_checksums=[WASM_SUM], expect_multisig=MS)
        self.assertEqual(r.errors, [])
        self.assertFalse(any("not verified" in w for w in r.warnings))

    def test_meta_swapped_multisig_blocked(self):
        # Even if the coordinator swaps meta and sender to another multisig, block when it differs from the signer's expectation
        evil = "cosmos1evilmultisig"
        msg = store_msg()
        msg["sender"] = evil
        r = review(unsigned([msg]), dict(META, multisig=evil), expect_checksums=[WASM_SUM], expect_multisig=MS)
        self.assertTrue(any("!= expected" in e for e in r.errors))

    def test_not_given_is_warning(self):
        r = review(unsigned([store_msg()]), META, expect_checksums=[WASM_SUM])
        self.assertEqual(r.errors, [])
        self.assertTrue(any("multisig address not verified" in w for w in r.warnings))

    def test_bank_send_signer_checked(self):
        msg = {"@type": "/cosmos.bank.v1beta1.MsgSend", "from_address": "cosmos1other", "to_address": "x", "amount": []}
        r = review(unsigned([msg]), META, expect_multisig=MS)
        self.assertTrue(any("not the multisig" in w for w in r.warnings))


class FeeTest(unittest.TestCase):
    def test_other_denom_warned_and_not_summed(self):
        tx = unsigned([store_msg()], fee="80000")
        tx["auth_info"]["fee"]["amount"].append({"denom": "ibc/X", "amount": "999999999"})
        r = review(tx, META, expect_checksums=[WASM_SUM], gas_price=Decimal("0.01"), fee_denom="uatom")
        self.assertTrue(any("unexpected fee denom" in w for w in r.warnings))
        self.assertFalse(any("exceeds" in w for w in r.warnings))


FIXTURES = Path(__file__).resolve().parent / "fixtures"


class GaiadFixtureTest(unittest.TestCase):
    """Real gaiad v29.0.1 --generate-only / tx sign output."""

    def test_store(self):
        tx = tf.read_json(FIXTURES / "gaiad_v29_store.tx.json")
        meta = tf.read_json(FIXTURES / "gaiad_v29_store.tx.meta.json")
        want = hashlib.sha256(b"\x00asm\x01\x00\x00\x00").hexdigest()
        self.assertEqual(tf.store_checksums(tx), [want])
        r = review(tx, meta, expect_checksums=[want], expect_multisig=meta["multisig"], fee_denom="uatom")
        self.assertEqual(r.errors, [])

    def test_instantiate2_merged(self):
        tx = tf.read_json(FIXTURES / "gaiad_v29_instantiate2_merged.tx.json")
        msgs = tf.messages(tx)
        self.assertEqual([m["@type"] for m in msgs], [tf.INSTANTIATE2, tf.INSTANTIATE2])
        self.assertIsInstance(msgs[0]["msg"], dict, "gaiad v29 emits msg as a JSON object")
        self.assertEqual(tf.decode_salt(msgs[0]["salt"]), b"skip-cb-v3")
        self.assertIn("entry_point_contract_address", tf.decode_contract_msg(msgs[0]["msg"]))
        self.assertEqual(tx["auth_info"]["fee"]["gas_limit"], "1000000")

    def test_sig_pubkeys(self):
        sig = tf.read_json(FIXTURES / "gaiad_v29_store.tx.sig.alice.json")
        keys = tf.sig_pubkeys(sig)
        self.assertEqual(len(keys), 1)
        self.assertTrue(keys[0])


class MergeTest(unittest.TestCase):
    def test_merge_sums_gas_and_fee(self):
        a = unsigned([inst2_msg({"a": 1})], gas="500000", fee="5000")
        b = unsigned([inst2_msg({"b": 2})], gas="500000", fee="5000")
        m = tf.merge([a, b])
        self.assertEqual(len(tf.messages(m)), 2)
        self.assertEqual(tf.messages(m)[0]["msg"], {"a": 1})
        self.assertEqual(m["auth_info"]["fee"]["gas_limit"], "1000000")
        self.assertEqual(m["auth_info"]["fee"]["amount"], [{"denom": "uatom", "amount": "10000"}])
        self.assertEqual(len(tf.messages(a)), 1, "inputs must not be mutated")

    def test_merge_rejects_memo_mismatch(self):
        a, b = unsigned([inst2_msg({})]), unsigned([inst2_msg({})])
        b["body"]["memo"] = "x"
        with self.assertRaises(UsageError):
            tf.merge([a, b])

    def test_merge_rejects_signed(self):
        a, b = unsigned([inst2_msg({})]), unsigned([inst2_msg({})])
        b["signatures"] = ["sig"]
        with self.assertRaises(UsageError):
            tf.merge([a, b])

    def test_merge_meta_mismatch(self):
        for k, v in (("sequence", "4"), ("chain_id", "osmosis-1"), ("multisig", "cosmos1x")):
            with self.assertRaises(UsageError, msg=k):
                tf.merge_meta([META, dict(META, **{k: v})])
        self.assertEqual(tf.merge_meta([META, dict(META)]), META)


class PathTest(unittest.TestCase):
    def test_derived_paths(self):
        p = Path("work/store_ep.tx.json")
        self.assertEqual(tf.meta_path(p), Path("work/store_ep.tx.meta.json"))
        self.assertEqual(tf.sig_path(p, "alice"), Path("work/store_ep.tx.sig.alice.json"))
        self.assertEqual(tf.signed_path(p), Path("work/store_ep.tx.signed.json"))
        with self.assertRaises(UsageError):
            tf.meta_path(Path("store.json"))


class VersionTest(unittest.TestCase):
    def test_parse_cli_deps_with_replace_uses_original(self):
        # node_info Module.Version is pre-replace, so normalize the local side the same way
        deps = [
            "github.com/cosmos/cosmos-sdk@v0.50.13 => github.com/osmosis-labs/cosmos-sdk@v0.50.13-v28-osmo-2",
            "cosmossdk.io/x/tx@v0.14.0",
            "github.com/CosmWasm/wasmd@v0.60.9",
            "github.com/other/lib@v1.0.0",
        ]
        local = parse_cli_build_deps(deps)
        self.assertEqual(local, {"cosmos_sdk": "v0.50.13", "x_tx": "v0.14.0", "wasmd": "v0.60.9"})
        chain = parse_cli_build_deps([{"path": "github.com/cosmos/cosmos-sdk", "version": "v0.50.13"}])
        self.assertEqual(diff_exact(local, chain), [])

    def test_empty_chain_deps_fail(self):
        self.assertTrue(diff_exact({"cosmos_sdk": "v0.53.8"}, {}))

    def test_parse_node_deps(self):
        deps = [{"path": "github.com/cosmos/cosmos-sdk", "version": "v0.53.8", "sum": "h1:x"}]
        self.assertEqual(parse_cli_build_deps(deps), {"cosmos_sdk": "v0.53.8"})

    def test_diffs(self):
        chain = {"cosmos_sdk": "v0.53.8", "x_tx": "v0.14.0", "wasmd": "v0.60.9"}
        self.assertEqual(diff_exact(dict(chain), chain), [])
        patch = dict(chain, cosmos_sdk="v0.53.7")
        self.assertEqual(len(diff_exact(patch, chain)), 1)
        self.assertEqual(diff_minor(patch, chain), [])
        self.assertEqual(len(diff_minor(dict(chain, wasmd="v0.61.0"), chain)), 1)


class ConfigTest(unittest.TestCase):
    def test_cosmoshub(self):
        cfg = load_chain("cosmoshub")
        self.assertEqual(cfg.chain_id, "cosmoshub-4")
        self.assertEqual(cfg.default_gas("store"), 8000000)
        self.assertEqual(cfg.default_gas("unknown"), 500000)
        self.assertEqual(cfg.fee_amount(8000000, Decimal("0.01")), 80000)
        self.assertEqual(cfg.fee_amount(3, Decimal("0.5")), 2)

    def test_chain_name_path_traversal_rejected(self):
        for bad in ("../work/x", "cosmoshub/../x", "/etc/x", "Cosmoshub", "", "a b"):
            with self.assertRaises(UsageError, msg=bad):
                load_chain(bad)

    def test_unknown_and_invalid(self):
        with self.assertRaises(UsageError):
            load_chain("nope")
        with tempfile.TemporaryDirectory() as d:
            Path(d, "bad.toml").write_text('chain_id = "x"\n')
            with self.assertRaises(UsageError):
                load_chain("bad", Path(d))


if __name__ == "__main__":
    unittest.main()
