# Spec: audiorecorder

## Objective

App desktop Linux per registrare audio da una sorgente selezionabile:

- un **microfono** (o qualsiasi ingresso), oppure
- l'**audio di sistema** (ciò che si sente dalle casse/cuffie), catturato dalla
  sorgente *monitor* di PipeWire/PulseAudio.

L'utente sceglie il device da una lista, sceglie la **cartella di output**
tramite dialog nativo, registra con controlli record/pausa/riprendi/stop, vede un
VU meter in tempo reale e ottiene un file **WAV** riproducibile.

**Successo** = un utente può, senza terminale, selezionare "Monitor of ...",
premere Record, riprodurre musica, premere Stop e ritrovare un WAV che contiene
esattamente l'audio riprodotto; e ripetere la stessa cosa con il microfono.

## Tech Stack

- **Linguaggio:** Rust (edition 2021)
- **GUI:** `eframe` / `egui` 0.36 (immediate-mode, single binary)
- **Capture audio:** API PulseAudio
  - `libpulse-binding` 2.30 → enumerazione sorgenti (inclusi i monitor)
  - `libpulse-simple-binding` 2.29 → stream di registrazione bloccante su thread dedicato
- **Scrittura WAV:** `hound` 3.5 (PCM 16-bit)
- **Selezione cartella:** `rfd` 0.17 (dialog nativo via XDG Desktop Portal, nessuna dipendenza GTK)
- **Errori:** `anyhow` 1
- **Timestamp nomi file:** `chrono` 0.4

### Perché PulseAudio e non cpal

`cpal` su Linux è un wrapper ALSA. ALSA (via `pipewire-alsa`) espone solo un PCM
`default`/`pulse` e **non enumera le sorgenti monitor**, quindi non permette di
scegliere "audio di sistema". L'API PulseAudio, esposta da `pipewire-pulse`
(attivo su questa macchina), enumera tutte le sorgenti — microfono *e* monitor —
con un'unica API. Il costo è una dipendenza di sistema di build (`libpulse-dev`).

## Commands

```bash
# Build
cargo build --release

# Esecuzione
cargo run --release

# Test
cargo test

# Lint (deve passare senza warning)
cargo clippy --all-targets -- -D warnings

# Formattazione
cargo fmt --check
```

Nota: non serve installare `libpulse-dev`. I binding Rust usano un fallback che
linka direttamente `libpulse.so.0` / `libpulse-simple.so.0` (presenti a runtime
su sistemi con PipeWire/PulseAudio).

## Project Structure

```
audiorecorder/
├── Cargo.toml
├── README.md
├── docs/
│   └── spec.md              # questo documento
└── src/
    ├── main.rs              # bootstrap eframe::run_native
    ├── app.rs               # struct App: stato + rendering egui
    └── audio/
        ├── mod.rs           # re-export
        ├── devices.rs       # enumerazione sorgenti PulseAudio
        ├── recorder.rs      # start/pausa/riprendi/stop + thread di capture + WAV
        └── meter.rs         # calcolo peak/RMS e smoothing per il VU meter
```

La cartella di output predefinita è `./recordings/`, creata automaticamente; i
file sono nominati `rec-YYYY-MM-DD_HH-MM-SS.wav`.

## Code Style

- Nessun `unsafe` (le crate PulseAudio forniscono wrapper safe).
- Errori propagati con `anyhow::Result`; niente `unwrap()` sul percorso utente.
- Unità del dominio esplicite: `SampleRate(u32)`, `Channels(u16)` dove utile.
- Nessun commento se non per decisioni non ovvie.
- Nomi: `snake_case` per funzioni/variabili, `PascalCase` per tipi.

Esempio di stile per la parte testabile (meter):

```rust
pub fn peak(samples: &[i16]) -> f32 {
    samples
        .iter()
        .map(|s| (*s as f32).abs() / i16::MAX as f32)
        .fold(0.0, f32::max)
}

pub fn rms(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
    (sum_sq / samples.len() as f64).sqrt() as f32 / i16::MAX as f32
}
```

## Testing Strategy

- **Framework:** test integrati in Rust (`cargo test`), moduli `#[cfg(test)]`.
- **Unit test (logica pura):**
  - `meter::peak` / `meter::rms` su buffer noti (silenzio, pieno scala, sinusoidi).
  - `SmoothedMeter`: attacco immediato, rilascio con decadimento.
  - Generazione del nome file con timestamp (formato deterministico).
  - Scrittura WAV: dato un vettore di campioni, scrivere con `hound` e rileggere
    verificando sample rate, canali, lunghezza e valori.
- **Non automatizzabile (richiede server audio):** enumerazione device e
  cattura reale → checklist manuale.
- **Checklist manuale (acceptance):**
  1. La lista device mostra microfono **e** voci "Monitor of ...".
  2. Registro 5s dal microfono → il WAV contiene la mia voce, durata ≈ 5s.
  3. Registro 5s dal monitor riproducendo musica → il WAV contiene la musica.
  4. Pausa/riprendi: la durata registrata esclude il tempo in pausa; niente click.
  5. Stop finalizza il file ed è subito riproducibile (`aplay file.wav`).
  6. Il VU meter reagisce all'audio e cala in silenzio.

## Boundaries

- **Always:**
  - Eseguire `cargo test` e `cargo clippy --all-targets -- -D warnings` prima di ogni commit.
  - Scrivere WAV standard PCM 16-bit, senza header proprietari.
  - Gestire gli errori del thread di capture e mostrarli nella UI.
- **Ask first:**
  - Installare pacchetti di sistema (`libpulse-dev`).
  - Aggiungere/rimuovere crate o cambiare formato di output (es. FLAC/MP3).
  - Introdurre `unsafe` o dipendenze PipeWire native.
- **Never:**
  - Modificare la configurazione di sistema PipeWire/PulseAudio.
  - Scrivere file fuori dalla cartella di output scelta.
  - Fare commit di segreti o artefatti binari.

## Success Criteria

1. `cargo build --release` e `cargo clippy --all-targets -- -D warnings` puliti.
2. `cargo test` verde (unit test su meter e WAV).
3. La lista device include le sorgenti monitor, selezionabili.
4. L'utente può scegliere la cartella di output con un dialog nativo; il file
   viene creato lì.
5. Registrazione da microfono e da monitor produce WAV corretti (checklist manuale).
6. Record/Pausa/Riprendi/Stop funzionano senza crash né perdita di sincronia.
7. VU meter aggiornato in tempo reale (≥ 20 fps) senza bloccare la UI.

## Open Questions

- Nessuna. Decisioni prese: WAV 16-bit PCM, nessun controllo di gain, selezione
  della cartella di output via dialog nativo (`rfd`).
