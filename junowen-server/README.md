# junowen-server

## create

```sh
cargo lambda deploy \
  --binary-name junowen-server \
  --enable-function-url \
  --env-var ENV=prod \
  --profile $PROFILE \
  junowen-server
```

## Dynamo DB definition

* env = dev | prod
* table_name = Offer | Answer | ReservedRoom | ReservedRoomOpponentAnswer | ReservedRoomSpectatorAnswer

### {env}.{table_name}

* Partition Key = { name: String }
* Capacity mode = ondemand
* delete protection
* TTL = ttl_sec
