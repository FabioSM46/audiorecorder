# Implementation Plan: audiorecorder

## Overview

App egui che registra da microfono o audio di sistema (sorgenti monitor
PipeWire/PulseAudio) su thread dedicato, scrivendo WAV 16-bit. La UI resta
reattiva: riceve il livello del VU meter via atomics e gli errori via canale.

## Architecture Decisions

- **PulseAudio API invece di cpal/ALSA**: ALSA non enumera i monitor. Vedi spec.
- **Thread di capture separato**: `libpulse-simple-binding` è bloccante; isolarlo
  evita di congelare il render loop di egui.
- **Comunicazione lock-free**: `Arc<AtomicU32>` (peak level, f32 bits) e
  `Arc<AtomicBool>` (pause/stop). Canale `mpsc` solo per gli errori.
- **WAV 16-bit** con `hound`; la conversione i16↔f32 per il meter è locale.
- **Stream aperto in pausa**: in pausa si continua a drenare la sorgente ma non
  si scrive, evitando overflow del buffer PulseAudio.

## Dependency Graph

```
Cargo scaffold
   ├── meter (puro, testabile)         [indipendente]
   ├── devices (enumerazione Pulse) ──┐
   └── recorder (capture + WAV) ──────┤
                                      └──> app (UI egui) ──> main
```

## Task List

### Phase 1: Foundation

- [x] **Task 1 — Scaffold Cargo + eframe minimo**
  - Acceptance: `cargo build` compila; finestra vuota "audiorecorder".
  - Verify: `cargo build`, `cargo run` (finestra).
  - Files: `Cargo.toml`, `.gitignore`, `src/main.rs`.
  - Scope: S

- [x] **Task 2 — meter.rs (TDD)**
  - Acceptance: `peak`/`rms` corretti su buffer noti; `SmoothedMeter` attacco
    immediato e rilascio decadente.
  - Verify: `cargo test meter`.
  - Files: `src/audio/mod.rs`, `src/audio/meter.rs`.
  - Scope: S

- [x] **Task 3 — devices.rs**
  - Acceptance: `list_sources()` ritorna nome + descrizione di tutte le sorgenti,
    inclusi i monitor, con flag `is_default`/`is_monitor`.
  - Verify: `cargo test` (compila) + check manuale via `cargo run` log.
  - Files: `src/audio/devices.rs`.
  - Scope: S

### Checkpoint A: Foundation
- [x] `cargo build` e `cargo test` puliti
- [x] enumerazione sorgenti verificata su questa macchina

### Phase 2: Core

- [x] **Task 4 — recorder.rs**
  - Acceptance: `Recorder::start(device, path)` apre lo stream e scrive un WAV
    valido; `pause/resume/stop` funzionano; il level atomico si aggiorna.
  - Verify: test su `WavWriter` (round-trip hound) + `cargo build`.
  - Files: `src/audio/recorder.rs`.
  - Scope: M

- [x] **Task 5 — app.rs + wiring UI**
  - Acceptance: dropdown device (con refresh), selezione cartella output con
    dialog nativo, Record/Pausa/Stop, timer, VU meter, messaggio d'errore; la UI
    non si blocca durante la registrazione.
  - Verify: `cargo run` + checklist manuale (mic e monitor).
  - Files: `src/app.rs`, `src/main.rs`.
  - Scope: M

### Checkpoint B: Core
- [x] Registrazione da microfono produce WAV corretto (durata reale verificata)
- [ ] Registrazione da monitor: WAV valido prodotto; contenuto audio da confermare manualmente (dipende dall'audio in riproduzione)
- [x] Pausa/ripresa/stop corretti

### Phase 3: Polish

- [x] **Task 6 — README + errori + cartella output**
  - Acceptance: `./recordings/` auto-creata; README con dipendenze e uso.
  - Verify: `cargo clippy --all-targets -- -D warnings`, `cargo test`.
  - Files: `README.md`, `src/audio/recorder.rs`.
  - Scope: S

### Checkpoint C: Complete
- [x] `cargo test` verde, `cargo clippy -- -D warnings` pulito
- [x] Tutti i criteri di successo della spec soddisfatti

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| API `libpulse-binding` diversa dalle attese | High | Compilare presto piccoli esempi; consultare docs.rs |
| `libpulse-simple-binding` non thread-safe come atteso | Med | Lo stream nasce e muore dentro il thread di capture |
| Nessuna sorgente monitor esposta | Med | Fallback alla sorgente default + messaggio chiaro |
| `libpulse-dev` assente (build fallisce) | High | Installare prima; documentare in README |

## Open Questions

- Nessuna: bit depth (16), gain (nessuno) e cartella output sono stati decisi.
