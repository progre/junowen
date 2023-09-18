use windows::Win32::Foundation::{CloseHandle, HANDLE};

pub struct SafeHandle(pub HANDLE);

impl Drop for SafeHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) }.unwrap();
    }
}
