use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use libpulse_binding::def::BufferAttr;
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;

use crate::audio::meter::{peak, rms};

const BLOCK_FRAMES: usize = 1024;

pub struct Recorder {
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
    errors: Receiver<String>,
    spec: Spec,
}

struct Shared {
    paused: AtomicBool,
    stop: AtomicBool,
    peak_bits: AtomicU32,
    rms_bits: AtomicU32,
    frames: AtomicU64,
}

impl Recorder {
    pub fn start(device_name: &str, path: &Path, spec: Spec) -> Result<Self> {
        let shared = Arc::new(Shared {
            paused: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            peak_bits: AtomicU32::new(0),
            rms_bits: AtomicU32::new(0),
            frames: AtomicU64::new(0),
        });
        let (sender, errors) = mpsc::channel();

        let device = device_name.to_string();
        let path = path.to_path_buf();
        let capture_shared = Arc::clone(&shared);
        let handle = thread::Builder::new()
            .name("audio-capture".into())
            .spawn(move || {
                if let Err(error) = capture(&device, &path, spec, &capture_shared) {
                    let _ = sender.send(format!("{error:#}"));
                }
            })
            .context("impossibile avviare il thread di registrazione")?;

        Ok(Self {
            shared,
            handle: Some(handle),
            errors,
            spec,
        })
    }

    pub fn is_paused(&self) -> bool {
        self.shared.paused.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Relaxed);
    }

    pub fn peak(&self) -> f32 {
        f32::from_bits(self.shared.peak_bits.load(Ordering::Relaxed))
    }

    pub fn rms(&self) -> f32 {
        f32::from_bits(self.shared.rms_bits.load(Ordering::Relaxed))
    }

    pub fn duration(&self) -> Duration {
        let channels = self.spec.channels.max(1) as u64;
        let rate = self.spec.rate.max(1) as u64;
        let seconds = self.shared.frames.load(Ordering::Relaxed) as f64 / (rate * channels) as f64;
        Duration::from_secs_f64(seconds)
    }

    pub fn poll_error(&self) -> Option<String> {
        self.errors.try_recv().ok()
    }

    pub fn stop(mut self) -> Result<()> {
        self.shared.stop.store(true, Ordering::SeqCst);
        self.join();
        if let Ok(error) = self.errors.try_recv() {
            return Err(anyhow!(error));
        }
        Ok(())
    }

    fn join(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        self.join();
    }
}

fn capture(device: &str, path: &Path, spec: Spec, shared: &Shared) -> Result<()> {
    let bytes_per_frame = (spec.channels as usize).max(1) * std::mem::size_of::<i16>();
    let mut buffer = vec![0u8; BLOCK_FRAMES * bytes_per_frame];

    let stream_spec = Spec {
        format: Format::S16NE,
        channels: spec.channels,
        rate: spec.rate,
    };
    let buffer_attr = BufferAttr {
        maxlength: u32::MAX,
        tlength: u32::MAX,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize: (BLOCK_FRAMES * bytes_per_frame) as u32,
    };
    let simple = Simple::new(
        None,
        "audiorecorder",
        Direction::Record,
        Some(device),
        "Registrazione",
        &stream_spec,
        None,
        Some(&buffer_attr),
    )
    .map_err(|e| anyhow!("impossibile aprire la sorgente '{device}': {e}"))?;

    let wav_spec = hound::WavSpec {
        channels: spec.channels as u16,
        sample_rate: spec.rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, wav_spec)
        .with_context(|| format!("impossibile creare il file {}", path.display()))?;

    while !shared.stop.load(Ordering::SeqCst) {
        simple
            .read(&mut buffer)
            .map_err(|e| anyhow!("errore di lettura dalla sorgente: {e}"))?;

        let mut samples = Vec::with_capacity(buffer.len() / 2);
        for chunk in buffer.chunks_exact(2) {
            samples.push(i16::from_ne_bytes([chunk[0], chunk[1]]));
        }

        shared
            .peak_bits
            .store(peak(&samples).to_bits(), Ordering::Relaxed);
        shared
            .rms_bits
            .store(rms(&samples).to_bits(), Ordering::Relaxed);

        if !shared.paused.load(Ordering::Relaxed) {
            for sample in &samples {
                writer.write_sample(*sample)?;
            }
            shared
                .frames
                .fetch_add(samples.len() as u64, Ordering::Relaxed);
        }
    }

    writer
        .finalize()
        .context("impossibile finalizzare il file WAV")?;
    shared.peak_bits.store(0, Ordering::Relaxed);
    shared.rms_bits.store(0, Ordering::Relaxed);
    Ok(())
}
