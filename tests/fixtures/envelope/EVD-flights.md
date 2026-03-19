---
id: "EVD-flights"
type: evidence
name: "Flight Records"
sourcing: "documentary"
participants:
  - entity: "ITM-alice"
    role: "passenger"
  - entity: "ITM-bob"
    role: "passenger"
  - entity: "ITM-loc-a"
    role: "location"
  - entity: "ITM-loc-b"
    role: "location"
flights:
  - id: 1
    date: "2024-02-10"
    origin: "ITM-loc-a"
    destination: "ITM-loc-b"
    passengers:
      - "ITM-alice"
      - "ITM-bob"
  - id: 2
    date: "2024-02-15"
    origin: "ITM-loc-b"
    destination: "ITM-loc-a"
    passengers:
      - "ITM-alice"
---

Flight records showing travel between Location A and Location B.
