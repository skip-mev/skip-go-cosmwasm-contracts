# 2026-11-17 cosmoshub — entry point / ibc callbacks redeploy (signing guide)

- multisig: `cosmos13t73c9jehdr8hhlmrajmlcmd03zh97kdkwrnap` (2-of-3, account 3729500)
- txs to sign

| file | sequence | wasm sha256 |
|---|---|---|
| `store_skip_go_ibc_adapter_ibc_callbacks.tx.json` | 1 | `41a218089e021c955c91c24291fd11c5baf5dbf044f83fbcee34e5e6633e2fa3` |
| `store_skip_go_entry_point.tx.json` | 2 | `9e95b47ecf33f0adad11dac99fc2c523898a674ddbcfac322d09b6f96a5d53ac` |

> Verify the multisig address, wasm sha256 and tx file sha256 against the values **received through a separate channel**, not against this file.

## 1. Prerequisites

- git, make, Go (`GOTOOLCHAIN=auto` fetches the required version)
- Python 3.11+ (no extra packages)

## 2. Install gaiad (v29.0.1)

Build the same version as the chain node from source. The official darwin release binary does not run on macOS.

```bash
git clone --depth 1 --branch v29.0.1 https://github.com/cosmos/gaia.git && cd gaia
GOTOOLCHAIN=auto make install
gaiad version --long | grep -E '^version|cosmos_sdk_version'
# version: v29.0.1
# cosmos_sdk_version: v0.53.8
```

If you get `gaiad: command not found`, run `export PATH=$PATH:$(go env GOPATH)/bin`.

## 3. Add your key

If your key is already in the gaiad keyring, skip to step 4.

**Ledger** (Cosmos app open, device unlocked)
```bash
gaiad keys add <key name> --ledger
```

**Recover from mnemonic**
```bash
gaiad keys add <key name> --recover
# Enter the mnemonic. The default macOS keyring (os) stores it in Keychain
```

If you use the `file`/`test` keyring backend, set it before the following commands, e.g. `export MSIG_KEYRING_BACKEND=file`.

## 4. Check membership and add the multisig key

```bash
gaiad keys show <key name> -p
```

The `key` value in the output must be one of the three below (on-chain order).

```
1: A04K9pcxy3DCQZOjvCBC68si0cFkgGyhtoIM4g8GwcgC
2: ApYEx8aqbo8efL3ss2x34SZJAkqzj1aCP4AbPyy8yjKj
3: Ar4WSqW4U9GhYmjt6UnY8GbYq5r5rqitldNDwG4c7Vm9
```

`gaiad tx sign` refuses to sign unless the multisig key is in your keyring (`key with address cosmos13t73… not found`).
Add the **other two** members by public key only, then build the multisig key in the order above with your own key in its slot.

```bash
# e.g. if your key is #2
gaiad keys add msig-m1 --pubkey '{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"A04K9pcxy3DCQZOjvCBC68si0cFkgGyhtoIM4g8GwcgC"}'
gaiad keys add msig-m3 --pubkey '{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"Ar4WSqW4U9GhYmjt6UnY8GbYq5r5rqitldNDwG4c7Vm9"}'
gaiad keys add hub-msig --multisig msig-m1,<key name>,msig-m3 --multisig-threshold 2 --nosort

gaiad keys show hub-msig -a   # must be cosmos13t73c9jehdr8hhlmrajmlcmd03zh97kdkwrnap
```

If the address differs, the order is wrong: `gaiad keys delete hub-msig` and add it again.

## 5. Sign

Run from the repo root. Signing is offline; no node configuration is needed.

```bash
git fetch && git checkout feat/hub-ep-and-cb-upgrade && git pull
alias msig="python3 $PWD/scripts/multisig/msig.py"   # define at the repo root (pins the absolute path)
RUN=scripts/multisig/runs/2026-11-17-cosmoshub-ep-cb-upgrade
MS=cosmos13t73c9jehdr8hhlmrajmlcmd03zh97kdkwrnap

# (optional) compare tx file hashes with the values received through a separate channel
shasum -a 256 $RUN/*.tx.json

msig sign $RUN/store_skip_go_ibc_adapter_ibc_callbacks.tx.json \
  --from <key name> --expect-multisig $MS \
  --expect-checksum 41a218089e021c955c91c24291fd11c5baf5dbf044f83fbcee34e5e6633e2fa3

msig sign $RUN/store_skip_go_entry_point.tx.json \
  --from <key name> --expect-multisig $MS \
  --expect-checksum 9e95b47ecf33f0adad11dac99fc2c523898a674ddbcfac322d09b6f96a5d53ac
```

- For a Ledger key, add `--ledger` and approve on the device.
- Check `sender`, `sequence` and `wasm sha256` on screen, then enter `y`.
- Output: two files, `$RUN/*.tx.sig.<key name>.json`.

## 6. Send your signatures

```bash
git add $RUN/*.tx.sig.*.json
git commit -m "sign: <key name> store txs"
git push
```

If you don't have push access, send only the two `*.tx.sig.<key name>.json` files to the coordinator.

## 7. Combine and broadcast (coordinator)

Once two signatures are collected, combine them (signatures are verified) and broadcast. `combine` uses the `hub-msig` key from step 4.

```bash
# Run in a shell where alias msig, RUN and MS from step 5 are defined
git pull

# Order matters: callbacks (seq 1) → entry_point (seq 2)
msig combine $RUN/store_skip_go_ibc_adapter_ibc_callbacks.tx.json $RUN/store_skip_go_ibc_adapter_ibc_callbacks.tx.sig.*.json --multisig-key hub-msig
msig broadcast $RUN/store_skip_go_ibc_adapter_ibc_callbacks.tx.signed.json   # → CB_ID

msig combine $RUN/store_skip_go_entry_point.tx.json $RUN/store_skip_go_entry_point.tx.sig.*.json --multisig-key hub-msig
msig broadcast $RUN/store_skip_go_entry_point.tx.signed.json                 # → EP_ID
```

- Each broadcast writes `*.tx.result.json`; read the code id from it and commit the `*.tx.result.json` files.

## 8. Instantiate (sequence 3)

Do this after the code ids are known. Both contracts use instantiate2 so their addresses are precomputed and can reference each other, and are **deployed without an admin (`--no-admin`)** (no migrate, immutable).

```bash
export MSIG_MULTISIG=cosmos13t73c9jehdr8hhlmrajmlcmd03zh97kdkwrnap
export MSIG_WORK_DIR=$RUN
CB_SUM=41a218089e021c955c91c24291fd11c5baf5dbf044f83fbcee34e5e6633e2fa3
EP_SUM=9e95b47ecf33f0adad11dac99fc2c523898a674ddbcfac322d09b6f96a5d53ac
CB_SALT=<cb salt>   # must not collide with a previous deployment
EP_SALT=<ep salt>

# 1) predict addresses
msig address cosmoshub --checksum $CB_SUM --salt $CB_SALT   # → CB_ADDR
msig address cosmoshub --checksum $EP_SUM --salt $EP_SALT   # → EP_ADDR

# 2) create two txs → merge into one
msig create cosmoshub instantiate2 --code-id <CB_ID> --salt $CB_SALT --no-admin \
  --label "Skip Go IBC Callbacks Adapter" \
  --msg '{"entry_point_contract_address":"<EP_ADDR>"}' -o $RUN/i_cb.tx.json
msig create cosmoshub instantiate2 --code-id <EP_ID> --salt $EP_SALT --no-admin \
  --label "Skip Go Entry Point" \
  --msg '{"swap_venues":[],"ibc_transfer_contract_address":"<CB_ADDR>"}' -o $RUN/i_ep.tx.json
msig merge $RUN/i_cb.tx.json $RUN/i_ep.tx.json -o $RUN/instantiate.tx.json
msig show $RUN/instantiate.tx.json --online   # check sequence 3 and admin "(none, immutable)"
```

Signers sign the same way as in steps 5–6. With `--online`, the on-chain checksum of each code id and the predicted addresses are shown as well.
`--expect-checksum` only applies to store txs, so check by eye: the code id of the callbacks msg must show `41a21808…`, the entry point msg `9e95b47e…`, and both must show admin `(none, immutable)`.

```bash
msig sign $RUN/instantiate.tx.json --from <key name> --expect-multisig $MS --online
```

Once two signatures are collected, combine and broadcast while verifying the predicted addresses.

```bash
msig combine $RUN/instantiate.tx.json $RUN/instantiate.tx.sig.*.json --multisig-key hub-msig
msig broadcast $RUN/instantiate.tx.signed.json --expect-address <CB_ADDR> --expect-address <EP_ADDR>
```
