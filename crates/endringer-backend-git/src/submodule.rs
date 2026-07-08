//! Submodule listing via gix's `.gitmodules` parser.

use std::path::PathBuf;

use anyhow::{Context, Result};
use endringer_backend_core::types::SubmoduleInfo;
use gix::bstr::ByteSlice;
use gix::Repository;

/// Returns metadata for every submodule declared in `.gitmodules`.
/// Returns an empty Vec when no `.gitmodules` file exists.
pub(crate) fn submodules(repo: &Repository) -> Result<Vec<SubmoduleInfo>> {
    let modules = match repo
        .modules()
        .context("reading .gitmodules")?
    {
        Some(m) => m,
        None => return Ok(vec![]),
    };

    let file = modules.to_owned();
    let mut out = Vec::new();

    for name_bstr in file.names() {
        let name = name_bstr.to_str_lossy().into_owned();

        let path = file
            .path(name_bstr)
            .ok()
            .map(|p| PathBuf::from(p.to_os_str_lossy().as_ref() as &std::ffi::OsStr))
            .unwrap_or_else(|| PathBuf::from(&name));

        let url = file
            .url(name_bstr)
            .ok()
            .map(|u| u.to_bstring().to_string());

        out.push(SubmoduleInfo { name, path, url });
    }

    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}
