#[derive(Debug)]
pub struct Hihiirokane {
    pub protocols: Protocols,
    pub shell: ShellData,
}

/// Delegate types for protocol implementations.
#[derive(Debug)]
pub struct Protocols {}

/// Data associated with Wayland shell implementations.
#[derive(Debug)]
pub struct ShellData {}
