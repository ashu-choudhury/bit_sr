//! SysListView32 Out-Of-Process Memory Interrogation.
//! Implements Section 5.2 of WINDOWS.md.

use crate::error::{Error, Result};
use crate::watchdog::safe_send_message_timeout;
use std::mem::size_of;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

const LVM_FIRST: u32 = 0x1000;
const LVM_GETITEMCOUNT: u32 = LVM_FIRST + 4;
const LVM_GETITEMTEXTW: u32 = LVM_FIRST + 115;
const LVIF_TEXT: u32 = 0x01;

#[repr(C)]
struct LVITEMW {
    mask: u32,
    i_item: i32,
    i_sub_item: i32,
    state: u32,
    state_mask: u32,
    psz_text: usize,
    cch_text_max: i32,
    i_image: i32,
    l_param: isize,
    i_indent: i32,
    i_group_id: i32,
    c_columns: u32,
    pu_columns: usize,
    pi_col_fmt: usize,
    i_group: i32,
}

pub struct SysListView32Reader;

impl SysListView32Reader {
    /// Queries the total number of items in a SysListView32 control.
    pub fn get_item_count(hwnd: HWND) -> Option<usize> {
        safe_send_message_timeout(hwnd, LVM_GETITEMCOUNT, WPARAM(0), LPARAM(0), 500)
    }

    /// Reads item text across process boundaries safely using VirtualAllocEx.
    pub fn get_item_text(hwnd: HWND, item_index: i32, sub_item_index: i32) -> Result<String> {
        let mut process_id: u32 = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        }

        if process_id == 0 {
            return Err(Error::Internal("Failed to obtain process ID for window".into()));
        }

        let process_handle = unsafe {
            OpenProcess(
                PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
                false,
                process_id,
            )
        }
        .map_err(|e| Error::Windows(e))?;

        let total_size = size_of::<LVITEMW>() + 512;
        let remote_mem = unsafe {
            VirtualAllocEx(
                process_handle,
                None,
                total_size,
                MEM_COMMIT,
                PAGE_READWRITE,
            )
        };

        if remote_mem.is_null() {
            unsafe { let _ = CloseHandle(process_handle); }
            return Err(Error::Internal("VirtualAllocEx failed in target process".into()));
        }

        let remote_text_ptr = (remote_mem as usize) + size_of::<LVITEMW>();

        let local_item = LVITEMW {
            mask: LVIF_TEXT,
            i_item: item_index,
            i_sub_item: sub_item_index,
            state: 0,
            state_mask: 0,
            psz_text: remote_text_ptr,
            cch_text_max: 256,
            i_image: 0,
            l_param: 0,
            i_indent: 0,
            i_group_id: 0,
            c_columns: 0,
            pu_columns: 0,
            pi_col_fmt: 0,
            i_group: 0,
        };

        // Write LVITEMW to target process
        let write_ok = unsafe {
            WriteProcessMemory(
                process_handle,
                remote_mem,
                &local_item as *const _ as *const _,
                size_of::<LVITEMW>(),
                None,
            )
        };

        if write_ok.is_err() {
            unsafe {
                let _ = VirtualFreeEx(process_handle, remote_mem, 0, MEM_RELEASE);
                let _ = CloseHandle(process_handle);
            }
            return Err(Error::Internal("WriteProcessMemory failed".into()));
        }

        // Send LVM_GETITEMTEXTW with timeout
        let result = safe_send_message_timeout(
            hwnd,
            LVM_GETITEMTEXTW,
            WPARAM(item_index as usize),
            LPARAM(remote_mem as isize),
            1000,
        );

        let mut text_buf = [0u16; 256];
        if result.is_some() {
            let _ = unsafe {
                ReadProcessMemory(
                    process_handle,
                    remote_text_ptr as *const _,
                    text_buf.as_mut_ptr() as *mut _,
                    512,
                    None,
                )
            };
        }

        // Cleanup remote memory & process handle
        unsafe {
            let _ = VirtualFreeEx(process_handle, remote_mem, 0, MEM_RELEASE);
            let _ = CloseHandle(process_handle);
        }

        let len = text_buf.iter().position(|&c| c == 0).unwrap_or(text_buf.len());
        Ok(String::from_utf16_lossy(&text_buf[..len]))
    }
}
