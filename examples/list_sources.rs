use audiorecorder::audio::devices::list_sources;

fn main() -> anyhow::Result<()> {
    for device in list_sources()? {
        println!(
            "{:<40} monitor={:<5} default={:<5} {}",
            device.name,
            device.is_monitor,
            device.is_default,
            device.display_name()
        );
    }
    Ok(())
}
