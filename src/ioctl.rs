use nix::{
    fcntl::{OFlag, open},
    ioctl_write_ptr,
    sys::stat::Mode,
    unistd::close,
};

use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    iter::zip,
    mem::size_of,
    os::{fd::AsRawFd, unix::io::RawFd},
    path::PathBuf,
    process::exit,
};

const BCH_COUNTER_NR: u16 = 100;
pub const BCH2_COUNTER_NAMES: &[&str] = &[
    "io_read",
    "io_write",
    "io_move",
    "bucket_invalidate",
    "bucket_discard",
    "bucket_alloc",
    "bucket_alloc_fail",
    "btree_cache_scan",
    "btree_cache_reap",
    "btree_cache_cannibalize",
    "btree_cache_cannibalize_lock",
    "btree_cache_cannibalize_lock_fail",
    "btree_cache_cannibalize_unlock",
    "btree_node_write",
    "btree_node_read",
    "btree_node_compact",
    "btree_node_merge",
    "btree_node_split",
    "btree_node_rewrite",
    "btree_node_alloc",
    "btree_node_free",
    "btree_node_set_root",
    "btree_path_relock_fail",
    "btree_path_upgrade_fail",
    "btree_reserve_get_fail",
    "journal_entry_full",
    "journal_full",
    "journal_reclaim_finish",
    "journal_reclaim_start",
    "journal_write",
    "io_read_promote",
    "io_read_bounce",
    "io_read_retry",
    "io_read_split",
    "io_read_reuse_race",
    "io_move_read",
    "io_move_write",
    "io_move_finish",
    "io_move_fail",
    "io_move_start_fail",
    "copygc",
    "copygc_wait",
    "gc_gens_end",
    "gc_gens_start",
    "trans_blocked_journal_reclaim",
    "trans_restart_btree_node_reused",
    "trans_restart_btree_node_split",
    "trans_restart_fault_inject",
    "trans_restart_iter_upgrade",
    "trans_restart_journal_preres_get",
    "trans_restart_journal_reclaim",
    "trans_restart_journal_res_get",
    "trans_restart_key_cache_key_realloced",
    "trans_restart_key_cache_raced",
    "trans_restart_mark_replicas",
    "trans_restart_mem_realloced",
    "trans_restart_memory_allocation_failure",
    "trans_restart_relock",
    "trans_restart_relock_after_fill",
    "trans_restart_relock_key_cache_fill",
    "trans_restart_relock_next_node",
    "trans_restart_relock_parent_for_fill",
    "trans_restart_relock_path",
    "trans_restart_relock_path_intent",
    "trans_restart_too_many_iters",
    "trans_restart_traverse",
    "trans_restart_upgrade",
    "trans_restart_would_deadlock",
    "trans_restart_would_deadlock_write",
    "trans_restart_injected",
    "trans_restart_key_cache_upgrade",
    "trans_traverse_all",
    "transaction_commit",
    "write_super",
    "trans_restart_would_deadlock_recursion_limit",
    "trans_restart_write_buffer_flush",
    "trans_restart_split_race",
    "write_buffer_flush_slowpath",
    "write_buffer_flush_sync",
    "bucket_discard_fast",
    "io_read_inline",
    "io_read_hole",
    "io_move_write_fail",
    "io_move_created_rebalance",
    "io_move_evacuate_bucket",
    "io_read_nopromote",
    "io_read_nopromote_may_not",
    "io_read_nopromote_already_promoted",
    "io_read_nopromote_unwritten",
    "io_read_nopromote_congested",
    "io_read_nopromote_in_flight",
    "io_move_drop_only",
    "io_move_noop",
    "error_throw",
    "accounting_key_to_wb_slowpath",
    "io_read_fail_and_poison",
];

#[repr(C)]
struct BchIoctlQueryCounters {
    nr: u16,
    flags: u16,
    pad: u32,
    d: [u64; 0],
}

// This is what actually creates the bch_ioctl_query_counters function
ioctl_write_ptr!(bch_ioctl_query_counters, 0xbc, 21, BchIoctlQueryCounters);

fn read_counters(fd: RawFd) -> nix::Result<Box<BchIoctlQueryCounters>> {
    let size = size_of::<BchIoctlQueryCounters>() + (BCH_COUNTER_NR as usize) * size_of::<u64>();

    // This allocates zeroed memory
    let mut counters: Box<BchIoctlQueryCounters> = unsafe {
        let layout = std::alloc::Layout::from_size_align(size, 8).unwrap();
        let ptr = std::alloc::alloc_zeroed(layout);
        Box::from_raw(ptr as *mut BchIoctlQueryCounters)
    };

    //  This sets any needed fields
    counters.nr = BCH_COUNTER_NR;

    // This will actually issue the ioctl
    unsafe {
        bch_ioctl_query_counters(fd, &*counters)?;
    }

    Ok(counters)
}

pub fn process_counters(path: Option<OsString>) -> HashMap<String, u64> {
    let binding: PathBuf = if let Some(p) = path {
        p.into()
    } else {
        env::current_dir().unwrap()
    };

    let mountpoint = binding.to_str().unwrap();

    let fd = match open(mountpoint, OFlag::O_RDONLY, Mode::empty()) {
        Ok(fd) => fd,
        Err(err) => {
            eprintln!("ERROR: Failed to open mountpoint {mountpoint}: {err}");
            exit(1);
        }
    };

    let raw_fd = fd.as_raw_fd();

    let counters = match read_counters(raw_fd) {
        Ok(counters) => counters,
        Err(err) => {
            eprintln!("ERROR: Failed to read counters: {err}");
            eprintln!("ERROR: {mountpoint} is most likely not apart of a bcachefs filesystem");
            close(raw_fd).unwrap();
            exit(1);
        }
    };

    let mut results = HashMap::new();

    unsafe {
        let counters_slice = std::slice::from_raw_parts(counters.d.as_ptr(), counters.nr as usize);

        for (name, value) in zip(BCH2_COUNTER_NAMES, counters_slice) {
            results.insert(name.to_string(), *value);
        }
    }
    close(fd).unwrap();
    results
}
