extern crate x11_dl;
extern crate hostname;
extern crate libc;
extern crate gl;

use std::ffi::CString;
use std::io::Write;
use std::mem;
use std::os::raw::*;
use std::ptr;

use x11_dl::xlib;
use x11_dl::glx;

macro_rules! cstr {
  ($s:expr) => (
    concat!($s, "\0") as *const str as *const [c_char] as *const c_char
  );
}

unsafe fn set_wm_pid(xlib: &xlib::Xlib, dpy: *mut xlib::Display, win: c_ulong) {
    match hostname::get_hostname() {
        None => writeln!(&mut std::io::stderr(), "Couldn't get hostname. Skipping _NET_WM_PID").unwrap(),
        Some(name) => {
            let mut host_name : (xlib::XTextProperty) = mem::uninitialized();

            let name_str = CString::new(name).unwrap();
            let mut name_ptr = name_str.into_raw();
            (xlib.XStringListToTextProperty)(&mut name_ptr as *mut *mut c_char, 1, &mut host_name);
            (xlib.XSetWMClientMachine)(dpy, win, &mut host_name);
            (xlib.XFree)(host_name.value as *mut std::os::raw::c_void);

            let pid = libc::getpid();

            (xlib.XChangeProperty)(dpy, win,
                                   (xlib.XInternAtom)(dpy, cstr!("_NET_WM_PID"), xlib::False),
                                   (xlib.XInternAtom)(dpy, cstr!("CARDINAL"), xlib::False),
                                   32, xlib::PropModeReplace, &pid as *const i32 as *const c_uchar, 1);
        }
    }
}

unsafe fn mod_atom(xlib: &xlib::Xlib, dpy: *mut xlib::Display, win: c_ulong,
                   name: *const c_char, value: *const c_char, mode: c_int) {
    let t = (xlib.XInternAtom)(dpy, value, xlib::False);
    (xlib.XChangeProperty)(dpy, win,
                           (xlib.XInternAtom)(dpy, name, xlib::False),
                           (xlib.XInternAtom)(dpy, cstr!("ATOM"), xlib::False),
                           32, mode, &t as *const u64 as *const c_uchar, 1);
}

unsafe fn add_atom(xlib: &xlib::Xlib, dpy: *mut xlib::Display, win: c_ulong,
                   name: *const c_char, value: *const c_char) {
    mod_atom(xlib, dpy, win, name, value, xlib::PropModeAppend);
}


unsafe fn set_atom(xlib: &xlib::Xlib, dpy: *mut xlib::Display, win: c_ulong,
                   name: *const c_char, value: *const c_char) {
    mod_atom(xlib, dpy, win, name, value, xlib::PropModeReplace);
}

unsafe fn set_as_dock(xlib: &xlib::Xlib, dpy: *mut xlib::Display, win: c_ulong) {
    // First set the _NET_WM_PID
    set_wm_pid(xlib, dpy, win);

    set_atom(xlib, dpy, win, cstr!("_NET_WM_WINDOW_TYPE"), cstr!("_NET_WM_WINDOW_TYPE_DOCK"));
    set_atom(xlib, dpy, win, cstr!("_NET_WM_STATE"), cstr!("_NET_WM_STATE_ABOVE"));
    add_atom(xlib, dpy, win, cstr!("_NET_WM_STATE"), cstr!("_NET_WM_STATE_STICKY"));

    let d: u32 = 0xffffffff;
    (xlib.XChangeProperty)(dpy, win,
                           (xlib.XInternAtom)(dpy, cstr!("_NET_WM_DESKTOP"), xlib::False),
                           (xlib.XInternAtom)(dpy, cstr!("CARDINAL"), xlib::False),
                           32, xlib::PropModeReplace, &d as *const u32 as *const c_uchar, 1);
}

//unsafe fn set_window_size(xlib: &xlib::Xlib, dpy: *mut xlib::Display, win: c_ulong, width: c_int, height: c_int) {
//    let mut changes : xlib::XWindowChanges = xlib::XWindowChanges { width: width, height: height, x: 0, y: 0, border_width: 0, sibling: 0, stack_mode: 0 };
//    (xlib.XConfigureWindow)(dpy, win, (xlib::CWWidth | xlib::CWHeight) as u32, &mut changes as *mut xlib::XWindowChanges);
//}
//
//unsafe fn set_window_position(xlib: &xlib::Xlib, dpy: *mut xlib::Display, win: c_ulong, x: c_int, y: c_int) {
//    let mut changes : xlib::XWindowChanges = xlib::XWindowChanges { width: 0, height: 0, x: x, y: y, border_width: 0, sibling: 0, stack_mode: 0 };
//    (xlib.XConfigureWindow)(dpy, win, (xlib::CWX | xlib::CWY) as u32, &mut changes as *mut xlib::XWindowChanges);
//}

unsafe extern fn handle_error(_: *mut xlib::Display, err: *mut xlib::XErrorEvent) -> i32 {
    println!("Error {}: {}", (*err).type_, (*err).error_code);
    return 0;
}

unsafe extern fn handle_io_error(_: *mut xlib::Display) -> i32 {
    println!("IOError");
    return 0;
}

unsafe fn create_xwin() {
    let xlib = xlib::Xlib::open().unwrap();
    let glx = glx::Glx::open().unwrap();
    let dpy = (xlib.XOpenDisplay)(ptr::null());
    let screen = (xlib.XDefaultScreen)(dpy);
    let root = (xlib.XRootWindow)(dpy, screen);

    (xlib.XSetErrorHandler)(Some(handle_error));
    (xlib.XSetIOErrorHandler)(Some(handle_io_error));

    // Hook close requests
    let wm_protocols = (xlib.XInternAtom)(dpy, cstr!("WM_PROTOCOLS"), xlib::False);
    let wm_delete_window = (xlib.XInternAtom)(dpy, cstr!("WM_DELETE_WINDOW"), xlib::False);

    let mut attribs = [
        glx::GLX_RGBA, glx::GLX_DOUBLEBUFFER, 0,
        glx::GLX_DEPTH_SIZE, 24,
        glx::GLX_STENCIL_SIZE, 8,
        glx::GLX_RED_SIZE, 8,
        glx::GLX_BLUE_SIZE, 8,
        glx::GLX_GREEN_SIZE, 8,
    ];
    let visual = (glx.glXChooseVisual)(dpy, 0, attribs.as_mut_ptr());

    if visual.is_null() {
        println!("Couldn't get the visual");
        return;
    }

    let mut attrs = xlib::XSetWindowAttributes {
        background_pixmap: 0,
        border_pixmap: 0,
        bit_gravity: 0,
        win_gravity: 0,
        backing_store: 0,
        backing_planes: 0,
        backing_pixel: 0,
        save_under: 0,
        do_not_propagate_mask: 0,
        cursor: 0,

        background_pixel: (xlib.XWhitePixel)(dpy, screen),
        border_pixel: (xlib.XBlackPixel)(dpy, screen),
        event_mask: xlib::ExposureMask,
        override_redirect: 1,
        colormap: (xlib.XCreateColormap)(dpy, root, (*visual).visual , xlib::AllocNone)
    };

    let win = (xlib.XCreateWindow)(dpy, root, 0, 0, 1024, 20, 0, (*visual).depth,
                                   xlib::InputOutput as u32, (*visual).visual,
                                   xlib::CWBackPixel | xlib::CWColormap | xlib::CWBorderPixel | xlib::CWEventMask,
                                   &mut attrs as *mut xlib::XSetWindowAttributes);
    if win <= 0 {
        println!("Couldn't create glx window");
        return;
    }

    set_as_dock(&xlib, dpy, win);

    let cxt = (glx.glXCreateContext)(dpy, visual, ptr::null_mut(), 1);
    if cxt.is_null() {
        println!("Couldn't create context");
        return;
    }
    (glx.glXMakeCurrent)(dpy, win, cxt);

    gl::load_with(|s| {
        let cstr = std::ffi::CString::new(s).unwrap();
        let ret = (glx.glXGetProcAddress)(cstr.as_ptr() as *const u8);

        return ret.unwrap() as *const std::os::raw::c_void;
    });

    let vendor = gl::GetString(gl::VENDOR);
    let cstr = std::ffi::CString::from_raw(vendor as *mut i8);
    println!("{}", cstr.into_string().unwrap());

    // Set window title
    (xlib.XStoreName)(dpy, win, cstr!("ongybar"));

    let mut protocols = [wm_delete_window];
    (xlib.XSetWMProtocols)(dpy, win, protocols.as_mut_ptr(), protocols.len() as c_int);

    (xlib.XMapRaised)(dpy, win);

    let mut event: xlib::XEvent = mem::uninitialized();

    loop {
        (xlib.XNextEvent)(dpy, &mut event);

        match event.get_type() {
            xlib::ClientMessage => {
                let xclient = xlib::XClientMessageEvent::from(event);

                if xclient.message_type == wm_protocols && xclient.format == 32 {
                    let protocol = xclient.data.get_long(0) as xlib::Atom;
                    if protocol == wm_delete_window {
                        break;
                    }
                }
            }, 
            _ => ()
        }
    }

    (xlib.XCloseDisplay)(dpy);
}

fn main() {
    unsafe {
        create_xwin();
    }
}
