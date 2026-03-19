---
id: "EVD-transactions"
type: evidence
name: "Financial Records"
sourcing: "documentary"
participants:
  - entity: "ITM-alice"
    role: "payer"
  - entity: "ITM-bob"
    role: "payee"
transactions:
  - id: 1
    date: "2024-03-01"
    from: "ITM-alice"
    to: "ITM-bob"
    amount: 5000.00
    currency: "USD"
    method: "wire"
  - id: 2
    date: "2024-03-15"
    from: "ITM-bob"
    to: "ITM-alice"
    amount: 2500.00
    currency: "USD"
---

Financial transaction records between Alice and Bob.
