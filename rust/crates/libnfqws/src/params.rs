use crate::sys_libnfqws::{gid_t, hostlist_files_head, ip_cache, ipset_files_head, log_target, sockaddr_in, sockaddr_in6, uid_t, wordexp_t, IF_NAMESIZE};
use crate::utils::str_to_c_array;
use pastey::paste;
use std::ffi::c_uint;
use std::net::IpAddr;
use std::os::raw::{c_char, c_int};

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
        #[doc = concat!("A builder for [`",stringify!($name) ,"`]")]
        $(
            #[doc = concat!(
                "* [`",
                stringify!($param_name),
                "`](",
                make_params!{@type_str $ty, $none_action},
                "):",
                make_params!{@join_doc [$($doc)+]},
                make_params!{ @build_doc $none_action, $($default)? },
                $( "\n  _Available only if_ `cfg(", stringify!($cfg), ")`" )?
            )]
        )*
        struct [<$name Builder>] {
            $(
                $(#[cfg($cfg)])?
                $param_name: Option<$ty>,
            )*
        }

        #[allow(dead_code)]
        impl [<$name Builder>] {
            $(
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
            )*

            fn new() -> Self {
                Default::default()
            }

            fn build(&self) -> $name {
                let config = [<$name Items>] {
                    $(
                        $(#[cfg($cfg)])?
                        $param_name: make_params!{ @build $param_name, self.$param_name.clone(), &config, $none_action, $($default)? },
                    )*
                };

                $name {
                    inner: config
                }
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

            fn builder() -> [<$name Builder>] {
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
    }};

    (@join_doc [$($docs:literal)+]) => {
        concat!($($docs, "\n  ")+)
    };

    (@type_str $ty:ty, option) => { "Option<". stringify!($ty) .">" };
    (@type_str $ty:ty, $id:ident) => { stringify!($ty) };

    ( @type $ty:ty, option) => { Option<$ty> };
    ( @type $ty:ty, $id:ident) => { $ty };

    ( @build_doc option, ) => { "Optional value, not required, default: None" };
    ( @build_doc def, $default:expr ) => { concat!("Not required, default: `", stringify!($default), "`") };
    ( @build_doc def_itself, ) => { "Not required, default: [`Default::default()`]" };
    ( @build_doc required, ) => { "**Required**" };

    ( @build $name:ident, $value:expr, $config:expr, option, ) => { $value };
    ( @build $name:ident, $value:expr, $config:expr, def, $default:expr ) => { $value.unwrap_or($default) };
    ( @build $name:ident, $value:expr, $config:expr, def_itself, ) => { $value.unwrap_or_default() };
    ( @build $name:ident, $value:expr, $config:expr, required, ) => {
        $value.expect(concat!("Value ", stringify!($name), " is required, but not present."))
    };
    ( @build $name:ident, $value:expr, $config:expr, auto, $default_fn:expr ) => {{
        paste! {
            match $value {
                Some(v) => v,
                None => {
                    let f: &dyn Fn(&[<$name Items>]) -> _ = &$default_fn;
                    f($config)
                }
            }
        }
    }};
    ( @build $name:ident, $value:expr, $config:expr, generated, $default_fn:expr ) => {{
        paste! {
            let f: &dyn Fn(&[<$name Items>]) -> _ = &$default_fn;
            f($config)
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

make_params!(Params {
    public: {
        /// Data folder |> Main data folder
        data_folder: String, def, "data".to_string();
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
