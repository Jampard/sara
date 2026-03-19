---
id: "EVD-bad-participants"
type: evidence
name: "Bad Participants Evidence"
sourcing: "forensic"
participants:
  - entity: "ITM-alice"
    role: "sender"
messages:
  - id: 1
    from: "ITM-alice"
    to:
      - "ITM-bob"
    date: "2024-01-15"
---

Evidence where ITM-bob appears in messages but not in participants.
