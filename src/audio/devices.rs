use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::{Context, FlagSet, State};
use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
use libpulse_binding::operation::{Operation, State as OperationState};
use libpulse_binding::sample::Spec;

#[derive(Clone, Debug)]
pub struct SourceDevice {
    pub name: String,
    pub description: String,
    pub is_monitor: bool,
    pub is_default: bool,
    pub spec: Spec,
}

impl SourceDevice {
    pub fn display_name(&self) -> String {
        let kind = if self.is_monitor {
            "sistema"
        } else {
            "microfono"
        };
        let default = if self.is_default {
            " · predefinito"
        } else {
            ""
        };
        format!(
            "{} [{}, {} Hz, {}ch]{}",
            self.description, kind, self.spec.rate, self.spec.channels, default
        )
    }
}

pub fn list_sources() -> Result<Vec<SourceDevice>> {
    let mut mainloop =
        Mainloop::new().ok_or_else(|| anyhow!("impossibile creare il mainloop PulseAudio"))?;
    let mut context = Context::new(&mainloop, "audiorecorder")
        .ok_or_else(|| anyhow!("impossibile creare il contesto PulseAudio"))?;
    context
        .connect(None, FlagSet::NOFLAGS, None)
        .map_err(|e| anyhow!("connessione a PulseAudio non riuscita: {e}"))?;

    wait_for_ready(&mut mainloop, &context)?;
    let default_source = default_source_name(&mut mainloop, &context)?;

    let collected: Arc<Mutex<Vec<SourceInfo>>> = Arc::new(Mutex::new(Vec::new()));
    let failed = Arc::new(AtomicBool::new(false));

    let sink = Arc::clone(&collected);
    let error_flag = Arc::clone(&failed);
    let operation = context
        .introspect()
        .get_source_info_list(move |result| match result {
            ListResult::Item(info) => {
                let name = info.name.as_deref().unwrap_or_default().to_string();
                let description = info
                    .description
                    .as_deref()
                    .filter(|d| !d.is_empty())
                    .unwrap_or(name.as_str())
                    .to_string();
                sink.lock().unwrap().push(SourceInfo {
                    name,
                    description,
                    is_monitor: info.monitor_of_sink.is_some(),
                    spec: info.sample_spec,
                });
            }
            ListResult::End => {}
            ListResult::Error => error_flag.store(true, Ordering::SeqCst),
        });

    run_operation(&mut mainloop, &operation)?;
    if failed.load(Ordering::SeqCst) {
        return Err(anyhow!(
            "errore durante l'enumerazione delle sorgenti audio"
        ));
    }

    let mut devices: Vec<SourceDevice> = collected
        .lock()
        .unwrap()
        .drain(..)
        .map(|info| SourceDevice {
            is_default: default_source.as_deref() == Some(info.name.as_str()),
            name: info.name,
            description: info.description,
            is_monitor: info.is_monitor,
            spec: info.spec,
        })
        .collect();

    devices.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then_with(|| a.is_monitor.cmp(&b.is_monitor))
            .then_with(|| a.description.cmp(&b.description))
    });

    Ok(devices)
}

struct SourceInfo {
    name: String,
    description: String,
    is_monitor: bool,
    spec: Spec,
}

fn wait_for_ready(mainloop: &mut Mainloop, context: &Context) -> Result<()> {
    loop {
        match context.get_state() {
            State::Ready => return Ok(()),
            State::Failed | State::Terminated => {
                return Err(anyhow!("connessione a PulseAudio fallita"))
            }
            _ => {}
        }
        iterate(mainloop)?;
    }
}

fn default_source_name(mainloop: &mut Mainloop, context: &Context) -> Result<Option<String>> {
    let value: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&value);
    let operation = context.introspect().get_server_info(move |info| {
        *slot.lock().unwrap() = info.default_source_name.as_deref().map(str::to_string);
    });
    run_operation(mainloop, &operation)?;
    let result = value.lock().unwrap().clone();
    Ok(result)
}

fn iterate(mainloop: &mut Mainloop) -> Result<()> {
    match mainloop.iterate(true) {
        IterateResult::Success(_) => Ok(()),
        IterateResult::Quit(_) => Err(anyhow!("mainloop PulseAudio terminato inaspettatamente")),
        IterateResult::Err(e) => Err(anyhow!("errore nel mainloop PulseAudio: {e}")),
    }
}

fn run_operation<T: ?Sized>(mainloop: &mut Mainloop, operation: &Operation<T>) -> Result<()> {
    while operation.get_state() == OperationState::Running {
        iterate(mainloop)?;
    }
    match operation.get_state() {
        OperationState::Done => Ok(()),
        _ => Err(anyhow!("operazione PulseAudio annullata")),
    }
}
