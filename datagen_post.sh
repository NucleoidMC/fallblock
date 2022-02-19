#!/bin/bash

cat generated/reports/registries.json | jq '."minecraft:block_entity_type".entries | with_entries({key, value: .value.protocol_id})' > src/world/block_entities.json
cat generated/reports/blocks.json | jq > src/world/blocks.json
