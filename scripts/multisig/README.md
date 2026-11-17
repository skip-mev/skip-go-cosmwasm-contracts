# msig — multisig tx tool

A tool to **create → show → sign → combine → broadcast** multisig txs on Cosmos SDK chains.
Signing and encoding are handled by the chain CLI (`gaiad`, etc.); this tool **shows signers what they are signing in readable form and checks it against expected checksums**.

- Python 3.11+ standard library only (nothing to install)
- Never handles mnemonics or private keys. Keys must live in the chain CLI keyring (or on a Ledger).

```bash
alias msig="python3 $PWD/scripts/multisig/msig.py"   # define at the repo root (zsh does not word-split "$M")
```

## Signer

Your keyring must hold the multisig key as well (`gaiad keys add <name> --multisig <k1>,<k2>,<k3> --multisig-threshold <t> --nosort`, others added by `--pubkey`); `gaiad tx sign --multisig` refuses otherwise.

You receive: `<name>.tx.json`, `<name>.tx.meta.json`
And, **through a separate channel**: the wasm checksum (your own build's `artifacts/checksums.txt`, or the value published in the PR/release)

```bash
msig sign $RUN/store_skip_go_entry_point.tx.json \
  --from <my key name> [--ledger] \
  --expect-multisig <multisig address> \
  --expect-checksum <wasm sha256>
```

- The tx content is printed. Check `wasm sha256`, `sender`, `admin`, `msg`, then enter `y`.
- If the wasm hash in the tx differs from `--expect-checksum`, **signing is refused**.
- If the multisig in meta.json differs from `--expect-multisig` (or `MSIG_MULTISIG`), **signing is refused**. Use the multisig address you already know, not the one from the coordinator's files.
- Output is `<name>.tx.sig.<key name>.json`. Send only this file to the coordinator.
- With `--online` on an instantiate tx, the on-chain checksum of the code_id and the predicted address are shown as well.

To review without signing, run `msig show <tx> [--expect-checksum ..] [--online]`.

## Run folder (runs/)

One deployment = one folder: `scripts/multisig/runs/<date>-<chain>-<purpose>/` (e.g. `2026-11-20-cosmoshub-eureka`).
`create` writes txs into this folder, and `sign`/`combine`/`broadcast` write their output next to the input tx, so all files of one deployment stay together.
Exchange the whole folder via git. Only `*.tx.signed.json` is gitignored (reproducible, prevents re-broadcast).

```bash
export MSIG_WORK_DIR=scripts/multisig/runs/2026-11-20-cosmoshub-eureka
RUN=$MSIG_WORK_DIR
```

1. Coordinator: `create` → commit and push `*.tx.json`, `*.tx.meta.json`
2. Signers: pull → `sign` → commit and push `*.tx.sig.<key>.json` (one file per signer, so no conflicts)
3. Coordinator: pull → `combine` → `broadcast` → commit `*.tx.result.json`

## Coordinator

```bash
export MSIG_MULTISIG=<multisig address or multisig key name in the keyring>

# 1) store (one wasm = one tx). Pre-build consecutive txs with --sequence
msig create cosmoshub store artifacts/skip_go_ibc_adapter_ibc_callbacks.wasm
msig create cosmoshub store artifacts/skip_go_entry_point.wasm --sequence <S+1>

# 2) after collecting signatures: combine (verifies signatures) → broadcast
msig combine $RUN/store_skip_go_entry_point.tx.json $RUN/store_skip_go_entry_point.tx.sig.*.json --multisig-key <ms key>
msig broadcast $RUN/store_skip_go_entry_point.tx.signed.json        # prints code_id

# 3) instantiate2: predict addresses → create two txs → merge into one
#    use --admin multisig|<address> or --no-admin
msig address cosmoshub --checksum <EP_SUM> --salt skip-eureka-ep-v3
msig address cosmoshub --checksum <CB_SUM> --salt skip-eureka-cb-v3
msig create cosmoshub instantiate2 --code-id <CB_ID> --salt skip-eureka-cb-v3 --admin multisig \
   --label "Skip Go IBC Callbacks Adapter" --msg '{"entry_point_contract_address":"<EP_ADDR>"}' -o $RUN/i_cb.tx.json
msig create cosmoshub instantiate2 --code-id <EP_ID> --salt skip-eureka-ep-v3 --admin multisig \
   --label "Skip Go Entry Point" --msg '{"swap_venues":[],"ibc_transfer_contract_address":"<CB_ADDR>"}' -o $RUN/i_ep.tx.json
msig merge $RUN/i_cb.tx.json $RUN/i_ep.tx.json -o $RUN/instantiate.tx.json
# ... sign / combine ...
msig broadcast $RUN/instantiate.tx.signed.json --expect-address <CB_ADDR> --expect-address <EP_ADDR>

# other txs: arguments after `--` are passed through to `<bin> tx`
msig create cosmoshub tx -- wasm migrate <contract> <code_id> '{}'
```

- `create` and `combine` check that the local CLI's cosmos-sdk, `x/tx` and wasmd versions **exactly match** the chain node (abort otherwise). Signers only need the same minor line. Signatures from mismatched versions are caught by `combine`'s signature verification.
- Announce the tx file sha256 and wasm checksum printed by `create` through a **different channel** from the tx files.

### Options and environment variables
| | |
|---|---|
| `MSIG_MULTISIG` | multisig address or key name (`--multisig`) |
| `MSIG_MULTISIG_KEY` | multisig key name used by combine (`--multisig-key`) |
| `MSIG_KEYRING_BACKEND`, `MSIG_HOME` | CLI keyring settings (`--keyring-backend`, `--home`) |
| `MSIG_BIN` | override the chain CLI path |
| `MSIG_WORK_DIR` | output location for `create` (`--work-dir`). Default `scripts/multisig/work/` (gitignored, for personal practice) |

### Exit codes
`0` success · `1` usage/environment · `2` verification failed/refused · `3` CLI failure · `4` chain error

## Installing the chain CLI (gaiad example)

Build the same version as the chain node from source. As of 2026-11, the official darwin-amd64 release binary does not run on macOS (`dyld: __DATA_CONST segment missing SG_READ_ONLY flag`).

```bash
git clone --depth 1 --branch v29.0.1 https://github.com/cosmos/gaia.git && cd gaia
GOTOOLCHAIN=auto make install      # automatically fetches the Go version go.mod requires
gaiad version --long
```

## Adding a chain

Add `chains/<chain>.toml` (see `chains/cosmoshub.toml`). Check the following first:
- [ ] CLI name (`bin`), chain_id, bech32 prefix, fee denom, endpoints (`https://[rpc|lcd]-<chain>.mainnet.cosmoslabs.kr`)
- [ ] gas price policy: `query` if a feemarket query path exists, otherwise `fixed`
- [ ] key type is `secp256k1` (injective/evmos-family chains need separate review)
- [ ] code upload permission (`/cosmwasm/wasm/v1/codes/params`), instantiate2 support
- [ ] `msig show` renders store and instantiate2 correctly

## Tests
```bash
python3 -m unittest discover -s scripts/multisig/tests -v
```
