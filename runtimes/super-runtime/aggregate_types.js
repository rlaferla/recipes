// Reads in the type definitions from all pallets in the runtime and the runtime's own tpes
// Naively aggregates types and writes them to disk.

const fs = require('fs');

// A list of all the installed recipe pallets' folder names.
// Does not include system pallets because Apps already supports them.
// Redundant with construct_runtime!
const pallets = [
  "adding-machine",
  "basic-token",
  "charity",
  "check-membership",
  "compounding-interest",
  "constant-config",
  "default-instance",
  "double-map",
  "execution-schedule",
  "fixed-point",
  "generic-event",
  "hello-substrate",
  "last-caller",
  "offchain-demo",
  "randomness",
  "ringbuffer-queue",
  "randomness",
  "simple-event",
  "simple-map",
  "single-value",
  "storage-cache",
  "struct-storage",
  "sum_storage",
  "vec-set",
]

// Types that are native to the runtime itself (ie come from lib.rs)
// These specifics are from https://polkadot.js.org/api/start/types.extend.html#impact-on-extrinsics
const runtimeOwnTypes = {
  "Address": "AccountId",
  "LookupSource": "AccountId",
  "Weight": "u32"
}

// Loop through all pallets aggregating types
let finalTypes = runtimeOwnTypes;
let palletTypes;
for (let dirname of pallets) {
  let path = `../../pallets/${dirname}/types.json`;
  palletTypes = JSON.parse(fs.readFileSync(path, 'utf8'));
  finalTypes = {...finalTypes, ...palletTypes};
}

// Write output to disk
fs.writeFileSync("types.json", JSON.stringify(finalTypes, null, 2), 'utf8');
