---
id: "EVD-bad-multi"
type: evidence
name: "Multiple Envelope Types"
sourcing: "documentary"
participants:
  - entity: "ITM-alice"
    role: "sender"
  - entity: "ITM-bob"
    role: "recipient"
  - entity: "ITM-loc-a"
    role: "location"
  - entity: "ITM-loc-b"
    role: "location"
messages:
  - id: 1
    from: "ITM-alice"
    to:
      - "ITM-bob"
flights:
  - id: 1
    date: "2024-02-10"
    origin: "ITM-loc-a"
    destination: "ITM-loc-b"
    passengers:
      - "ITM-alice"
---

Evidence with both messages and flights (should fail mutual exclusivity).
