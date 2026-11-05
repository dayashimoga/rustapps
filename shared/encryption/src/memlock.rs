#![allow(unsafe_code)]
//! OS Memory Page Locking
//!
//! Provides `LockedKey` and `LockedBytes` which lock sensitive cryptographic keys
//! in RAM using Windows `VirtualLock` or Unix `mlock`. This prevents the OS kernel
//! from swapping sensitive key material to disk (swapfile or pagefile.sys).
//! Memory is automatically unlocked and zeroized upon drop.

use zeroize::Zeroize;

#[cfg(windows)]
#[allow(non_snake_case)]
extern "system" {
    fn VirtualLock(lpAddress: *const std::ffi::c_void, dwSize: usize) -> i32;
    fn VirtualUnlock(lpAddress: *const std::ffi::c_void, dwSize: usize) -> i32;
}

#[cfg(unix)]
extern "C" {
    fn mlock(addr: *const std::ffi::c_void, len: usize) -> std::ffi::c_int;
    fn munlock(addr: *const std::ffi::c_void, len: usize) -> std::ffi::c_int;
}

/// A fixed-size memory-locked buffer for master cryptographic keys.
///
/// Prevents the memory buffer from being written to swap or page files.
/// Guarantees zeroization and memory unlocking when dropped.
#[derive(Debug)]
pub struct LockedKey<const N: usize> {
    data: [u8; N],
    is_locked: bool,
}

impl<const N: usize> LockedKey<N> {
    /// Creates a new `LockedKey` wrapping the provided byte array and attempts to lock it in RAM.
    pub fn new(key: [u8; N]) -> Self {
        let mut key_obj = Self {
            data: key,
            is_locked: false,
        };
        key_obj.lock();
        key_obj
    }

    /// Access the underlying key bytes.
    pub fn as_bytes(&self) -> &[u8; N] {
        &self.data
    }

    /// Returns whether the operating system successfully locked the memory buffer in RAM.
    pub fn is_locked(&self) -> bool {
        self.is_locked
    }

    fn lock(&mut self) {
        #[cfg(windows)]
        {
            // SAFETY: `self.data` points to a valid, aligned buffer of N bytes that is
            // initialized and alive for the lifetime of `self`.
            let res = unsafe {
                VirtualLock(self.data.as_ptr() as *const std::ffi::c_void, N)
            };
            self.is_locked = res != 0;
        }

        #[cfg(unix)]
        {
            // SAFETY: `self.data` points to a valid, aligned buffer of N bytes that is
            // initialized and alive for the lifetime of `self`.
            let res = unsafe {
                mlock(self.data.as_ptr() as *const std::ffi::c_void, N)
            };
            self.is_locked = res == 0;
        }
    }
}

impl<const N: usize> Drop for LockedKey<N> {
    fn drop(&mut self) {
        if self.is_locked {
            #[cfg(windows)]
            {
                // SAFETY: `self.data` is currently locked and valid for N bytes.
                // We unlock it before wiping and releasing memory.
                unsafe {
                    VirtualUnlock(self.data.as_ptr() as *const std::ffi::c_void, N);
                }
            }

            #[cfg(unix)]
            {
                // SAFETY: `self.data` is currently locked and valid for N bytes.
                // We unlock it before wiping and releasing memory.
                unsafe {
                    munlock(self.data.as_ptr() as *const std::ffi::c_void, N);
                }
            }
        }
        self.data.zeroize();
    }
}

/// A dynamic-size memory-locked buffer for secrets, passphrases, and tokens.
#[derive(Debug)]
pub struct LockedBytes {
    data: Vec<u8>,
    is_locked: bool,
}

impl LockedBytes {
    /// Creates a new `LockedBytes` instance from a vector and attempts to lock it in RAM.
    pub fn new(data: Vec<u8>) -> Self {
        let mut bytes_obj = Self {
            data,
            is_locked: false,
        };
        bytes_obj.lock();
        bytes_obj
    }

    /// Access the underlying bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Returns whether the buffer was successfully locked in RAM.
    pub fn is_locked(&self) -> bool {
        self.is_locked
    }

    fn lock(&mut self) {
        if self.data.is_empty() {
            return;
        }

        #[cfg(windows)]
        {
            // SAFETY: `self.data` points to a valid heap allocation of length `len` bytes.
            let res = unsafe {
                VirtualLock(self.data.as_ptr() as *const std::ffi::c_void, self.data.len())
            };
            self.is_locked = res != 0;
        }

        #[cfg(unix)]
        {
            // SAFETY: `self.data` points to a valid heap allocation of length `len` bytes.
            let res = unsafe {
                mlock(self.data.as_ptr() as *const std::ffi::c_void, self.data.len())
            };
            self.is_locked = res == 0;
        }
    }
}

impl Drop for LockedBytes {
    fn drop(&mut self) {
        if self.is_locked && !self.data.is_empty() {
            #[cfg(windows)]
            {
                // SAFETY: `self.data` was previously locked for `self.data.len()` bytes.
                unsafe {
                    VirtualUnlock(self.data.as_ptr() as *const std::ffi::c_void, self.data.len());
                }
            }

            #[cfg(unix)]
            {
                // SAFETY: `self.data` was previously locked for `self.data.len()` bytes.
                unsafe {
                    munlock(self.data.as_ptr() as *const std::ffi::c_void, self.data.len());
                }
            }
        }
        self.data.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locked_key_lifecycle() {
        let raw = [42u8; 32];
        let locked = LockedKey::new(raw);
        assert_eq!(locked.as_bytes(), &raw);
        // Note: is_locked might be true or false depending on OS privileges/working set,
        // but drop and zeroize must always succeed safely.
        let _ = locked.is_locked();
    }

    #[test]
    fn test_locked_bytes_lifecycle() {
        let raw = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let locked = LockedBytes::new(raw.clone());
        assert_eq!(locked.as_slice(), raw.as_slice());
        let _ = locked.is_locked();
    }

    #[test]
    fn test_empty_locked_bytes() {
        let locked = LockedBytes::new(Vec::new());
        assert!(!locked.is_locked());
        assert!(locked.as_slice().is_empty());
    }
}
