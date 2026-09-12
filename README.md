# audiorecorder

Registratore audio desktop per Linux: cattura da un **microfono** oppure
dall'**audio di sistema** (ciò che senti dalle casse/cuffie) e salva in **WAV
16-bit**.

Interfaccia grafica con `egui`/`eframe`, cattura tramite l'API PulseAudio, che su
PipeWire funziona grazie a `pipewire-pulse`.

## Caratteristiche

- Selezione della sorgente tra microfoni e sorgenti *monitor* (audio di sistema)
- Scelta della cartella di output con dialog nativo
- Record / Pausa / Riprendi / Stop
- VU meter in tempo reale (picco in dBFS)
- Timer di durata
- Gestione errori con messaggi in interfaccia

## Requisiti

- Linux con **PipeWire** (o PulseAudio) e il socket PulseAudio attivo
- Rust (edition 2021) — testato con `rustc` 1.97
- A runtime servono le librerie `libpulse.so.0` e `libpulse-simple.so.0`
  (normalmente già presenti; NON serve `libpulse-dev` per compilare, perché i
  binding Rust usano un fallback di linking diretto)

## Build ed esecuzione

```bash
cargo build --release
cargo run --release
```

## Uso

1. Scegli la sorgente dal menu a tendina. Le voci `[microfono]` sono ingressi
   fisici; le voci `[sistema]` sono i monitor delle uscite (audio del PC).
2. Facoltativamente cambia la cartella di output con **Scegli…**.
3. Premi **● Registra**, poi **⏸ Pausa** / **▶ Riprendi** e **■ Stop**.
4. I file vengono salvati come `rec-<data>_<ora>.wav`.

> Nota: per registrare l'audio di sistema devi scegliere una sorgente
> `[sistema]`. Il monitor di un'uscita sospesa (nessun audio in riproduzione)
> può non produrre dati finché non avvii la riproduzione.

## Quality check

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Strumenti di debug

Elenca le sorgenti disponibili senza aprire la GUI:

```bash
cargo run --example list_sources
```

## Struttura

```
src/
  main.rs               # bootstrap eframe
  app.rs                # stato e interfaccia egui
  audio/
    devices.rs          # enumerazione sorgenti PulseAudio
    recorder.rs         # thread di cattura, scrittura WAV, pausa/stop
    meter.rs            # calcolo picco/RMS e smoothing per il VU meter
examples/
  list_sources.rs       # elenco sorgenti da terminale
docs/
  spec.md               # specifica
  plan.md               # piano di implementazione
```
