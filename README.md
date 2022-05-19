# Commands

Use `./build.sh` to build
[optional] if you want to deploy a new dev contract, delele neardev/ folder.
Then use `source .dev_deploy.sh` to deploy dev contract

Example intialization `near call $CONTRACT_NAME new '{"owner_id": "katesona6.testnet", "total_supply": "10000000000000"}' --accountId $CONTRACT_NAME` to initiate the dev contract.
