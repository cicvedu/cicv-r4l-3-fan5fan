// SPDX-License-Identifier: GPL-2.0

//! Rust miscellaneous device sample.

use kernel::prelude::*;
use kernel::{
    chrdev, condvar_init,
    file::{self, File},
    io_buffer::{IoBufferReader, IoBufferWriter},
    mutex_init,
    str::CStr,
    sync::{CondVar, Mutex},
    task::Task,
};

module! {
    type: RustCdev,
    name: "completion",
    author: "fan5fan",
    description: "Example of Kernel's completion mechanism for rust",
    license: "GPL",
}

/* struct SharedState {
    next: Mutex<bool>,
    state_changed: CondVar,
}

impl SharedState {
    fn try_new() -> Result<Arc<Self>> {
        let mut state = Pin::from(UniqueArc::try_new(Self {
            next: unsafe { Mutex::new(false) },
            state_changed: unsafe { CondVar::new() },
        })?);

        let pinned = unsafe { state.as_mut().map_unchecked_mut(|s| &mut s.state_changed) };
        kernel::condvar_init!(pinned, "SharedState::state_changed");

        let pinned = unsafe { state.as_mut().map_unchecked_mut(|s| &mut s.next) };
        kernel::mutex_init!(pinned, "SharedState::next");

        Ok(state.into())
    }
}

struct CdevOps;
#[vtable]
impl file::Operations for CdevOps {
    type Data = Arc<SharedState>;
    // type OpenData = Arc<SharedState>;

    fn open(_shared: &(), _file: &File) -> Result<Self::Data> {
        let state = SharedState::try_new()?;
        Ok(state.clone())
    }

    fn read(
        shared: ArcBorrow<'_, SharedState>,
        _: &File,
        data: &mut impl IoBufferWriter,
        offset: u64,
    ) -> Result<usize> {
        if data.is_empty() || offset != 0 {
            return Ok(0);
        }

        let mut lock = shared.next.lock();

        if shared.state_changed.wait(&mut lock) {
            return Err(EINTR);
        }

        shared.state_changed.notify_one();
        shared.state_changed.notify_all();
        shared.state_changed.free_waiters();

        pr_info!("next: {}\n", *lock);

        Ok(0)
    }

    fn write(
        shared: ArcBorrow<'_, SharedState>,
        _: &File,
        data: &mut impl IoBufferReader,
        _offset: u64,
    ) -> Result<usize> {
        pr_info!("write: {}\n", data.len());

        let mut lock = shared.next.lock();
        *lock = true;

        shared.state_changed.notify_one();
        shared.state_changed.notify_all();
        shared.state_changed.free_waiters();
        Ok(data.len())
    }
} */

kernel::init_static_sync! {
    static SAMPLE_MUTEX: Mutex<bool> = false;
    static SAMPLE_CONDVAR: CondVar;
}

struct CdevOps;
#[vtable]
impl file::Operations for CdevOps {
    type Data = ();
    // type OpenData = Arc<SharedState>;

    fn open(_shared: &(), _file: &File) -> Result<Self::Data> {
        pr_info!("completion_open() is invoked\n");

        let mut data = Pin::from(Box::try_new(unsafe { Mutex::new(0) })?);
        mutex_init!(data.as_mut(), "Sync::data");

        // SAFETY: `init` is called below.
        let mut cv = Pin::from(Box::try_new(unsafe { CondVar::new() })?);
        condvar_init!(cv.as_mut(), "Sync::cv");

        Ok(())
    }

    fn read(_shared: (), _: &File, data: &mut impl IoBufferWriter, offset: u64) -> Result<usize> {
        pr_info!("completion_read() is invoked\n");
        if data.is_empty() || offset != 0 {
            return Ok(0);
        }

        {
            let mut lock = SAMPLE_MUTEX.lock();

            pr_info!(
                "process {}({}) is going to sleep\n",
                Task::current().pid(),
                CStr::from_char_ptr(Task::current().comm().as_ptr())
            );
            while *lock != true {
                if SAMPLE_CONDVAR.wait(&mut lock) {
                    return Err(EINTR);
                }
            }

            *lock = false;
        }

        SAMPLE_CONDVAR.notify_one();
        SAMPLE_CONDVAR.notify_all();
        SAMPLE_CONDVAR.free_waiters();

        pr_info!(
            "awoken {}({})\n",
            Task::current().pid(),
            CStr::from_char_ptr(Task::current().comm().as_ptr())
        );

        Ok(0)
    }

    fn write(_shared: (), _: &File, data: &mut impl IoBufferReader, _offset: u64) -> Result<usize> {
        pr_info!("completion_write() is invoked\n");

        let mut lock = SAMPLE_MUTEX.lock();
        *lock = true;

        SAMPLE_CONDVAR.notify_one();
        SAMPLE_CONDVAR.notify_all();
        SAMPLE_CONDVAR.free_waiters();

        pr_info!(
            "process {}({}) awakening the readers...\n",
            Task::current().pid(),
            CStr::from_char_ptr(Task::current().comm().as_ptr())
        );

        Ok(data.len())
    }
}

struct RustCdev {
    _dev: Pin<Box<chrdev::Registration<1>>>,
}

impl kernel::Module for RustCdev {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        pr_info!("{name} is loaded (init)\n");

        // let state = SharedState::try_new()?;
        let mut chrdev_reg = chrdev::Registration::new_pinned(name, 0, module)?;
        chrdev_reg.as_mut().register::<CdevOps>()?;

        Ok(RustCdev { _dev: chrdev_reg })
    }
}

impl Drop for RustCdev {
    fn drop(&mut self) {
        pr_info!("completion unloaded (exit)\n");
    }
}
