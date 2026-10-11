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

## CloudWatch access stats

```sh
cargo --bin access-stats -- iam-check \
  --profile $PROFILE \
  --function-name junowen-server
```

```sh
cargo --bin access-stats -- room-activity \
  --profile $PROFILE \
  --function-name junowen-server \
  --start 2026-06-01T00:00:00+09:00 \
  --end 2026-07-01T00:00:00+09:00
```

Minimum IAM policy:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "logs:StartQuery",
        "logs:GetQueryResults"
      ],
      "Resource": [
        "arn:aws:logs:ap-northeast-1:<account-id>:log-group:/aws/lambda/junowen-server:*"
      ]
    }
  ]
}
```
