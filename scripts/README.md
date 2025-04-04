# Scripts for running test/dev net

## General
- all params inside ```.sh``` files must be changed for your paths
- all scripts has a relative paths and configured for running from this path where README is

## Stage 1
### Prepairing
Run ```prepare-<..>.sh``` for prepare the target type of chain

## Stage 2
### Run proccess
1. Run the full node with ```..-full.sh```
2. Run the collator or validator nodes with ```..-collator..sh``` or ```..validator..sh```

If you run the parachain with local relaychain - first run the relaychain with ```start-relay-full.sh``` for local relaychain.

