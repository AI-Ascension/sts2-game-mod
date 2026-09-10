// SPDX-License-Identifier: MIT

#[repr(C)]
#[derive(Debug)]
pub struct RuntimeCallbacks {
    pub request: Option<super::RuntimeRequestCallback>,
}

#[repr(C)]
#[derive(Debug)]
pub struct RuntimeRequest {
    pub kind: u32,
    pub instance_id: *const u8,
    pub instance_id_len: usize,
    pub caller_id: *const u8,
    pub caller_id_len: usize,
    pub session_id: *const u8,
    pub session_id_len: usize,
    pub lease_id: *const u8,
    pub lease_id_len: usize,
    pub lease_epoch: *const u8,
    pub lease_epoch_len: usize,
    pub correlation_id: *const u8,
    pub correlation_id_len: usize,
    pub body: *const u8,
    pub body_len: usize,
}
