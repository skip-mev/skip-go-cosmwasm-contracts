# msig — multisig tx tool

Cosmos SDK 체인의 multisig tx를 **만들고(create) → 확인하고(show) → 서명하고(sign) → 결합하고(combine) → 전송(broadcast)** 하는 도구.
서명과 인코딩은 체인 CLI(`gaiad` 등)가 처리하고, 이 도구는 서명자가 **무엇에 서명하는지 읽을 수 있게 보여주고, 기대 checksum과 대조**하는 일을 한다.

- Python 3.11+ 표준 라이브러리만 사용 (설치할 패키지 없음)
- 니모닉과 개인키는 다루지 않는다. 키는 체인 CLI 키링(또는 Ledger)에 있어야 한다.

## 서명자 (Signer)

받는 것: `<name>.tx.json`, `<name>.tx.meta.json`
그리고 **다른 경로로** 받는 것: wasm checksum (직접 빌드한 `artifacts/checksums.txt` 또는 PR/릴리스에 공개된 값)

```bash
python3 scripts/multisig/msig.py sign $RUN/store_skip_go_entry_point.tx.json \
  --from <내 키 이름> [--ledger] \
  --expect-multisig <multisig 주소> \
  --expect-checksum <wasm sha256>
```

- tx 내용이 출력된다. `wasm sha256`, `sender`, `admin`, `msg`를 확인하고 `y`를 입력한다.
- tx 안의 wasm 해시가 `--expect-checksum`과 다르면 **서명이 거부**된다.
- meta.json의 multisig가 `--expect-multisig`(또는 `MSIG_MULTISIG`)와 다르면 **서명이 거부**된다. multisig 주소는 코디네이터가 보낸 파일이 아니라 미리 알고 있는 값을 넣는다.
- 결과로 `<name>.tx.sig.<키 이름>.json`이 생성된다. 이 파일만 코디네이터에게 보낸다.
- instantiate tx에 `--online`을 붙이면 code_id의 온체인 checksum과 예측 주소가 함께 표시된다.

서명 없이 내용만 확인하려면 `msig.py show <tx> [--expect-checksum ..] [--online]`을 실행한다.

## 실행 폴더 (runs/)

배포 1회 = 폴더 1개: `scripts/multisig/runs/<날짜>-<체인>-<목적>/` (예: `2026-11-20-cosmoshub-eureka`).
`create`가 이 폴더에 tx를 만들고, `sign`/`combine`/`broadcast`는 입력 tx 옆에 결과를 쓰므로 한 배포의 파일이 모두 한곳에 모인다.
폴더째 git으로 주고받는다. `*.tx.signed.json`만 gitignore(재생성 가능, 재전송 방지).

```bash
export MSIG_WORK_DIR=scripts/multisig/runs/2026-11-20-cosmoshub-eureka
RUN=$MSIG_WORK_DIR
```

1. 코디네이터: `create` → `*.tx.json`, `*.tx.meta.json` 커밋·push
2. 서명자: pull → `sign` → `*.tx.sig.<키>.json` 커밋·push (서명자별 파일이라 충돌 없음)
3. 코디네이터: pull → `combine` → `broadcast` → `*.tx.result.json` 커밋

## 코디네이터 (Coordinator)

```bash
export MSIG_MULTISIG=<multisig 주소 또는 키링의 multisig 키 이름>
M="python3 scripts/multisig/msig.py"

# 1) store (wasm 1개 = tx 1건). 연속 tx는 --sequence로 미리 생성
$M create cosmoshub store artifacts/skip_go_ibc_adapter_ibc_callbacks.wasm
$M create cosmoshub store artifacts/skip_go_entry_point.wasm --sequence <S+1>

# 2) 서명 수집 후 결합 (서명 검증 포함) → 전송
$M combine $RUN/store_skip_go_entry_point.tx.json $RUN/store_skip_go_entry_point.tx.sig.*.json --multisig-key <ms key>
$M broadcast $RUN/store_skip_go_entry_point.tx.signed.json        # code_id 출력

# 3) instantiate2: 주소 예측 → tx 2개 생성 → 1건으로 병합
$M address cosmoshub --checksum <EP_SUM> --salt skip-eureka-ep-v3
$M address cosmoshub --checksum <CB_SUM> --salt skip-eureka-cb-v3
$M create cosmoshub instantiate2 --code-id <CB_ID> --salt skip-eureka-cb-v3 --admin multisig \
   --label "Skip Go IBC Callbacks Adapter" --msg '{"entry_point_contract_address":"<EP_ADDR>"}' -o $RUN/i_cb.tx.json
$M create cosmoshub instantiate2 --code-id <EP_ID> --salt skip-eureka-ep-v3 --admin multisig \
   --label "Skip Go Entry Point" --msg '{"swap_venues":[],"ibc_transfer_contract_address":"<CB_ADDR>"}' -o $RUN/i_ep.tx.json
$M merge $RUN/i_cb.tx.json $RUN/i_ep.tx.json -o $RUN/instantiate.tx.json
# ... sign / combine ...
$M broadcast $RUN/instantiate.tx.signed.json --expect-address <CB_ADDR> --expect-address <EP_ADDR>

# 기타 tx: `--` 뒤 인자를 `<bin> tx`에 그대로 전달
$M create cosmoshub tx -- wasm migrate <contract> <code_id> '{}'
```

- `create`와 `combine`은 로컬 CLI의 cosmos-sdk, `x/tx`, wasmd 버전이 체인 노드와 **정확히 같은지** 확인한다(다르면 중단). 서명자는 같은 minor 라인이면 된다. 버전이 어긋난 서명은 `combine`의 서명 검증에서 걸린다.
- `create`가 출력하는 tx 파일 sha256과 wasm checksum은 tx 파일과 **다른 채널**로 공지한다.

### 옵션과 환경변수
| | |
|---|---|
| `MSIG_MULTISIG` | multisig 주소 또는 키 이름 (`--multisig`) |
| `MSIG_MULTISIG_KEY` | combine에 쓸 multisig 키 이름 (`--multisig-key`) |
| `MSIG_KEYRING_BACKEND`, `MSIG_HOME` | CLI 키링 설정 (`--keyring-backend`, `--home`) |
| `MSIG_BIN` | 체인 CLI 경로 덮어쓰기 |
| `MSIG_WORK_DIR` | `create` 산출물 위치 (`--work-dir`). 기본 `scripts/multisig/work/`(gitignore, 개인 실습용) |

### Exit code
`0` 성공 · `1` 사용법/환경 · `2` 검증 실패/거부 · `3` CLI 실패 · `4` 체인 오류

## 체인 CLI 설치 (gaiad 예시)

체인 노드와 같은 버전을 소스에서 빌드한다. 2026-11 기준 macOS에서는 공식 darwin-amd64 릴리스 바이너리가 실행되지 않는다(`dyld: __DATA_CONST segment missing SG_READ_ONLY flag`).

```bash
git clone --depth 1 --branch v29.0.1 https://github.com/cosmos/gaia.git && cd gaia
GOTOOLCHAIN=auto make install      # go.mod가 요구하는 Go 버전을 자동으로 받는다
gaiad version --long
```

## 체인 추가

`chains/<chain>.toml`을 추가한다(`chains/cosmoshub.toml` 참고). 그 전에 다음을 확인한다.
- [ ] CLI 이름(`bin`), chain_id, bech32 prefix, fee denom, 엔드포인트(`https://[rpc|lcd]-<chain>.mainnet.cosmoslabs.kr`)
- [ ] gas price 정책: feemarket 조회 경로가 있으면 `query`, 없으면 `fixed`
- [ ] 키 타입이 `secp256k1`인지 (injective, evmos 계열은 별도 검토)
- [ ] code upload 권한 (`/cosmwasm/wasm/v1/codes/params`), instantiate2 지원 여부
- [ ] `msig show`로 store와 instantiate2 출력이 정상인지

## 테스트
```bash
python3 -m unittest discover -s scripts/multisig/tests -v
```
