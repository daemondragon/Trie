extern {
    #[cfg_attr(all(target_os = "macos", target_arch = "x86"), link_name = "setrlimit$UNIX2003")]
    fn setrlimit(resource: i32, rlim: *const rlimit) -> i32;
}

#[repr(C)]
struct rlimit {
    rlim_cur: u64,
    rlim_max: u64,
}

/// All limits that can be placed on the process.
/// A limit work affect the process even after going out of scope,
/// and can't be reverted (other than applying another bigger limit).
pub enum Limit {
    /// How many RAM memory in bytes the process can use.
    /// The quantity of virtual memory is not affected by this.
    Memory(u64),
    /// How much CPU times in seconds the process can use.
    CPUTime(u64),
}

impl Limit {
    /// Apply the limit and return if it could be applied or not.
    pub fn apply(self) -> bool {
        let (max, kind) = match self {
            Limit::Memory(value)  => (value, 2/* RLIMIT_DATA */),
            Limit::CPUTime(value) => (value, 0/* RLIMIT_CPU */),
        };

        let limit = rlimit { rlim_cur: max, rlim_max: max };
        unsafe { setrlimit(kind, &limit) == 0 }
    }
}