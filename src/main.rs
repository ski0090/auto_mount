use auto_mount::{smart_auto_mount, SmartMountError};

fn main() -> Result<(), SmartMountError> {
    // Option 1: Smart auto-mount (recommended)
    // Automatically decides whether to use GPT based on disk size
    smart_auto_mount()?;

    // Option 2: Simple auto-mount (no GPT conversion)
    // simple_auto_mount()?;

    // Option 3: Force GPT auto-mount
    // gpt_auto_mount()?;

    Ok(())
}
