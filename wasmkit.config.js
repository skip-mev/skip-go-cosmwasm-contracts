
const juno_testnet_accounts = [
  {
    name: 'account_0',
    address: 'juno1evpfprq0mre5n0zysj6cf74xl6psk96gus7dp5',
    mnemonic: 'omit sphere nurse rib tribe suffer web account catch brain hybrid zero act gold coral shell voyage matter nose stick crucial fog judge text'
  },
  {
    name: 'account_1',
    address: 'juno1njamu5g4n0vahggrxn4ma2s4vws5x4w3u64z8h',
    mnemonic: 'student prison fresh dwarf ecology birth govern river tissue wreck hope autumn basic trust divert dismiss buzz play pistol focus long armed flag bicycle'
  }
];

const neutron_testnet_accounts = [
  {
    name: 'account_0',
    address: 'neutron1jtdje5vq42sknl22r4wu9sahryu5wcrdqsccjh',
    mnemonic: 'category fine rapid trumpet dune early wish under nothing dance property wreck'
  },
];

const archway_testnet_accounts = [
  {
    name: 'account_0',
    address: 'archway1jtdje5vq42sknl22r4wu9sahryu5wcrd3yd7z8',
    mnemonic: 'category fine rapid trumpet dune early wish under nothing dance property wreck'
  },
];

const osmosis_testnet_accounts = [
  {
    name: 'account_0',
    address: 'osmosis1jtdje5vq42sknl22r4wu9sahryu5wcrdztt62s',
    mnemonic: 'category fine rapid trumpet dune early wish under nothing dance property wreck'
  },
];

const neutron_localnet_accounts = [
  {
    name: 'account_0',
    address: 'neutron1m9l358xunhhwds0568za49mzhvuxx9ux8xafx2',
    mnemonic: 'banner spread envelope side kite person disagree path silver will brother under couch edit food venture squirrel civil budget number acquire point work mass'
  },
  {
    name: 'account_1',
    address: 'neutron10h9stc5v6ntgeygf5xf945njqq5h32r54rf7kf',
    mnemonic: 'veteran try aware erosion drink dance decade comic dawn museum release episode original list ability owner size tuition surface ceiling depth seminar capable only'
  },
  {
    name: 'account_2',
    address: 'neutron14xcrdjwwxtf9zr7dvaa97wy056se6r5erln9pf',
    mnemonic: 'obscure canal because tomorrow tribe sibling describe satoshi kiwi upgrade bless empty math trend erosion oblige donate label birth chronic hazard ensure wreck shine'
  }
];

const juno_localnet_accounts = [
  {
    name: 'account_0',
    address: 'juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y',
    mnemonic: 'clip hire initial neck maid actor venue client foam budget lock catalog sweet steak waste crater broccoli pipe steak sister coyote moment obvious choose'
  },
];

const juno_mainnet_accounts = [
];
const neutron_mainnet_accounts = [
];
const osmosis_mainnet_accounts = [
];

// Default list covers most of the supported network
// Networks which are not required can be removed from here
const networks = {
  neutron_localnet: {
    endpoint: 'http://localhost:26657/',
    chainId: 'testing-1',
    accounts: neutron_localnet_accounts,
    fees: {
      upload: {
        amount: [{ amount: "750000", denom: "untrn" }],
        gas: "3000000",
      },
      init: {
        amount: [{ amount: "250000", denom: "untrn" }],
        gas: "1000000",
      },
      exec: {
        amount: [{ amount: "250000", denom: "untrn" }],
        gas: "1000000",
      }
    },
  },
  juno_localnet: {
    endpoint: 'http://localhost:26657/',
    chainId: 'testing-1',
    accounts: juno_localnet_accounts,
    fees: {
      upload: {
        amount: [{ amount: "750000", denom: "ujunox" }],
        gas: "3000000",
      },
      init: {
        amount: [{ amount: "250000", denom: "ujunox" }],
        gas: "1000000",
      },
      exec: {
        amount: [{ amount: "250000", denom: "ujunox" }],
        gas: "1000000",
      }
    },
  },
  juno_testnet: {
    endpoint: 'https://rpc.uni.junonetwork.io/',
    chainId: 'uni-6',
    accounts: juno_testnet_accounts,
    fees: {
      upload: {
        amount: [{ amount: "750000", denom: "ujunox" }],
        gas: "3000000",
      },
      init: {
        amount: [{ amount: "250000", denom: "ujunox" }],
        gas: "1000000",
      },
      exec: {
        amount: [{ amount: "250000", denom: "ujunox" }],
        gas: "1000000",
      }
    },
  },
  juno_mainnet: {
    endpoint: 'https://juno-rpc.polkachu.com/',
    chainId: 'juno-1',
    accounts: juno_mainnet_accounts,
    fees: {
      upload: {
        amount: [{ amount: "750000", denom: "ujuno" }],
        gas: "3000000",
      },
      init: {
        amount: [{ amount: "250000", denom: "ujuno" }],
        gas: "1000000",
      },
      exec: {
        amount: [{ amount: "250000", denom: "ujuno" }],
        gas: "1000000",
      }
    },
  },
  neutron_testnet: {
    endpoint: 'https://rpc-palvus.pion-1.ntrn.tech/',
    chainId: 'pion-1',
    accounts: neutron_testnet_accounts,
    fees: {
      upload: {
        amount: [{ amount: "750000", denom: "untrn" }],
        gas: "3000000",
      },
      init: {
        amount: [{ amount: "250000", denom: "untrn" }],
        gas: "1000000",
      },
      exec: {
        amount: [{ amount: "250000", denom: "untrn" }],
        gas: "1000000",
      }
    },
  },
  neutron_mainnet: {
    endpoint: 'https://rpc-kralum.neutron-1.neutron.org',
    chainId: 'neutron-1',
    accounts: neutron_mainnet_accounts,
    fees: {
      upload: {
        amount: [{ amount: "750000", denom: "untrn" }],
        gas: "3000000",
      },
      init: {
        amount: [{ amount: "250000", denom: "untrn" }],
        gas: "1000000",
      },
      exec: {
        amount: [{ amount: "250000", denom: "untrn" }],
        gas: "1000000",
      }
    },
  },
  archway_testnet: {
    endpoint: 'https://rpc.constantine-2.archway.tech',
    chainId: 'constantine-2',
    accounts: archway_testnet_accounts,
    fees: {
      upload: {
        amount: [{ amount: "750000", denom: "uconst" }],
        gas: "3000000",
      },
      init: {
        amount: [{ amount: "250000", denom: "uconst" }],
        gas: "1000000",
      },
      exec: {
        amount: [{ amount: "250000", denom: "uconst" }],
        gas: "1000000",
      }
    },
  },
  osmosis_testnet: {
    endpoint: 'https://rpc.testnet.osmosis.zone/',
    chainId: 'osmo-test-4',
    accounts: osmosis_testnet_accounts,
    fees: {
      upload: {
        amount: [{ amount: "750000", denom: "uosmo" }],
        gas: "3000000",
      },
      init: {
        amount: [{ amount: "250000", denom: "uosmo" }],
        gas: "1000000",
      },
      exec: {
        amount: [{ amount: "250000", denom: "uosmo" }],
        gas: "1000000",
      }
    },
  },
  osmosis_mainnet: {
    endpoint: 'https://rpc.osmosis.zone/',
    chainId: 'osmosis-1',
    accounts: osmosis_mainnet_accounts,
    fees: {
      upload: {
        amount: [{ amount: "750000", denom: "uosmo" }],
        gas: "3000000",
      },
      init: {
        amount: [{ amount: "250000", denom: "uosmo" }],
        gas: "1000000",
      },
      exec: {
        amount: [{ amount: "250000", denom: "uosmo" }],
        gas: "1000000",
      }
    },
  }
};

module.exports = {
  networks: {
    default: networks.neutron_testnet,
    testnet: networks.neutron_testnet,
    localnet: networks.juno_localnet,
    mainnet: networks.neutron_mainnet,
  },
   
  localnetworks: {
    juno: {
      docker_image: "uditgulati0/juno-node",
      rpc_port: 26657,
      rest_port: 1317,
      flags: ["GAS_LIMIT=10000000", "STAKE_TOKEN=ujunox", "TIMEOUT_COMMIT=5s"],
      docker_command: "./setup_and_run.sh juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y",
    },
    neutron: {
      docker_image: "uditgulati0/neutron-node",
      rpc_port: 26657,
      rest_port: 1317,
      flags: ["RUN_BACKGROUND=0"],
    },
    osmosis: {
      docker_image: "uditgulati0/osmosis-node",
      rpc_port: 26657,
      rest_port: 1317,
      flags: [],
      docker_command: "/osmosis/setup.sh",
    },
  },
  mocha: {
    timeout: 60000
  },
  rust: {
    version: "1.71.0",
  },
  commands: {
    compile: "RUSTFLAGS='-C link-arg=-s' cargo build --lib --release --target wasm32-unknown-unknown",
    schema: "cargo run --example schema",
  },
};
