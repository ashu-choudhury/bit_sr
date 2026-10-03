//! Clean-room NVDA Controller Client Local RPC (LRPC) Server implementation.
//! Provides full binary and protocol compatibility with `nvdaControllerClient.dll`
//! without using or linking any GPL code.

use bit_sr_core::events::AccessibilityEvent;
use crossbeam_channel::Sender;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::PSECURITY_DESCRIPTOR;
use windows::Win32::System::StationsAndDesktops::{
    GetThreadDesktop, GetUserObjectInformationW, UOI_NAME,
};

type RpcIfHandle = *const c_void;
type RpcStatus = i32;

const RPC_S_OK: RpcStatus = 0;
const RPC_S_DUPLICATE_ENDPOINT: RpcStatus = 1740;

const RPC_C_PROTSEQ_MAX_REQS_DEFAULT: u32 = 10;
const RPC_C_LISTEN_MAX_CALLS_DEFAULT: u32 = 1234;
const RPC_IF_AUTOLISTEN: u32 = 0x0001;
const RPC_IF_ALLOW_CALLBACKS_WITH_NO_AUTH: u32 = 0x0010;

unsafe extern "system" {
    fn RpcServerUseProtseqEpW(
        Protseq: PCWSTR,
        MaxCalls: u32,
        Endpoint: PCWSTR,
        SecurityDescriptor: *const c_void,
    ) -> RpcStatus;

    fn RpcServerRegisterIf3(
        IfSpec: RpcIfHandle,
        MgrTypeUuid: *const c_void,
        MgrEpv: *const c_void,
        Flags: u32,
        MaxCalls: u32,
        MaxRpcSize: u32,
        IfCallback: *const c_void,
        SecurityDescriptor: *const c_void,
    ) -> RpcStatus;

    fn RpcServerUnregisterIf(
        IfSpec: RpcIfHandle,
        MgrTypeUuid: *const c_void,
        WaitForCallsToComplete: u32,
    ) -> RpcStatus;
}

unsafe extern "C" {
    static NvdaController_v1_0_s_ifspec: *const c_void;
    static NvdaController2_v1_0_s_ifspec: *const c_void;
    static NvdaController3_v1_0_s_ifspec: *const c_void;
}

// Global dispatcher state accessed by the C RPC callback routines
static EVENT_SENDER: OnceLock<Sender<AccessibilityEvent>> = OnceLock::new();
static IS_SPEAKING_FLAG: AtomicBool = AtomicBool::new(false);
static SERVER_MUTEX: Mutex<Option<NvdaServerInner>> = Mutex::new(None);

#[derive(Clone, Copy)]
struct SendableRpcIfHandle(RpcIfHandle);
unsafe impl Send for SendableRpcIfHandle {}

struct NvdaServerInner {
    registered_interfaces: Vec<SendableRpcIfHandle>,
}


// ---------------------------------------------------------------------------
// Exported C RPC Server Routines dispatched by the MIDL stub
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "system" fn testIfRunning(_handle: *mut c_void) -> u32 {
    ERROR_SUCCESS.0
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn speakText(_handle: *mut c_void, text: *const u16) -> u32 {
    if text.is_null() {
        return ERROR_SUCCESS.0;
    }
    let string = unsafe { utf16_ptr_to_string(text) };
    if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.try_send(AccessibilityEvent::ExternalSpeech {
            text: string,
            interrupt: true,
        });
    }
    ERROR_SUCCESS.0
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn cancelSpeech(_handle: *mut c_void) -> u32 {
    if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.try_send(AccessibilityEvent::ExternalSpeechCancel);
    }
    ERROR_SUCCESS.0
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn brailleMessage(_handle: *mut c_void, message: *const u16) -> u32 {
    if message.is_null() {
        return ERROR_SUCCESS.0;
    }
    let string = unsafe { utf16_ptr_to_string(message) };
    if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.try_send(AccessibilityEvent::ExternalSpeech {
            text: string,
            interrupt: false,
        });
    }
    ERROR_SUCCESS.0
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn getProcessId(_handle: *mut c_void, pid: *mut u32) -> u32 {
    if !pid.is_null() {
        unsafe {
            *pid = std::process::id();
        }
    }
    ERROR_SUCCESS.0
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn speakSsml(
    _handle: *mut c_void,
    ssml: *const u16,
    _symbol_level: i32,
    _priority: u32,
    _asynchronous: u8,
) -> u32 {
    if ssml.is_null() {
        return ERROR_SUCCESS.0;
    }
    let raw = unsafe { utf16_ptr_to_string(ssml) };
    let stripped = strip_xml_tags(&raw);
    if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.try_send(AccessibilityEvent::ExternalSpeech {
            text: stripped,
            interrupt: false,
        });
    }
    ERROR_SUCCESS.0
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn isSpeaking(_handle: *mut c_void, speaking: *mut u8) -> u32 {
    if !speaking.is_null() {
        unsafe {
            *speaking = if IS_SPEAKING_FLAG.load(Ordering::Relaxed) { 1 } else { 0 };
        }
    }
    ERROR_SUCCESS.0
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

unsafe fn utf16_ptr_to_string(ptr: *const u16) -> String {
    let mut len = 0;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
}

fn strip_xml_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(ch);
        }
    }
    out.trim().to_string()
}

/// Generates the NVDA desktop namespace: `<sessionId>.<desktopName>`
pub fn get_desktop_namespace() -> String {
    let session_id = unsafe {
        let mut id = 0u32;
        let _ = windows::Win32::System::RemoteDesktop::ProcessIdToSessionId(std::process::id(), &mut id);
        id
    };

    let desktop_name = unsafe {
        if let Ok(h_desk) = GetThreadDesktop(windows::Win32::System::Threading::GetCurrentThreadId()) {
            let mut buf = [0u16; 64];
            let mut needed = 0u32;
            if GetUserObjectInformationW(
                HANDLE(h_desk.0),
                UOI_NAME,
                Some(buf.as_mut_ptr() as *mut _),
                (buf.len() * 2) as u32,
                Some(&mut needed),
            )
            .is_ok()
            {
                let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                String::from_utf16_lossy(&buf[..len])
            } else {
                "Default".to_string()
            }
        } else {
            "Default".to_string()
        }
    };

    format!("{}.{}", session_id, desktop_name)
}

/// RAII handle for the running NVDA Controller Client compatibility server.
pub struct NvdaControllerServer {
    endpoint: String,
    yielded_to_nvda: bool,
}

impl NvdaControllerServer {
    /// Starts the NVDA Controller LRPC server on the current desktop.
    /// If an official NVDA instance is already active and owns the endpoint,
    /// catches `RPC_S_DUPLICATE_ENDPOINT` cleanly and yields to NVDA.
    pub fn start(tx: Sender<AccessibilityEvent>) -> crate::Result<Self> {
        let _ = EVENT_SENDER.set(tx);

        let ns = get_desktop_namespace();
        let endpoint = format!("NvdaCtlr.{}", ns);
        log::info!("Starting NVDA Controller LRPC server on endpoint: ncalrpc:[{}]", endpoint);

        let mut wide_endpoint: Vec<u16> = endpoint.encode_utf16().chain(std::iter::once(0)).collect();
        let mut wide_protseq: Vec<u16> = "ncalrpc".encode_utf16().chain(std::iter::once(0)).collect();
        let wide_sddl: Vec<u16> = "D:(A;;GA;;;WD)(A;;GA;;;AC)\0".encode_utf16().collect();

        unsafe {
            let mut psd = PSECURITY_DESCRIPTOR::default();
            let mut sec_size = 0u32;
            let conv_res = ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(wide_sddl.as_ptr()),
                SDDL_REVISION_1,
                &mut psd,
                Some(&mut sec_size),
            );

            if let Err(e) = conv_res {
                log::warn!("Could not create security descriptor for NVDA LRPC: {:?}", e);
            }

            let status = RpcServerUseProtseqEpW(
                PCWSTR(wide_protseq.as_mut_ptr()),
                RPC_C_PROTSEQ_MAX_REQS_DEFAULT,
                PCWSTR(wide_endpoint.as_mut_ptr()),
                psd.0,
            );

            if status == RPC_S_DUPLICATE_ENDPOINT {
                log::info!(
                    "NVDA is currently active on desktop '{}'. bit_sr yielding LRPC endpoint gracefully.",
                    ns
                );
                if !psd.0.is_null() {
                    let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(psd.0)));
                }
                return Ok(Self {
                    endpoint,
                    yielded_to_nvda: true,
                });
            } else if status != RPC_S_OK {
                log::error!("RpcServerUseProtseqEpW failed with status: {}", status);
                if !psd.0.is_null() {
                    let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(psd.0)));
                }
                return Err(crate::Error::Internal(format!(
                    "RpcServerUseProtseqEpW failed (status: {})",
                    status
                )));
            }

            // Register standard NvdaController interfaces: 1.0, 2.0, 3.0
            let interfaces: [RpcIfHandle; 3] = [
                NvdaController_v1_0_s_ifspec,
                NvdaController2_v1_0_s_ifspec,
                NvdaController3_v1_0_s_ifspec,
            ];

            let mut registered = Vec::new();
            for (idx, &if_spec) in interfaces.iter().enumerate() {
                let reg_status = RpcServerRegisterIf3(
                    if_spec,
                    std::ptr::null(),
                    std::ptr::null(),
                    RPC_IF_AUTOLISTEN | RPC_IF_ALLOW_CALLBACKS_WITH_NO_AUTH,
                    RPC_C_LISTEN_MAX_CALLS_DEFAULT,
                    0,
                    std::ptr::null(),
                    psd.0,
                );

                if reg_status == RPC_S_OK {
                    registered.push(SendableRpcIfHandle(if_spec));
                } else {
                    log::warn!(
                        "RpcServerRegisterIf3 failed for interface index {} (status: {})",
                        idx,
                        reg_status
                    );
                }
            }

            if !psd.0.is_null() {
                let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(psd.0)));
            }

            let mut inner = SERVER_MUTEX.lock().unwrap();
            *inner = Some(NvdaServerInner {
                registered_interfaces: registered,
            });
        }

        log::info!("NVDA Controller LRPC server active and ready for audio games and applications!");
        Ok(Self {
            endpoint,
            yielded_to_nvda: false,
        })
    }

    /// Returns the registered LRPC endpoint string (e.g. `NvdaCtlr.1.Default`).
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Whether bit_sr yielded because an official NVDA instance is already running.
    pub fn is_yielded_to_nvda(&self) -> bool {
        self.yielded_to_nvda
    }

    /// Updates whether bit_sr is currently speaking for `nvdaController_isSpeaking`.
    pub fn set_speaking(speaking: bool) {
        IS_SPEAKING_FLAG.store(speaking, Ordering::Relaxed);
    }
}

impl Drop for NvdaControllerServer {
    fn drop(&mut self) {
        if self.yielded_to_nvda {
            return;
        }

        let mut guard = SERVER_MUTEX.lock().unwrap();
        if let Some(inner) = guard.take() {
            unsafe {
                for if_spec in inner.registered_interfaces {
                    let _ = RpcServerUnregisterIf(if_spec.0, std::ptr::null(), 1);
                }
            }
            log::info!("NVDA Controller LRPC server stopped cleanly.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_namespace_formatting() {
        let ns = get_desktop_namespace();
        assert!(!ns.is_empty());
        assert!(ns.contains('.'));
        println!("Calculated desktop namespace: {}", ns);
    }

    #[test]
    fn test_strip_xml_tags() {
        let ssml = "<speak>Hello <emphasis>world</emphasis>!</speak>";
        assert_eq!(strip_xml_tags(ssml), "Hello world!");

        let plain = "Simple plain text";
        assert_eq!(strip_xml_tags(plain), "Simple plain text");
    }

    #[test]
    fn test_nvda_controller_routines() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let _ = EVENT_SENDER.set(tx);

        unsafe {
            // 1. testIfRunning
            let status = testIfRunning(std::ptr::null_mut());
            assert_eq!(status, 0);

            // 2. getProcessId
            let mut pid = 0u32;
            let status = getProcessId(std::ptr::null_mut(), &mut pid);
            assert_eq!(status, 0);
            assert_eq!(pid, std::process::id());

            // 3. speakText
            let wide: Vec<u16> = "Welcome to bit_sr\0".encode_utf16().collect();
            let status = speakText(std::ptr::null_mut(), wide.as_ptr());
            assert_eq!(status, 0);

            let event = rx.try_recv().expect("Expected ExternalSpeech event");
            match event {
                AccessibilityEvent::ExternalSpeech { text, interrupt } => {
                    assert_eq!(text, "Welcome to bit_sr");
                    assert!(interrupt);
                }
                other => panic!("Unexpected event: {:?}", other),
            }

            // 4. cancelSpeech
            let status = cancelSpeech(std::ptr::null_mut());
            assert_eq!(status, 0);

            let event = rx.try_recv().expect("Expected ExternalSpeechCancel event");
            assert_eq!(event, AccessibilityEvent::ExternalSpeechCancel);

            // 5. isSpeaking flag
            let mut speaking = 0u8;
            NvdaControllerServer::set_speaking(true);
            let status = isSpeaking(std::ptr::null_mut(), &mut speaking);
            assert_eq!(status, 0);
            assert_eq!(speaking, 1);

            NvdaControllerServer::set_speaking(false);
            let status = isSpeaking(std::ptr::null_mut(), &mut speaking);
            assert_eq!(status, 0);
            assert_eq!(speaking, 0);

            // 6. speakSsml
            let ssml_wide: Vec<u16> = "<speak>SSML test</speak>\0".encode_utf16().collect();
            let status = speakSsml(std::ptr::null_mut(), ssml_wide.as_ptr(), 0, 0, 1);
            assert_eq!(status, 0);

            let event = rx.try_recv().expect("Expected ExternalSpeech event from SSML");
            match event {
                AccessibilityEvent::ExternalSpeech { text, interrupt } => {
                    assert_eq!(text, "SSML test");
                    assert!(!interrupt);
                }
                other => panic!("Unexpected event: {:?}", other),
            }
        }
    }
}
