use std::fs;
use std::os::unix;
use std::path::{Path, PathBuf};
use std::result;

use crate::cli::Apps;
use crate::path::{expand_app, expand_env, PathError};

#[derive(Debug)]
pub enum LinkStatus {
    SrcUnexists,
    DstUnexists,
    Exists,
    Unexpected(PathBuf),
}

use LinkStatus::*;

#[derive(Debug)]
pub struct Link {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub status: LinkStatus,
}

impl Link {
    fn src_unexists(src: PathBuf, dst: PathBuf) -> Self {
        Link {
            src,
            dst,
            status: SrcUnexists,
        }
    }

    fn dst_unexists(src: PathBuf, dst: PathBuf) -> Self {
        Link {
            src,
            dst,
            status: DstUnexists,
        }
    }

    fn exists(src: PathBuf, dst: PathBuf) -> Self {
        Link {
            src,
            dst,
            status: Exists,
        }
    }

    fn unexpected(src: PathBuf, dst: PathBuf, expected: PathBuf) -> Self {
        Link {
            src,
            dst,
            status: Unexpected(expected),
        }
    }
}

pub fn check_link<P: AsRef<Path>>(
    apps: &Apps,
    dstdir: P,
    srcdir: P,
    link: &(String, String),
) -> result::Result<Link, PathError> {
    let (dst, src) = link;
    let dst = dstdir
        .as_ref()
        .join(expand_app(&|name| apps.dir(name), &expand_env(dst)?)?);
    let src = srcdir.as_ref().join(expand_env(src)?);

    if src.exists() {
        let real_dst = src.read_link()?;
        if dst == real_dst {
            Ok(Link::exists(src, dst))
        } else {
            Ok(Link::unexpected(src, dst, real_dst))
        }
    } else if !dst.exists() {
        Ok(Link::dst_unexists(src, dst))
    } else {
        Ok(Link::src_unexists(src, dst))
    }
}

pub fn make_link(src: PathBuf, dst: PathBuf) -> result::Result<Link, PathError> {
    let dir = src
        .parent()
        .ok_or_else(|| PathError::NoParent(src.display().to_string()))?;
    fs::create_dir_all(dir)?;
    unix::fs::symlink(&dst, &src)?;
    Ok(Link::exists(src, dst))
}
