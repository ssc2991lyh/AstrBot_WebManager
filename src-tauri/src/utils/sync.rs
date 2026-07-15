use std::sync::{LockResult, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

fn recover_poison<T>(result: LockResult<T>, name: &str, lock_kind: &str) -> T {
    match result {
        Ok(guard) => guard,
        Err(e) => {
            log::warn!("{name} {lock_kind} poisoned, recovering inner state");
            e.into_inner()
        }
    }
}

pub(crate) fn lock_mutex_recover<'a, T>(lock: &'a Mutex<T>, name: &str) -> MutexGuard<'a, T> {
    recover_poison(lock.lock(), name, "mutex")
}

pub(crate) fn read_lock_recover<'a, T>(lock: &'a RwLock<T>, name: &str) -> RwLockReadGuard<'a, T> {
    recover_poison(lock.read(), name, "read lock")
}

pub(crate) fn write_lock_recover<'a, T>(
    lock: &'a RwLock<T>,
    name: &str,
) -> RwLockWriteGuard<'a, T> {
    recover_poison(lock.write(), name, "write lock")
}
