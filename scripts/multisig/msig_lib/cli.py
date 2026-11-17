"""Subprocess wrapper for the chain CLI (gaiad, etc.). Signing, encoding and address derivation are all delegated to the CLI."""
import json
import os
import shutil
import subprocess

from .errors import CliError, UsageError
from .versions import parse_cli_build_deps


class ChainCli:
    def __init__(self, bin_name: str, keyring_backend: str | None = None, home: str | None = None):
        self.bin = os.environ.get("MSIG_BIN") or bin_name
        if not shutil.which(self.bin):
            raise UsageError(f"'{self.bin}' not found in PATH (set MSIG_BIN to override)")
        self.keyring_backend = keyring_backend or os.environ.get("MSIG_KEYRING_BACKEND")
        self.home = home or os.environ.get("MSIG_HOME")

    def _keyring_args(self) -> list[str]:
        args = []
        if self.keyring_backend:
            args += ["--keyring-backend", self.keyring_backend]
        if self.home:
            args += ["--home", self.home]
        return args

    def run(self, args: list[str], keyring: bool = False, check: bool = True) -> subprocess.CompletedProcess:
        cmd = [self.bin, *args, *(self._keyring_args() if keyring else [])]
        proc = subprocess.run(cmd, capture_output=True, text=True)
        if check and proc.returncode != 0:
            raise CliError(f"{' '.join(cmd[:4])} ... failed (exit {proc.returncode}):\n{proc.stderr.strip() or proc.stdout.strip()}")
        return proc

    def versions(self) -> dict:
        out = self.run(["version", "--long", "--output", "json"]).stdout
        try:
            data = json.loads(out)
        except json.JSONDecodeError as e:
            raise CliError(f"cannot parse `{self.bin} version --long --output json`: {e}") from e
        return parse_cli_build_deps(data.get("build_deps", []))

    def key_address(self, key_name: str) -> str:
        return self.run(["keys", "show", key_name, "-a"], keyring=True).stdout.strip()

    def multisig_threshold(self, key_name: str) -> int:
        data = json.loads(self.run(["keys", "show", key_name, "--output", "json"], keyring=True).stdout)
        pubkey = data.get("pubkey")
        pubkey = json.loads(pubkey) if isinstance(pubkey, str) else pubkey
        if not pubkey or "threshold" not in pubkey:
            raise UsageError(f"key '{key_name}' is not a multisig key")
        return int(pubkey["threshold"])

    def generate(self, tx_args: list[str]) -> dict:
        out = self.run(["tx", *tx_args]).stdout
        try:
            return json.loads(out)
        except json.JSONDecodeError as e:
            raise CliError(f"generate-only output is not JSON:\n{out[:500]}") from e

    def build_address(self, checksum_hex: str, creator: str, salt_hex: str) -> str:
        out = self.run(["q", "wasm", "build-address", checksum_hex, creator, salt_hex]).stdout
        return out.strip().splitlines()[-1].strip()

    def sign(self, tx_path, multisig: str, key: str, chain_id: str, account_number: str,
             sequence: str, out_path, ledger: bool = False) -> None:
        args = ["tx", "sign", str(tx_path), "--multisig", multisig, "--from", key,
                "--sign-mode", "amino-json", "--offline", "--chain-id", chain_id,
                "--account-number", account_number, "--sequence", sequence,
                "--output-document", str(out_path)]
        if ledger:
            args.append("--ledger")
        self.run(args, keyring=True)

    def multisign(self, tx_path, multisig_key: str, sig_paths: list, chain_id: str,
                  account_number: str, sequence: str, out_path) -> subprocess.CompletedProcess:
        # Intentionally never pass --skip-signature-verification
        args = ["tx", "multisign", str(tx_path), multisig_key, *map(str, sig_paths),
                "--offline", "--chain-id", chain_id, "--account-number", account_number,
                "--sequence", sequence, "--output-document", str(out_path)]
        return self.run(args, keyring=True, check=False)

    def broadcast(self, signed_path, node: str) -> dict:
        out = self.run(["tx", "broadcast", str(signed_path), "--node", node, "--output", "json"]).stdout
        return json.loads(out)

    def query_tx(self, txhash: str, node: str) -> dict | None:
        proc = self.run(["q", "tx", txhash, "--node", node, "--output", "json"], check=False)
        if proc.returncode != 0:
            if "not found" in (proc.stderr + proc.stdout).lower():
                return None
            raise CliError(f"q tx {txhash} failed:\n{proc.stderr.strip()}")
        return json.loads(proc.stdout)
