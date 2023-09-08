use std::mem::size_of;

use anyhow::{bail, Result};
use windows::{
    core::{PCSTR, PCWSTR},
    Win32::{
        Foundation::BOOLEAN,
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_SHARE_NONE, OPEN_EXISTING,
        },
        System::{
            Diagnostics::{
                Debug::{
                    ImageDirectoryEntryToDataEx, ImageNtHeader, ImageRvaToVa,
                    IMAGE_DIRECTORY_ENTRY_EXPORT,
                },
                ToolHelp::{
                    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, MODULEENTRY32W,
                    TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32,
                },
            },
            Memory::{
                CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ,
                MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READONLY,
            },
            SystemServices::IMAGE_EXPORT_DIRECTORY,
        },
    },
};

use crate::win_api_wrappers::SafeHandle;

fn kernel32_module_entry(process_id: u32) -> Result<MODULEENTRY32W> {
    let snapshot = SafeHandle(unsafe {
        CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, process_id)
    }?);

    let mut me = MODULEENTRY32W {
        dwSize: size_of::<MODULEENTRY32W>() as u32,
        ..Default::default()
    };

    unsafe { Module32FirstW(snapshot.0, &mut me) }?;
    loop {
        let module_name = unsafe { PCWSTR::from_raw(me.szModule.as_ptr()).to_string() }?;
        if module_name.to_ascii_uppercase() == "KERNEL32.DLL" {
            return Ok(me);
        }

        unsafe { Module32NextW(snapshot.0, &mut me) }?;
    }
}

fn load_library_w_addr_from_map_view_of_file(base: &ViewOfFile) -> Result<usize> {
    let nt_hdrs = unsafe { ImageNtHeader(base.0.Value) };
    let mut exp_size: u32 = Default::default();
    let exp = unsafe {
        ImageDirectoryEntryToDataEx(
            base.0.Value,
            BOOLEAN::from(false),
            IMAGE_DIRECTORY_ENTRY_EXPORT,
            &mut exp_size,
            None,
        )
    } as *const IMAGE_EXPORT_DIRECTORY;
    if exp.is_null() {
        bail!("ImageDirectoryEntryToDataEx failed");
    }
    let exp = unsafe { *exp };
    if exp.NumberOfNames == 0 {
        bail!("NumberOfNames is 0");
    }
    let addr_of_names =
        unsafe { ImageRvaToVa(nt_hdrs, base.0.Value, exp.AddressOfNames, None) } as *const u32;
    if addr_of_names.is_null() {
        bail!("ImageRvaToVa failed");
    }
    let addr_of_name_ordinals =
        unsafe { ImageRvaToVa(nt_hdrs, base.0.Value, exp.AddressOfNameOrdinals, None) }
            as *const u16;
    if addr_of_name_ordinals.is_null() {
        bail!("ImageRvaToVa failed");
    }
    let addr_of_funcs =
        unsafe { ImageRvaToVa(nt_hdrs, base.0.Value, exp.AddressOfFunctions, None) } as *const u32;
    if addr_of_funcs.is_null() {
        bail!("ImageRvaToVa failed");
    }

    let idx = (0..exp.NumberOfNames as usize)
        .map(
            |x| unsafe { ImageRvaToVa(nt_hdrs, base.0.Value, *addr_of_names.add(x), None) }
                as *const u8,
        )
        .position(|x| unsafe { PCSTR::from_raw(x).to_string() }.unwrap() == "LoadLibraryW")
        .unwrap();

    Ok((unsafe { *(addr_of_funcs.add(*(addr_of_name_ordinals.add(idx)) as usize)) }) as usize)
}

struct ViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS);

impl ViewOfFile {
    pub fn map(file_mapping: &SafeHandle) -> Result<Self> {
        let view = unsafe { MapViewOfFile(file_mapping.0, FILE_MAP_READ, 0, 0, 0) };
        if view.Value.is_null() {
            bail!("MapViewOfFile failed");
        }
        Ok(Self(view))
    }
}

impl Drop for ViewOfFile {
    fn drop(&mut self) {
        unsafe { UnmapViewOfFile(self.0) }.unwrap();
    }
}

pub fn load_library_w_addr(process_id: u32) -> Result<usize> {
    let me = kernel32_module_entry(process_id)?;

    let file = SafeHandle(unsafe {
        CreateFileW(
            PCWSTR::from_raw(me.szExePath.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_NONE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }?);

    let file_mapping =
        SafeHandle(unsafe { CreateFileMappingW(file.0, None, PAGE_READONLY, 0, 0, None) }?);
    let base = ViewOfFile::map(&file_mapping)?;

    Ok(me.modBaseAddr as usize + load_library_w_addr_from_map_view_of_file(&base)?)
}
