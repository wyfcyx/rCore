use crate::ALLOCATOR;
use alloc::string::String;

#[inline(always)]
fn sys_call(
    syscall_id: SyscallId,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> i32 {
    let id = syscall_id as usize;
    let mut ret: i32;
    let failed: i32;

    unsafe {
        #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
        llvm_asm!("ecall"
            : "={x10}" (ret)
            : "{x17}" (id), "{x10}" (arg0), "{x11}" (arg1), "{x12}" (arg2), "{x13}" (arg3), "{x14}" (arg4), "{x15}" (arg5)
            : "memory"
            : "volatile");
        #[cfg(target_arch = "x86")]
        llvm_asm!("int 0x80"
            : "={eax}" (ret)
            : "{eax}" (id), "{edx}" (arg0), "{ecx}" (arg1), "{ebx}" (arg2), "{edi}" (arg3), "{esi}" (arg4)
            : "memory"
            : "intel" "volatile");
        #[cfg(target_arch = "x86_64")]
        llvm_asm!("syscall"
            : "={rax}" (ret)
            : "{rax}" (id), "{rdi}" (arg0), "{rsi}" (arg1), "{rdx}" (arg2), "{r10}" (arg3), "{r8}" (arg4), "{r9}" (arg5)
            : "rcx" "r11" "memory"
            : "intel" "volatile");
        #[cfg(target_arch = "aarch64")]
        llvm_asm!("svc 0"
            : "={x0}" (ret)
            : "{x8}" (id), "{x0}" (arg0), "{x1}" (arg1), "{x2}" (arg2), "{x3}" (arg3), "{x4}" (arg4), "{x5}" (arg5)
            : "memory"
            : "volatile");
        #[cfg(target_arch = "mips")]
        {
            llvm_asm!("syscall"
            // v0 for syscall id
            : "={$2}" (ret), "={$7}" (failed)
            // v0, a0, a1, a2, a3, a4, a5
            : "{$2}" (id), "{$4}" (arg0), "{$5}" (arg1), "{$6}" (arg2), "{$7}" (arg3), "{$8}" (arg4), "{$9}" (arg5)
            : "memory"
            : "volatile");
            if failed != 0 {
                ret = -ret;
            }
        }
    }
    ret
}

pub fn enlarge_heap() {
    const HEAP_SIZE: usize = 16 * 1024 * 1024;
    let addr = sys_mmap(0, HEAP_SIZE, 0x3, 0x22, 0, 0) as usize;
    unsafe {
        ALLOCATOR.lock().init(addr, HEAP_SIZE);
    }
}

pub fn sys_exit(code: usize) -> ! {
    sys_call(SyscallId::Exit, code, 0, 0, 0, 0, 0);
    unreachable!()
}

pub fn sys_exec(name: *const u8, argv: *const *const u8, envp: *const *const u8) -> i32 {
    sys_call(
        SyscallId::Exec,
        name as usize,
        argv as usize,
        envp as usize,
        0,
        0,
        0,
    )
}

pub fn sys_write(fd: usize, base: *const u8, len: usize) -> i32 {
    sys_call(SyscallId::Write, fd, base as usize, len, 0, 0, 0)
}

pub fn sys_read(fd: usize, base: *mut u8, len: usize) -> i32 {
    sys_call(SyscallId::Read, fd, base as usize, len, 0, 0, 0)
}

pub fn sys_open(path: &str, flags: usize) -> i32 {
    // UNSAFE: append '\0' to the string
    use core::mem::replace;
    let end = unsafe { &mut *(path.as_ptr().offset(path.len() as isize) as *mut u8) };
    let backup = replace(end, 0);
    const AT_FDCWD: isize = -100;
    let ret = sys_call(
        SyscallId::Openat,
        AT_FDCWD as usize,
        path.as_ptr() as usize,
        flags,
        0,
        0,
        0,
    );
    *end = backup;
    ret
}

pub fn sys_close(fd: usize) -> i32 {
    sys_call(SyscallId::Close, fd, 0, 0, 0, 0, 0)
}

pub fn sys_dup2(fd1: usize, fd2: usize) -> i32 {
    sys_call(SyscallId::Dup3, fd1, fd2, 0, 0, 0, 0)
}

/// Fork the current process. Return the child's PID.
pub fn sys_vfork() -> i32 {
    let mut sp: usize = 0;
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {
        llvm_asm!("mv $0, sp" : "=r" (sp) :: );
    }
    // TODO: more arch
    const CLONE_VFORK: usize = 0x00004000;
    const CLONE_VM: usize = 0x00000100;
    const SIGCHILD: usize = 17;
    sys_call(
        SyscallId::Clone,
        CLONE_VFORK | CLONE_VM | SIGCHILD,
        sp,
        0,
        0,
        0,
        0,
    )
}

/// Wait the process exit.
/// Return the PID. Store exit code to `code` if it's not null.
pub fn sys_wait(pid: usize, code: *mut i32) -> i32 {
    sys_call(SyscallId::Wait, pid, code as usize, 0, 0, 0, 0)
}

pub fn sys_yield() -> i32 {
    sys_call(SyscallId::Yield, 0, 0, 0, 0, 0, 0)
}

/// Kill the process
pub fn sys_kill(pid: usize) -> i32 {
    sys_call(SyscallId::Kill, pid, 0, 0, 0, 0, 0)
}

/// Get the current process id
pub fn sys_getpid() -> i32 {
    sys_call(SyscallId::GetPid, 0, 0, 0, 0, 0, 0)
}

/// Get the current working directory
pub fn sys_getcwd() -> String {
    let buffer = [0u8; 256];
    sys_call(SyscallId::GetCwd, buffer.as_ptr() as usize, 256, 0, 0, 0, 0);
    String::from_utf8(buffer.to_vec()).unwrap()
}

/// Change the current working directory
pub fn sys_chdir(path: &str) {
    let path = String::from(path) + "\0";
    sys_call(
        SyscallId::Chdir,
        path.as_bytes().as_ptr() as usize,
        0,
        0,
        0,
        0,
        0,
    );
}

/// Check file accessibility
pub fn sys_access(path: &str) -> i32 {
    let path = String::from(path) + "\0";
    sys_call(
        SyscallId::FAccessAt,
        -100isize as usize,
        path.as_bytes().as_ptr() as usize,
        0,
        0,
        0,
        0,
    )
}

pub fn sys_sleep(time: usize) -> i32 {
    let ts = TimeSpec {
        sec: time as u64,
        nsec: 0,
    };
    sys_call(
        SyscallId::Sleep,
        &ts as *const TimeSpec as usize,
        0,
        0,
        0,
        0,
        0,
    )
}

pub fn sys_get_time() -> i32 {
    sys_call(SyscallId::GetTime, 0, 0, 0, 0, 0, 0)
}

pub fn sys_set_priority(priority: usize) -> i32 {
    sys_call(SyscallId::SetPriority, priority, 0, 0, 0, 0, 0)
}

pub fn sys_arch_prctl(code: i32, addr: usize) -> i32 {
    sys_call(SyscallId::ArchPrctl, code as usize, addr, 0, 0, 0, 0)
}

pub fn sys_map_pci_device(vendor: usize, product: usize) -> i32 {
    sys_call(SyscallId::MapPciDevice, vendor, product, 0, 0, 0, 0)
}

pub fn sys_get_paddr(vaddr: &[u64], paddr: &mut [u64]) -> i32 {
    assert_eq!(vaddr.len(), paddr.len());
    sys_call(
        SyscallId::GetPaddr,
        vaddr.as_ptr() as usize,
        paddr.as_ptr() as usize,
        vaddr.len(),
        0,
        0,
        0,
    )
}

pub fn sys_mmap(
    addr: usize,
    len: usize,
    prot: usize,
    flags: usize,
    fd: usize,
    offset: usize,
) -> i32 {
    sys_call(SyscallId::Mmap, addr, len, prot, flags, fd, offset)
}

pub fn sys_socket(domain: usize, socket_type: usize, protocol: usize) -> i32 {
    sys_call(SyscallId::Socket, domain, socket_type, protocol, 0, 0, 0)
}

pub fn sys_setsockopt(fd: usize, level: usize, opt: usize, optval: usize, optlen: usize) -> i32 {
    sys_call(SyscallId::SetSockOpt, fd, level, opt, optval, optlen, 0)
}

pub fn sys_listen(fd: usize, backlog: usize) -> i32 {
    sys_call(SyscallId::Listen, fd, backlog, 0, 0, 0, 0)
}

pub fn sys_bind(fd: usize, addr: *const SockAddrIn, addr_len: usize) -> i32 {
    sys_call(SyscallId::Bind, fd, addr as usize, addr_len, 0, 0, 0)
}

pub fn sys_accept(fd: usize, addr: *mut SockAddrIn, addr_len: *mut u32) -> i32 {
    sys_call(
        SyscallId::Accept,
        fd,
        addr as usize,
        addr_len as usize,
        0,
        0,
        0,
    )
}

pub fn sys_recvfrom(
    fd: usize,
    base: *mut u8,
    len: usize,
    flags: usize,
    addr: *mut SockAddrIn,
    addr_len: *mut u32,
) -> i32 {
    sys_call(
        SyscallId::RecvFrom,
        fd,
        base as usize,
        len,
        flags,
        addr as usize,
        addr_len as usize,
    )
}

pub fn sys_sendto(
    fd: usize,
    base: *const u8,
    len: usize,
    flags: usize,
    addr: usize,
    addr_len: usize,
) -> i32 {
    sys_call(
        SyscallId::SendTo,
        fd,
        base as usize,
        len,
        flags,
        addr,
        addr_len,
    )
}

pub fn sys_ioctl(fd: usize, request: usize, arg1: usize) -> i32 {
    sys_call(SyscallId::Ioctl, fd, request, arg1, 0, 0, 0)
}

#[cfg(any(target_arch = "x86_64", target_arch = "mips"))]
pub fn sys_poll(ufds: *mut PollFd, nfds: usize, timeout: usize) -> i32 {
    sys_call(SyscallId::Poll, ufds as usize, nfds, timeout, 0, 0, 0)
}

pub fn sys_ppoll(ufds: *mut PollFd, nfds: usize, timeout: *const TimeSpec) -> i32 {
    sys_call(
        SyscallId::Ppoll,
        ufds as usize,
        nfds,
        timeout as usize,
        0,
        0,
        0,
    )
}

pub fn sys_epoll_create1(flags: usize) -> i32 {
    sys_call(SyscallId::EpollCreate1, flags, 0, 0, 0, 0, 0)
}

#[cfg(any(target_arch = "x86_64", target_arch = "mips"))]
pub fn sys_epoll_create(size: usize) -> i32 {
    sys_call(SyscallId::EpollCreate, size, 0, 0, 0, 0, 0)
}

pub fn sys_epoll_ctl(epfd: usize, op: usize, fd: usize, event: *const EpollEvent) -> i32 {
    sys_call(SyscallId::EpollCtl, epfd, op, fd, event as usize, 0, 0)
}

pub fn sys_epoll_pwait(
    epfd: usize,
    events: *mut EpollEvent,
    maxevents: usize,
    timeout: usize,
    sigmask: usize,
) -> i32 {
    sys_call(
        SyscallId::EpollPwait,
        epfd,
        events as usize,
        maxevents,
        timeout,
        sigmask,
        0,
    )
}

#[cfg(any(target_arch = "x86_64", target_arch = "mips"))]
pub fn sys_epoll_wait(
    epfd: usize,
    events: *mut EpollEvent,
    maxevents: usize,
    timeout: usize,
) -> i32 {
    sys_call(
        SyscallId::EpollWait,
        epfd,
        events as usize,
        maxevents,
        timeout,
        0,
        0,
    )
}

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
enum SyscallId {
    Read = 0,
    Write = 1,
    Close = 3,
    Fstat = 4,
    Seek = 8,
    Mmap = 9,
    Munmap = 11,
    Ioctl = 16,
    Yield = 24,
    Sleep = 35,
    GetPid = 39,
    Socket = 41,
    Accept = 43,
    SendTo = 44,
    Bind = 49,
    Listen = 50,
    RecvFrom = 45,
    SetSockOpt = 54,
    Clone = 56,
    Exec = 59,
    Exit = 60,
    Wait = 61,
    Kill = 62,
    Fsync = 74,
    GetCwd = 79,
    Chdir = 80,
    GetTime = 96,
    SetPriority = 141,
    ArchPrctl = 158,
    GetDirEntry64 = 217,
    Openat = 257,
    FAccessAt = 269,
    Dup3 = 292,
    // custom
    MapPciDevice = 999,
    GetPaddr = 998,

    Poll = 7,
    Ppoll = 271,
    Select = 23,
    Pselect6 = 270,
    EpollCreate = 213,
    EpollCreate1 = 291,
    EpollWait = 232,
    EpollPwait = 281,
    EpollCtl = 233,
}

#[cfg(target_arch = "mips")]
pub fn sys_set_theaad_area(tls: usize) {
    sys_call(SyscallId::SetThreadArea, tls, 0, 0, 0, 0, 0);
}

// only for mips N32 abi
// see https://git.linux-mips.org/cgit/ralf/linux.git/tree/arch/mips/include/uapi/asm/unistd.h
#[cfg(target_arch = "mips")]
#[allow(dead_code)]
enum SyscallId {
    Read = 4003,
    Write = 4004,
    Close = 4006,
    Fstat = 4198,
    Seek = 4018,
    Mmap = 4090,
    Munmap = 4091,
    Ioctl = 4054,
    Yield = 4162,
    Sleep = 4166,
    GetPid = 4020,
    Socket = 4183,
    Listen = 4174,
    SendTo = 4180,
    RecvFrom = 4176,
    SetSockOpt = 4181,
    Bind = 4169,
    Accept = 4168,
    Clone = 4120,
    Exec = 4011,
    Exit = 4001,
    Wait = 4114,
    Kill = 4037,
    Fsync = 4118,
    GetCwd = 4203,
    Chdir = 4012,
    GetTime = 4078,
    SetPriority = 4097,
    SetThreadArea = 4283,
    ArchPrctl = -1,
    GetDirEntry64 = 4219,
    Openat = 4288,
    FAccessAt = 4300,
    Dup3 = 4327,
    // custom
    MapPciDevice = 999,
    GetPaddr = 998,

    Poll = 4188,
    Ppoll = 4302,
    Pselect6 = 4301,
    EpollCreate = 4248,
    EpollCreate1 = 4326,
    EpollWait = 4250,
    EpollPwait = 4313,
    EpollCtl = 4249,
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "mips")))]
#[allow(dead_code)]
enum SyscallId {
    Read = 63,
    Write = 64,
    Close = 57,
    Fstat = 80,
    Seek = 62,
    Mmap = 222,
    Munmap = 215,
    Ioctl = 29,
    Yield = 124,
    Socket = 198,
    SendTo = 206,
    Bind = 200,
    Listen = 201,
    Accept = 202,
    RecvFrom = 207,
    SetSockOpt = 208,
    Sleep = 101,
    GetPid = 172,
    Clone = 220,
    Exec = 221,
    Exit = 93,
    Wait = 260,
    Kill = 129,
    Fsync = 82,
    GetCwd = 17,
    Chdir = 49,
    GetTime = 169,
    SetPriority = 140,
    ArchPrctl = -4,
    GetDirEntry64 = 61,
    Openat = 56,
    Dup3 = 24,
    FAccessAt = 48,
    // custom
    MapPciDevice = 999,
    GetPaddr = 998,

    Ppoll = 73,
    Pselect6 = 72,
    EpollCreate1 = 20,
    EpollCtl = 21,
    EpollPwait = 22,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TimeSpec {
    pub sec: u64,
    pub nsec: u64,
}

#[repr(C)]
#[derive(Debug)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

#[repr(C)]
#[derive(Debug)]
pub struct PollFd {
    pub fd: u32,
    pub events: u16,
    pub revents: u16,
}

#[repr(C)]
pub union EPOLLDATAT {
    pub ptr: u64,
    pub fd: i32,
    pub v32: u32,
    pub v64: u64,
}

#[repr(packed)]
pub struct EpollEvent {
    pub events: u32,      /* Epoll events */
    pub data: EPOLLDATAT, /* User data variable */
}

impl core::default::Default for EpollEvent {
    fn default() -> EpollEvent {
        EpollEvent {
            events: 0,
            data: EPOLLDATAT { ptr: 0 },
        }
    }
}

impl EpollEvent {
    pub const IN: u32 = 0x001;
    pub const PRI: u32 = 0x002;
    pub const OUT: u32 = 0x004;
    pub const RDNORM: u32 = 0x040;
    pub const RDBAND: u32 = 0x080;
    pub const WRNORM: u32 = 0x100;
    pub const WRBAND: u32 = 0x200;
    pub const MSG: u32 = 0x400;
    pub const ERR: u32 = 0x008;
    pub const HUP: u32 = 0x010;
    pub const RDHUP: u32 = 0x2000;
    pub const EXCLUSIVE: u32 = 1 << 28;
    pub const WAKEUP: u32 = 1 << 29;
    pub const ONESHOT: u32 = 1 << 30;
    pub const ET: u32 = 1 << 31;
}
