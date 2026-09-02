// SPDX-License-Identifier: MIT

/// Version of the narrow native boundary owned by this target.
pub const ABI_VERSION: u32 = 1;

/// C-compatible description exchanged before native calls are accepted.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbiDescriptor {
    /// Version of the boundary contract.
    pub version: u32,
    /// Width of a native pointer in bits.
    pub pointer_width: u8,
    /// Reserved bytes that must remain zero for this version.
    pub reserved: [u8; 3],
}

impl AbiDescriptor {
    /// Returns the descriptor supported by this build.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            version: ABI_VERSION,
            pointer_width: current_pointer_width(),
            reserved: [0; 3],
        }
    }
}

/// Supplies the native descriptor without exposing native implementation details.
pub trait AbiPort {
    /// Returns the descriptor offered by the boundary.
    fn descriptor(&self) -> AbiDescriptor;
}

/// A native boundary compatibility failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbiError {
    /// The offered ABI version is not supported.
    VersionMismatch { expected: u32, actual: u32 },
    /// The offered pointer width is not supported.
    PointerWidthMismatch { expected: u8, actual: u8 },
}

/// Validates a native boundary before host work is admitted.
pub fn validate_abi(port: &impl AbiPort) -> Result<(), AbiError> {
    let offered = port.descriptor();
    if offered.version != ABI_VERSION {
        return Err(AbiError::VersionMismatch {
            expected: ABI_VERSION,
            actual: offered.version,
        });
    }
    let expected_width = AbiDescriptor::current().pointer_width;
    if offered.pointer_width != expected_width {
        return Err(AbiError::PointerWidthMismatch {
            expected: expected_width,
            actual: offered.pointer_width,
        });
    }
    Ok(())
}

const fn current_pointer_width() -> u8 {
    match usize::BITS {
        16 => 16,
        32 => 32,
        64 => 64,
        _ => 0,
    }
}
