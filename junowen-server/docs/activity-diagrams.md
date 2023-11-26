# activity-diagrams

The room is like a mailbox.

- Polling interval: 3 sec
- Room expires: 10 sec

## Why shared/reserved ?

- リザーブドルームが作成されることで共用ルームが制限されるべきではない
- リザーブドルームは共用ルームよりも維持費がかかるので共用ルームよりも選択しにくくあるべきだ

----

```mermaid
---
title: fn find_valid_room() -> Option<Room>
---

flowchart TB

DB[(DynamoDB)]
0000((S))
0000 --> 0010("Find Room(Offer)")
0010 -- Found? --> 0020{ }
0010 -.-> DB
  0020 -- NO --> 0022("return None")
  0022 --> 0024((E))
0020 -- YES<br><br>Expired? --> 0120{ }
  0120 -- NO --> 0140("return Some")
  0140 --> 0142((E))
0120 -- YES --> 0130("Remove Room(Offer) and Answer")
0130 --> 0055("return None")
0055 --> 0057((E))

0130 -.-> DB
```

```mermaid
flowchart TB
  DB[(DynamoDB)]
  0065 -.-> DB

  subgraph 0000 ["PUT /(custom|reserved-room)/{room_name}"]
    direction TB
    0001((S))
    0001 --> 0005{ }
    0005 --> 0010("fn find_valid_room()")
    0010 -.-> DB
    0010 -- Some? --> 0015{ }
      0015 -- YES --> 0020("409 Conflict<br>{ offer: Sdp } if shared-room<br>{ offer: Option < Sdp > } if reserved-room")
      0020 --> 0125((E))
      %% return
    0015 -- NO --> 0050("Store Room(Offer) (may include Spectator Offer)")
    0050 -.-> DB
    0050 -- Conflict? --> 0055{ }

    0055 -- YES --> 0005

    0055 --> 0065(Remove Answer)

    0100 -.-> DB
    subgraph "fn find_oppnent()"
      0065 -- Some? --> 0070{ }
        0070 -- YES<br><br>Reserve? --> 0072{ }
          0072 -- NO --> 0100("Remove Room(Offer)")
          0100 --> 0079{ }
          %% goto
        %% else
          0072 -- YES --> 0074("Remove Offer in Room")
          0074 --> 0079{ }
          %% goto
      %% else
        %% goto
    end
    0079 --> 0110("201 Created<br>{ answer: Sdp }")
    0110 --> 0998{ }

    0070 -- NO --> 0080("201 Created<br>{ key: String }")
    0080 --> 0998

    0998 --> 0999((E))
  end
```

```mermaid
flowchart TB
  DB[(DynamoDB)]

  subgraph 3000 ["POST /(custom|reserved-room)/{room_name}/keep"]
    direction TB
    3001((S))
    3001 --> 3010("Keep offer (may include Spectator Offer)")
    3010 -.-> DB
    3010 -- Succeeded? --> 3020{ }
      3020 -- NO --> 3030(Bad request)
      3030 --> 3031((E))
    3020 -- YES<br><br>Has opponent offer? --> 3040{ }
      3040 -- Some --> 3065("fn find_oppnent()")
      3065 -.-> DB
      3065 -- Some? --> 3070{ }
        3070 -- YES --> 3090(OK with Opponent Answer)
        3090 --> 3999((E))
        %% return
      %% else
        3070 -- NO --> 3080(No content)
        3080 --> 3089((E))
        %% return
    %% else
      3040 -- None<br><br>Has spectator offer? --> 3097{ }
      3097 -- Some --> 3100("fn find_spectator()")
      3100 -.-> DB
      3100 -- Some? --> 3170{ }
        3170 -- YES --> 3110(OK with Spectator Answer)
        3110 --> 3119((E))
      %% else
        3170 -- NO --> 3180(No content)
        3180 --> 3189((E))
    %% else
      3097 -- None --> 3280(No content)
    3280 --> 3289((E))
  end
```

```mermaid
flowchart TB
  DB[(DynamoDB)]

  subgraph "POST /(custom|reserved-room)/{room_name}/(join|spectate)"
    direction TB
    1001((S))
    1001 --> 1010(Store Answer)
    1010 -.-> DB
    1010 -- Conflict? --> 1020{ }

    1020 -- YES --> 1030(Send Conflict)
    1030 --> 1998{ }

    1020 -- NO --> 1100(Send Created)
    1100 --> 1998

    1998 --> 1999((E))
  end
```

```mermaid
flowchart TB
  DB[(DynamoDB)]

  subgraph "DELETE /(custom|reserved-room)/{room_name}"
    direction TB
    4001((S))
    4001 --> 4010(Delete Offer with key)
    4010 -.-> DB
    4010 -- Succeeded? --> 4020{ }

    4020 -- YES --> 4030(Delete Answer)
    4030 -.-> DB
    4030 --> 4040(No content)
    4040 --> 4998

    4020 -- NO --> 4100(Bad request)
    4100 --> 4998

    4998 --> 4999((E))
  end
```

```mermaid
---
title: Waiting for opponent
---

flowchart TB
  direction TB

  2000((S))
  2000 --> 2020("PUT /(custom|reserved-room)/{room_name}")
  2020 -- Which? --> 2030{ }
    2030 -- Offer --> 2040(Generate Answer)
    2040 --> 2060("POST /(custom|reserved-room)/{room_name}/join")
    2060 -- Which? --> 2070{ }
      2070 -- Room is full --> 2998
      %% goto
    %% case
      2070 -- OK --> 2080(Connect as Guest)
      2080 --> 2998{ }
      %% goto
  %% case
    2030 -- Answer --> 2200{ }
    2200 --> 2210(Connect as Host)
    2210 --> 2998
    %% goto
  %% case
    2030 -- Room key --> 2300{ }
    2300 --> 2320("POST /(custom|reserved-room)/{room_name}/keep")
    2320 -- Found Answer? --> 2330{ }
      2330 -- YES --> 2200
      %% goto
    %% case
      2330 -- NO --> 2300
      %% goto
  %% case
    2030 -- "Room is full (Reserved Room)" --> 2998

  2998 --> 2999((E))
```

```mermaid
---
title: Waiting for Spectator in Reserved Room
---

flowchart TB
  direction TB

  2000((S))
  2000 --> 2300{ }
  2300 --> 2320("POST /reserved-room/{room_name}/keep<br>(may include Spectator Offer)")
  2320 -- Found Answer? --> 2330{ }
    2330 -- NO --> 2300
    %% goto
  %% case
    2330 -- YES --> 2200(Connect for Spectator)
    2200 --> 2999((E))
```

```mermaid
---
title: Waiting for Spectator Host
---

flowchart TB
  direction TB

  0000((S))
  0000 --> 1000{ }
  1000 --> 2020("GET /(custom|reserved-room)/{room_name}")
  2020 -- Which? --> 2030{ }
    2030 -- 404 --> 2031(Room not found)
    2031 --> 2998{ }
    %% goto
  %% case
    2030 -- 200<br><br>State? --> 2032{ }
      2032 -- Waiting --> 2034(Match hasn't started)
      2034 --> 2998{ }
      %% goto
    %% case
      2032 -- Playing<br><br>Has Spectator Offer? --> 2036{ }
        2036 -- YES --> 2040(Generate Answer)
      %% else
        2036 -- NO --> 1000
        %% goto
    2040 --> 2060("POST /(custom|reserved-room)/{room_name}/spectate")
    2060 -- Which? --> 2070{ }
      2070 -- Conflict --> 1000
      %% goto
    %% case
      2070 -- OK --> 2080(Connect as Spectator)
      2080 --> 2998{ }
      %% goto

  2998 --> 2999((E))
```
