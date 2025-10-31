use std::mem::MaybeUninit;
use crate::sys_libnfqws::{gid_t, hostlist_files_head, ip_cache, ipset_files_head, log_target, sockaddr_in, sockaddr_in6, uid_t, wordexp_t, IF_NAMESIZE};
use crate::utils::str_to_c_array;
use pastey::paste;
use std::ffi::c_uint;
use std::net::IpAddr;
use std::os::raw::{c_char, c_int};
use std::time::Duration;
use bitflags::bitflags;
use derive_new::new;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("value '{0}' required, but not present")]
    MissingRequiredValue(&'static str)
}

trait AssertFromImplemented<T>: From<T> {}

/// The `make_params!` macro is used to generate parameter structs in two "modes":
/// - **public** — a struct intended for use in Rust code.
/// - **ffi** — a "raw" struct suitable for FFI.
///
/// **Important:** You must implement the conversion from `MyParams` to `RawMyParams`
/// yourself, for example via `From<MyParams>` or a custom method, to map the fields
/// from the public struct to the raw FFI struct.
macro_rules! make_params {
    (
        $name:ident {
            public: {
                $(
                    $(#[doc = $doc:literal])+
                    $(#[cfg($cfg:meta)])?
                    $param_name:ident : $ty:ty, $none_action:ident $(, $default:expr)?;
                )*
            },
            ffi: {
                $(
                    $(#[cfg($ffi_cfg:meta)])?
                    $ffi_param_name:ident : $ffi_ty:ty;
                )*
            } $(,)?
        }
    ) => { paste! {
        #[allow(dead_code)]
        pub struct $name {
            inner: [<$name Items>]
        }

        #[derive(Clone, Default)]
        #[allow(dead_code)]
        #[doc = concat!("A builder for [`", stringify!($name) ,"`]")]
        $(
            #[doc = concat!(
                "* [`",
                stringify!($param_name),
                "`](",
                make_params!{ @type_str $ty, $none_action },
                "):",
                make_params!{ @join_doc [$($doc)+] }, "\n  ",
                make_params!{ @build_doc $none_action, $($default)? },
                $( "\n  _Available only if_ `cfg(", stringify!($cfg), ")`" )?
            )]
        )*
        pub struct [<$name Builder>] {
            $(
                $(#[cfg($cfg)])?
                $param_name: Option<$ty>,
            )*
        }

        struct [<$name ItemsIntermediate>] {
            $(
                $(#[cfg($cfg)])?
                $param_name: MaybeUninit<make_params!{ @type $ty, $none_action }>,
            )*
        }

        impl [<$name ItemsIntermediate>] {
            fn assume_init(self) -> [<$name Items>] {
                unsafe {
                    [<$name Items>] {
                        $(
                            $(#[cfg($cfg)])?
                            $param_name: self.$param_name.assume_init(),
                        )*
                    }
                }
            }
        }

        impl Default for [<$name ItemsIntermediate>] {
            fn default() -> Self {
                Self {
                    $(
                        $(#[cfg($cfg)])?
                        $param_name: MaybeUninit::uninit(),
                    )*
                }
            }
        }

        #[allow(dead_code)]
        impl [<$name Builder>] {
            $(
                make_params!{ @make_set $($doc)+, $($cfg:meta)?, $param_name, $ty, $none_action }
            )*

            pub fn new() -> Self {
                Default::default()
            }

            pub fn build(&self) -> Result<$name, BuildError> {
                let mut intermediate = [<$name ItemsIntermediate>]::default();

                $(
                    $(#[cfg($cfg)])?
                    intermediate.$param_name = MaybeUninit::new(make_params!{ @build $name, $param_name, self.$param_name.clone(), &intermediate, $none_action, $($default)? });
                )*

                let config = $name {
                    inner: intermediate.assume_init()
                };

                Ok(config)
            }
        }

        #[derive(Clone)]
        #[allow(dead_code)]
        struct [<$name Items>] {
            $(
                $(#[cfg($cfg)])?
                $param_name: make_params!{@type $ty, $none_action},
            )*
        }

        #[allow(unused)]
        #[allow(dead_code)]
        impl $name {
            $(
                $(#[doc = $doc])+
                $(#[cfg($cfg)])?
                pub fn $param_name(&self) -> &make_params!{@type $ty, $none_action} {
                    &self.inner.$param_name
                }
            )*

            pub fn builder() -> [<$name Builder>] {
                [<$name Builder>]::new()
            }
        }

        #[repr(C)]
        #[allow(dead_code)]
        pub struct [<Raw $name>] {
            $(
                $(#[cfg($ffi_cfg)])?
                pub $ffi_param_name: $ffi_ty,
            )*
        }

        impl AssertFromImplemented<$name> for [<Raw $name>] {}
    }};

    ( @join_doc [$($docs:literal)+] ) => {
        concat!($($docs, "\n  ",)+)
    };

    ( @type_str $ty:ty, option ) => { concat!("Option<", stringify!($ty), ">") };
    ( @type_str $ty:ty, $id:ident ) => { stringify!($ty) };

    ( @type $ty:ty, option ) => { Option<$ty> };
    ( @type $ty:ty, $id:ident ) => { $ty };

    ( @make_set $($doc:literal)+, $($cfg:meta)?, $param_name:ident, $ty:ty, generated) => {
        // Cause generated, not able to set
    };

    ( @make_set $($doc:literal)+, $($cfg:meta)?, $param_name:ident, $ty:ty, $id:ident) => {
        paste! {
            $(#[doc = $doc])+
            $(#[cfg($cfg)])?
            pub fn $param_name(mut self, value: $ty) -> Self {
                self.[<$param_name _mut>](value);
                self
            }

            $(#[doc = $doc])+
            $(#[cfg($cfg)])?
            pub fn [<$param_name _mut>](&mut self, value: $ty) -> &mut Self {
                self.$param_name = Some(value);
                self
            }
        }
    };

    ( @build_doc option, ) => { "Optional value, not required, default: [`None`]" };
    ( @build_doc def, $default:expr ) => { concat!("Not required, default: `", stringify!($default), "`") };
    ( @build_doc def_itself, ) => { "Not required, default: [`Default::default()`]" };
    ( @build_doc required, ) => { "**Required**" };
    ( @build_doc auto, $($default:expr)? ) => { "Auto, if not preset it will be set to default, but it depended on current builder state" };
    ( @build_doc generated, $($default:expr)? ) => { "Generated, not able to set" };

    ( @build $name:ident, $param_name:ident, $value:expr, $intermediate:expr, option, ) => { $value };
    ( @build $name:ident, $param_name:ident, $value:expr, $intermediate:expr, def, $default:expr ) => { $value.unwrap_or($default) };
    ( @build $name:ident, $param_name:ident, $value:expr, $intermediate:expr, def_itself, ) => { $value.unwrap_or_default() };
    ( @build $name:ident, $param_name:ident, $value:expr, $intermediate:expr, required, ) => {
        paste! {
            $value.ok_or(BuildError::MissingRequiredValue(stringify!($param_name)))?
        }
    };

    ( @build $name:ident, $param_name:ident, $value:expr, $intermediate:expr, auto, $default_fn:expr ) => {{
        paste! {
            match $value {
                Some(v) => v,
                None => {
                    let f: &dyn Fn(&[<$name ItemsIntermediate>]) -> _ = &$default_fn;
                    f($intermediate)
                }
            }
        }
    }};

    ( @build $name:ident, $param_name:ident, $value:expr, $intermediate:expr, generated, $default_fn:expr ) => {{
        paste! {
            let f: &dyn Fn(&[<$name ItemsIntermediate>]) -> _ = &$default_fn;
            f($intermediate)
        }
    }};
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BindLl {
    Unwanted = 0,
    No = 1,
    Prefer = 2,
    Force = 3,
}

make_params!(Bind {
    public: {
        /// The IP address to bind to.
        addr: IpAddr, required;

        /// The network interface name.
        iface: String, required;

        /// Link-layer information
        ll: BindLl, required;

        /// Wait for the interface to be up before use (can be a timeout or logical flag).
        wait_ifup: i32, def_itself;

        /// Wait for the IP address to appear on the interface.
        wait_ip: i32, def_itself;

        /// Wait for a link-local address (for example, for IPv6).
        wait_ip_ll: i32, def_itself;
    },
    ffi: {
        bindaddr: [c_char; 64usize];
        bindiface: [c_char; IF_NAMESIZE as usize];
        bindll: BindLl;
        bind_wait_ifup: c_int;
        bind_wait_ip: c_int;
        bind_wait_ip_ll: c_int;
    }
});

impl From<Bind> for RawBind {
    fn from(bind: Bind) -> Self {
        const IF_NAMESIZE_USIZE: usize = IF_NAMESIZE as usize;

        let bindaddr = str_to_c_array::<64>(&bind.addr().to_string());
        let bindiface = str_to_c_array::<IF_NAMESIZE_USIZE>(bind.iface());

        RawBind {
            bindaddr,
            bindiface,
            bindll: *bind.ll(),
            bind_wait_ifup: *bind.wait_ifup() as c_int,
            bind_wait_ip: *bind.wait_ip() as c_int,
            bind_wait_ip_ll: *bind.wait_ip_ll() as c_int,
        }
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct BindFix: u8 {
        const V1 = 1 << 1;
        const V2 = 1 << 2;
    }
}

#[derive(Debug, Clone, new)]
pub struct ConnTrackTimeouts {
    pub syn: Duration,
    pub established: Duration,
    pub fin: Duration,
    pub udp: Option<Duration>
}

impl Default for ConnTrackTimeouts {
    fn default() -> Self {
        ConnTrackTimeouts::new(
            Duration::from_mins(1),
            Duration::from_mins(5),
            Duration::from_mins(1),
            Some(Duration::from_mins(1))
        )
    }
}

#[derive(Debug, Clone, new)]
pub struct WindowSize {
    pub size: u32,
    pub scale_factor: Option<u32>
}

impl WindowSize {
    pub fn size(size: u32) -> Self {
        Self {
            size,
            scale_factor: Some(0)
        }
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct WindowSizeCutoff: u8 {
        const OutgointPackets = 1 << 1;
        const DataPackets = 1 << 2;
        const RelativeSequences = 1 << 3;
    }
}

#[derive(Debug, Clone)]
pub enum SynackSplit {
    Sny,
    SynAck,
    AckSyn
}

#[derive(Debug, Clone, new)]
pub struct AutoTTL {
    pub delta: i32,
    pub min: Option<u32>,
    pub max: Option<u32>
}

impl Default for AutoTTL {
    fn default() -> Self {
        AutoTTL::new(5, Some(3), Some(64))
    }
}

impl From<Params> for RawParams {
    fn from(_value: Params) -> Self {
        todo!()
    }
}

make_params!(Params {
    public: {
        /// Trying to solve the problem of incorrect selection
        /// of outgoing interface for generated ip packets
        bind_fix: BindFix, option;

        /// Number of the queue
        queue: u16, option;

        /// Timeouts for internal connection tacker
        connection_tracker_timeouts: ConnTrackTimeouts, def_itself;

        /// Change tcp window size in outgoing packets
        window_size: WindowSize, option;

        /// Change server window size in outgoing source packets,
        /// data packets, relative to a sequence number less than N
        window_size_cutoff: (WindowSizeCutoff, u32), option;

        /// Automatically disable window size when
        /// a known protocol is detected
        window_size_forced_cutoff: bool, def, true;

        /// Internal connection tracker enabled or disabled
        connection_tracker: bool, def, false;

        /// Ip cache lifetime. [`None`] - without limit
        ipcache_lifetime: Duration, option;

        /// Enable hostname caching for use in phase zero strategies
        ipcache_hostname: bool, def, true;

        /// Do tcp handshake
        /// Instead of SYN,ACK send only SYN, SYN+ACK or ACK+SYN
        synack_split: SynackSplit, option;

        /// Modify original packet TTL
        modified_tll: u16, option;

        /// Modify the original IPv6 packets hop limit.
        /// If not provided, [`modified_ttl`] will be used
        modified_ttl_ip_v6: u16, option;

        /// Auto TTL mode for ipv4
        modified_auto_ttl: AutoTTL, def_itself;

        /// Auto TTL mode, only for ipv6
        /// If not set, [`modified_auto_ttl`] will be used
        modifier_auto_ttl_ip_v6: AutoTTL, auto, |ir| {
            // Safety: modified_auto_ttl already initialized
            unsafe { ir.modified_auto_ttl.assume_init_ref().clone() }
        };

        /// Change process uid
        user: String, option;

        /// Change process uid
        uid: u16, option;

        /// Change process gid
        gid: u16, option;
    },
    ffi: {
        #[cfg(not(any(target_os = "openbsd", target_os = "android")))]
        wexp: wordexp_t;

        debug_target: log_target;
        debug_logfile: [c_char; 4096usize];
        binds: [RawBind; 32];
        binds_last: c_int;
        bind_wait_only: bool;
        port: u16;
        connect_bind4: sockaddr_in;
        connect_bind6: sockaddr_in6;
        connect_bind6_ifname: [c_char; IF_NAMESIZE as usize];

        proxy_type: u8;
        fix_seg: c_uint;
        fix_seq_avial: bool;
        no_resolve: bool;
        skip_nodelay: bool;
        daemon: bool;
        droproot: bool;
        user: *mut c_char;
        uid: uid_t;
        gid: [gid_t; 64usize];
        gid_count: c_int;
        pdfile: [c_char; 4096usize];

        maxconn: c_int;
        resolver_threads: c_int;
        maxfiles: c_int;
        max_orphan_time: c_int;

        local_rcvbuf: c_int;
        local_sndbuf: c_int;
        remote_rcvbuf: c_int;
        remote_sndbuf: c_int;

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        tcp_user_timeout_local: c_int;

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        tcp_user_timeout_remote: c_int;

        #[cfg(target_os = "freebsd")]
        pf_enable: bool;

        #[cfg(target_os = "linux")]
        nosplice: bool;

        ttl_default: c_int;
        hostlist_auto_debuglog: [c_char; 4096usize];
        hostlists: hostlist_files_head;
        ipsets: ipset_files_head;
        tamper: bool;
        tamper_lim: bool;

        ipcache_lifetime: c_uint;
        cache_hostname: bool;
        ipcache: ip_cache;
    },
});
