use crate::stack_string::StackString;
use device_common::Packet;
use libc::*;
use std::mem;
use std::mem::size_of;
use std::ptr::null_mut;
use std::time::Duration;

pub type ResourceID = libc::c_int;

pub type FilePath = StackString<128>;

macro_rules! wrappers_assert {
    ($cond:expr, $($arg:tt)+) => {
        if !$cond {
            let errno = unsafe { *__errno_location() };
            panic!(
                "Error: wrappers::{} (errno: {})",
                format_args!($($arg)+),
                errno,
            );
        }
    };
}

pub struct PathnameServer {
    fd: ResourceID,
}

impl PathnameServer {
    pub fn new(file: &FilePath, queue_depth: usize) -> Self {
        let fd = unsafe { socket(AF_UNIX, SOCK_STREAM, 0) };
        wrappers_assert!(fd >= 0, "PathnameServer::new {}", "socket");
        unsafe {
            unlink(file.as_str().as_bytes().as_ptr() as *const c_char);
        }
        let mut address = sockaddr_un {
            sun_family: AF_UNIX as u16,
            sun_path: [0; 108],
        };
        let len = file.len();
        wrappers_assert!(len < 108, "PathnameServer::new {}", "file.len");
        file.as_str().as_bytes().iter().take(len).enumerate().for_each(|(i, &b)| address.sun_path[i] = b as c_char);
        let res = unsafe { bind(fd, &address as *const _ as *const _, size_of::<sockaddr_un>() as socklen_t) };
        wrappers_assert!(res == 0, "PathnameServer::new {}", "bind");
        let res = unsafe { chmod(file.as_str().as_bytes().as_ptr() as *const c_char, 0x666) };
        wrappers_assert!(res == 0, "PathnameServer::new {}", "chmod");
        let res = unsafe { listen(fd, queue_depth as i32) };
        wrappers_assert!(res == 0, "PathnameServer::new {}", "listen");
        let flags = unsafe { fcntl(fd, F_GETFL) };
        let res = unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) };
        wrappers_assert!(res == 0, "PathnameServer::new {}", "fcntl");
        Self { fd }
    }

    pub fn accept_connection(&self) -> Option<Connection> {
        let fd = unsafe { accept(self.fd, null_mut(), null_mut()) };
        let errno = unsafe { *__errno_location() };
        wrappers_assert!(
            fd >= 0 || (fd < 0 && errno == EWOULDBLOCK),
            "PathnameServer.accept_connection"
        );
        if fd >= 0 { Some(Connection { fd }) } else { None }
    }

    pub fn get_fd(&self) -> ResourceID {
        self.fd
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct Connection {
    fd: ResourceID,
}

impl Connection {
    pub fn receive(&self) -> Option<Packet> {
        let mut packet = Packet::default();
        if read(self.fd, &mut packet) == packet.len() {
            Some(packet)
        } else {
            None
        }
    }

    pub fn send(&self, packet: &Packet) {
        write(self.fd, packet);
    }

    pub fn close(&self) {
        close(self.fd);
    }

    pub fn get_fd(&self) -> ResourceID {
        self.fd
    }
}

pub struct KEventNetlinkSocket {
    fd: ResourceID,
}

impl KEventNetlinkSocket {
    pub fn new() -> Self {
        let fd = unsafe { socket(AF_NETLINK, SOCK_RAW, NETLINK_KOBJECT_UEVENT) };
        wrappers_assert!(fd >= 0, "KEventNetlinkSocket::new {}", "socket");
        let mut socket_addr: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
        socket_addr.nl_family = AF_NETLINK as sa_family_t;
        socket_addr.nl_pid = 0;
        socket_addr.nl_groups = 1;
        let addr_ptr = &socket_addr as *const sockaddr_nl as *const sockaddr;
        let addr_len = size_of::<sockaddr_nl>() as socklen_t;
        let res = unsafe { bind(fd, addr_ptr, addr_len) };
        wrappers_assert!(res == 0, "KEventNetlinkSocket::new {}", "bind");
        Self { fd }
    }

    pub fn read(&self, message: &mut [u8]) -> usize {
        read(self.fd, message)
    }

    pub fn get_fd(&self) -> ResourceID {
        self.fd
    }
}

pub struct File {
    fd: ResourceID,
}

impl File {
    pub fn exist(file: &FilePath) -> bool {
        unsafe { access(file.as_str().as_bytes().as_ptr() as *const c_char, F_OK) == 0 }
    }

    pub fn delete(file: &FilePath) {
        let res = unsafe { unlink(file.as_str().as_bytes().as_ptr() as *const c_char) };
        wrappers_assert!(res == 0, "File::delete");
    }

    pub fn open(file: &FilePath, mode: OpenMode) -> Self {
        let fd = unsafe { libc::open(file.as_str().as_bytes().as_ptr() as *const c_char, mode as c_int) };
        wrappers_assert!(fd >= 0, "File::open");
        Self { fd }
    }

    pub fn lock(&self) -> bool {
        let res = unsafe { flock(self.fd, LOCK_EX | LOCK_NB) };
        let errno = unsafe { *__errno_location() };
        wrappers_assert!(res == 0 || errno == EWOULDBLOCK, "File.lock");
        res == 0
    }

    pub fn seek(&self, position: u64) {
        let res = unsafe { lseek(self.fd, position as off_t, SEEK_SET) };
        wrappers_assert!(res >= 0, "File.seek");
    }

    pub fn read(&self, bytes: &mut [u8]) -> usize {
        read(self.fd, bytes)
    }

    pub fn write(&self, bytes: &[u8]) {
        write(self.fd, bytes)
    }

    pub fn close(&self) {
        close(self.fd);
    }
}

#[repr(i32)]
pub enum OpenMode {
    ReadOnly = O_RDONLY,
    ReadWrite = O_RDWR,
    Create = O_RDWR | O_CREAT,
}

fn read(fd: ResourceID, bytes: &mut [u8]) -> usize {
    let len = bytes.len();
    let count = unsafe { libc::read(fd, bytes.as_mut_ptr() as *mut c_void, len) };
    let errno = unsafe { *__errno_location() };
    wrappers_assert!(count >= 0 || (count == -1 && errno == ECONNRESET), "read");
    count as usize
}

fn write(fd: ResourceID, bytes: &[u8]) {
    let len = bytes.len();
    let count = unsafe { libc::write(fd, bytes.as_ptr() as *const c_void, len) };
    wrappers_assert!(count >= 0_isize, "write");
}

fn close(fd: ResourceID) {
    unsafe {
        libc::close(fd);
    }
}

pub struct EPoll {
    fd: ResourceID,
}

#[repr(i32)]
pub enum EPollAction {
    Add = EPOLL_CTL_ADD,
    Remove = EPOLL_CTL_DEL,
}

impl EPoll {
    pub fn new() -> Self {
        let fd = unsafe { epoll_create1(0) };
        wrappers_assert!(fd >= 0, "EPoll::new");
        Self { fd }
    }

    pub fn control(&self, action: EPollAction, fd: ResourceID) {
        let mut epoll_event = epoll_event {
            events: EPOLLIN as u32,
            u64: fd as u64,
        };
        let res = unsafe { epoll_ctl(self.fd, action as c_int, fd, &mut epoll_event) };
        wrappers_assert!(res == 0, "EPoll.control");
    }

    pub fn wait(&self) -> Option<ResourceID> {
        let mut events = [epoll_event { events: 0, u64: 0 }; 1];
        let count = unsafe { libc::epoll_wait(self.fd, events.as_mut_ptr(), events.len() as c_int, -1) };
        let errno = unsafe { *__errno_location() };
        wrappers_assert!(count >= 0 || errno == EINTR, "EPoll.wait");
        if count > 0 { Some(events[0].u64 as ResourceID) } else { None }
    }
}

pub struct INotify {
    fd: ResourceID,
}

impl INotify {
    pub fn new() -> Self {
        let fd = unsafe { libc::inotify_init() };
        wrappers_assert!(fd >= 0, "INotify::new {}", "inotify_init");
        let flags = unsafe { fcntl(fd, F_GETFL, 0) };
        wrappers_assert!(flags >= 0, "INotify::new {}:{}", "fcntl", 1);
        let res = unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) };
        wrappers_assert!(res >= 0, "INotify::new {}:{}", "fcntl", 2);
        Self { fd }
    }

    pub fn add_file(&self, file: &FilePath) -> ResourceID {
        let fd = unsafe { inotify_add_watch(self.fd, file.as_str().as_bytes().as_ptr() as *const c_char, IN_MODIFY) };
        wrappers_assert!(fd >= 0, "INotify.add_file");
        fd
    }

    pub fn get_target_fd(&self) -> Option<ResourceID> {
        let mut event = [0u8; 16];
        let count = unsafe { libc::read(self.fd, event.as_mut_ptr() as *mut c_void, event.len()) };
        let errno = unsafe { *__errno_location() };
        wrappers_assert!(count >= 0 || errno == EAGAIN, "INotify.get_target_fd");
        let event = unsafe { &*(event.as_ptr() as *const inotify_event) };
        if count > 0 { Some(event.wd) } else { None }
    }

    pub fn get_fd(&self) -> ResourceID {
        self.fd
    }
}

pub struct TimerFD {
    fd: ResourceID,
}

impl TimerFD {
    pub fn new() -> Self {
        let fd = unsafe { timerfd_create(CLOCK_MONOTONIC, 0) };
        wrappers_assert!(fd >= 0, "TimerFD::new");
        Self { fd }
    }

    pub fn set(&self, duration: Duration) {
        let timer_spec = itimerspec {
            it_interval: timespec {
                tv_sec: duration.as_secs() as i64,
                tv_nsec: duration.subsec_nanos() as i64,
            },
            it_value: timespec {
                tv_sec: duration.as_secs() as i64,
                tv_nsec: duration.subsec_nanos() as i64,
            },
        };
        let res = unsafe { timerfd_settime(self.fd, 0, &timer_spec, null_mut()) };
        wrappers_assert!(res == 0, "TimerFD.set");
    }

    pub fn read_missed(&self) -> u64 {
        let mut expirations: u64 = 0;
        let res = unsafe { libc::read(self.fd, &mut expirations as *mut u64 as *mut c_void, size_of::<u64>()) };
        wrappers_assert!(res >= 0, "TimerFD.read_missed");
        expirations
    }

    pub fn get_fd(&self) -> ResourceID {
        self.fd
    }
}

pub fn running_as_root() -> bool {
    unsafe { geteuid() == 0 }
}

pub fn setup_signal_handler() -> ResourceID {
    let mut mask: sigset_t = unsafe { mem::zeroed() };
    unsafe {
        sigemptyset(&mut mask);
        sigaddset(&mut mask, SIGINT);
        sigaddset(&mut mask, SIGTERM);
    }
    let res = unsafe { sigprocmask(SIG_BLOCK, &mask, null_mut()) };
    wrappers_assert!(res == 0, "setup_signal_handler {}", "sigprocmask");
    let fd = unsafe { signalfd(-1, &mask, 0) };
    wrappers_assert!(fd >= 0, "setup_signal_handler {}", "signalfd");
    fd
}

pub fn process_signal(signal_fd: ResourceID) -> bool {
    let mut si: signalfd_siginfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<signalfd_siginfo>();
    let count = unsafe { libc::read(signal_fd, &mut si as *mut _ as *mut c_void, size) };
    wrappers_assert!(count == size as isize, "process_signal");
    !(si.ssi_signo as c_int == SIGINT || si.ssi_signo as c_int == SIGTERM)
}
