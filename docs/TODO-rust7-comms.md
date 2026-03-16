# PLC Button Integration Plan

## Goals
- Add an Nginx dashboard card with buttons that flip PLC bits and display feedback.
- Keep the implementation inside the existing Rust API service (Axum) for now.
- Define a deterministic Siemens S7 DB layout so the web UI, API service, and PLC logic stay in sync.

## Proposed Data Block Layout (DB100 example)
| Offset (byte.bit) | Size          | Direction | Meaning                                     |
|-------------------|---------------|-----------|---------------------------------------------|
| 0.0–1.7           | 2 words (32b) | Send      | `cmd_bits[0..31]` – momentary buttons (Start/Stop/Reset/etc.) written by API, read by PLC. Bit 0 = “Button 1”, bit 1 = “Button 2”, … |
| 4                 | 4 bytes       | Send      | `cmd_int0` (DINT) – generic command value (e.g., recipe ID). |
| 8                 | 4 bytes       | Send      | `cmd_int1` (DINT). |
| 12                | 4 bytes       | Send      | `cmd_int2` (DINT). |
| 16                | 4 bytes       | Send      | `cmd_int3` (DINT). |
| 20                | 4 bytes       | Send      | `cmd_real0` (REAL) – analog/reference value. |
| 24                | 4 bytes       | Send      | `cmd_real1` (REAL). |
| 28.0–29.7         | 2 words       | Receive   | `status_bits[0..31]` – PLC feedback flags (ack, faults, interlocks). |
| 32                | 4 bytes       | Receive   | `status_int0` (DINT) – e.g., active recipe. |
| 36                | 4 bytes       | Receive   | `status_int1` (DINT). |
| 40                | 4 bytes       | Receive   | `status_int2` (DINT). |
| 44                | 4 bytes       | Receive   | `status_int3` (DINT). |
| 48                | 4 bytes       | Receive   | `status_real0` (REAL) – measured value. |
| 52                | 4 bytes       | Receive   | `status_real1` (REAL). |

**Why this structure?**
- 2 command words (32 bools) comfortably cover typical dashboard actions while staying aligned to word boundaries. Doubling it on the receive side gives symmetry for feedback bits.
- Four DINTs and two REALs each way cover most “parameter + measurement” use cases without bloating the block (total < 60 bytes).
- Everything is word-aligned, which keeps S7 read/write calls simple and minimizes network payloads.

## API-Service Changes
1. **Config**: add PLC settings to `config.rs` (DB number, rack/slot, IP, scan interval, mapping of button labels → bit indices).
2. **State**: extend `AppState` with `PlcService` holding:
   - Async S7 client (rust7 or snap7 via tokio spawn).
   - `Mutex<Vec<bool>>` for `cmd_bits`, plus cached ints/reals.
3. **Routes**:
   - `GET /api/plc/state` → dumps entire structure (bits, ints, reals, timestamps).
   - `POST /api/plc/actions` → accepts `{ bits: [{ index, value }], ints: [...], reals: [...] }`, updates the mutex, writes DB, returns new state.
4. **PLC IO**:
   - Pack/unpack bools into the first 4 bytes (2 words) before calling `write_area`.
   - Batch all data in one `DBWrite` to keep the PLC handshake atomic.

## Nginx Frontend Updates
1. Add a “PLC Controls” card in `docker/config/nginx/html/index.html` with buttons mapped to bit indices.
2. Use existing fetch/notification helpers to call `/api/plc/actions`.
3. Poll `/api/plc/state` every few seconds to light up active buttons and render numeric values (the 4 ints/2 reals).

## Next Steps
1. Implement `plc.rs` module + wire routes.
2. Add env vars (`PLC_IP`, `PLC_DB`, etc.) to `docker/.env` and docker-compose.
3. Create UI section + basic styling for the buttons and numeric readouts.
4. Validate end-to-end with a Siemens S7 test DB to confirm byte ordering and word alignment.
