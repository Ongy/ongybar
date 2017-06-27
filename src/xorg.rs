use xcb::dri2;

use gl;
use xcb;
use hostname;

use libc;

use opengl_graphics;

use x11::xlib;
use x11::glx::*;

use std;
use std::os::raw::*;
use std::ptr::null_mut;
use std::ffi::{CStr, CString};
use std::os::raw::{c_int, c_void};

use mio;
use mio::*;

const GLX_CONTEXT_MAJOR_VERSION_ARB: u32 = 0x2091;
const GLX_CONTEXT_MINOR_VERSION_ARB: u32 = 0x2092;

struct X11Window {
    conn: xcb::Connection,
    win: c_uint,
    width: u32,
    height: u32,

    screen_num: i32,
    wm_delete_window: u32,
    wm_protocols: u32,
    dri2_ev: u8,
    cmap: u32,
}

type GlXCreateContextAttribsARBProc =
    unsafe extern "C" fn (dpy: *mut xlib::Display, fbc: GLXFBConfig,
            share_context: GLXContext, direct: xlib::Bool,
            attribs: *const c_int) -> GLXContext;


unsafe fn load_gl_func (name: &str) -> *mut c_void {
    let cname = CString::new(name).unwrap();
    let ptr: *mut c_void = std::mem::transmute(glXGetProcAddress(
            cname.as_ptr() as *const u8
    ));
    if ptr.is_null() {
        panic!("could not load {}", name);
    }
    ptr
}

fn check_glx_extension(glx_exts: &str, ext_name: &str) -> bool {
    for glx_ext in glx_exts.split(" ") {
        if glx_ext == ext_name {
            return true;
        }
    }
    false
}

static mut CTX_ERROR_OCCURED: bool = false;
unsafe extern "C" fn ctx_error_handler(
        _dpy: *mut xlib::Display,
        _ev: *mut xlib::XErrorEvent) -> i32 {
    CTX_ERROR_OCCURED = true;
    0
}


// returns the glx version in a decimal form
// eg. 1.3  => 13
fn glx_dec_version(dpy: *mut xlib::Display) -> i32 {
    let mut maj: c_int = 0;
    let mut min: c_int = 0;
    unsafe {
        if glXQueryVersion(dpy,
                &mut maj as *mut c_int,
                &mut min as *mut c_int) == 0 {
            panic!("cannot get glx version");
        }
    }
    (maj*10 + min) as i32
}


fn get_glxfbconfig(dpy: *mut xlib::Display, screen_num: i32,
        visual_attribs: &[i32]) -> GLXFBConfig {
    unsafe {
        let mut fbcount: c_int = 0;
        let fbcs = glXChooseFBConfig(dpy, screen_num,
                visual_attribs.as_ptr(),
                &mut fbcount as *mut c_int);

        if fbcount == 0 {
            panic!("could not find compatible fb config");
        }
        // we pick the first from the list
        let fbc = *fbcs;
        xlib::XFree(fbcs as *mut c_void);
        fbc
    }
}

unsafe fn modify_atom(conn: &xcb::Connection, win: c_uint, mode: u8, name: &str, val: &str) {
    let atom = xcb::intern_atom(conn, false, name).get_reply().unwrap().atom();
    let value = [xcb::intern_atom(conn, false, val).get_reply().unwrap().atom()];

    xcb::change_property(conn, mode, win,
                         atom, xcb::ATOM_ATOM, 32,
                         &value);
}

unsafe fn set_wm_pid(conn: &xcb::Connection, win: c_uint) {
    match hostname::get_hostname() {
        None => println!("Coudln't get hostname. Skipping _NET_WM_PID"),
        Some(x) => {
            xcb::change_property(conn, xcb::PROP_MODE_REPLACE as u8,
                                 win, xcb::ATOM_WM_CLIENT_MACHINE,
                                 xcb::ATOM_STRING, 8, x.as_bytes());

            let atom = xcb::intern_atom(conn, false, "_NET_WM_PID").get_reply().unwrap().atom();

            let pid = libc::getpid();
            let pid_buffer = [pid];

            xcb::change_property(conn, xcb::PROP_MODE_REPLACE as u8,
                                 win, atom, xcb::ATOM_CARDINAL, 32,
                                 &pid_buffer);
        }
    }
}

unsafe fn set_dock(conn: &xcb::Connection, win: c_uint) {
    set_wm_pid(conn, win);

    modify_atom(conn, win, xcb::PROP_MODE_REPLACE as u8,
                "_NET_WM_WINDOW_TYPE", "_NET_WM_WINDOW_TYPE_DOCK");

    modify_atom(conn, win, xcb::PROP_MODE_REPLACE as u8,
                "_NET_WM_STATE", "_NET_WM_STATE_ABOVE");
    modify_atom(conn, win, xcb::PROP_MODE_APPEND as u8,
                "_NET_WM_STATE", "_NET_WM_STATE_STICKY");

    let val :[u32; 1] = [0xFFFFFFFF];
    let desktop = xcb::intern_atom(conn, false, "_NET_WM_DESKTOP").get_reply().unwrap().atom();
    xcb::change_property(conn, xcb::PROP_MODE_REPLACE as u8, win, desktop,
                         xcb::ATOM_CARDINAL, 32, &val);
}

unsafe fn create_window() -> (X11Window, *mut __GLXFBConfigRec) {
    let (conn, screen_num) = xcb::Connection::connect_with_xlib_display().unwrap();
    conn.set_event_queue_owner(xcb::EventQueueOwner::Xcb);

    if glx_dec_version(conn.get_raw_dpy()) < 13 {
        panic!("glx-1.3 is not supported");
    }

    let fbc = get_glxfbconfig(conn.get_raw_dpy(), screen_num, &[
            GLX_X_RENDERABLE    , 1,
            GLX_DRAWABLE_TYPE   , GLX_WINDOW_BIT,
            GLX_RENDER_TYPE     , GLX_RGBA_BIT,
            GLX_X_VISUAL_TYPE   , GLX_TRUE_COLOR,
            GLX_RED_SIZE        , 8,
            GLX_GREEN_SIZE      , 8,
            GLX_BLUE_SIZE       , 8,
            GLX_ALPHA_SIZE      , 8,
            GLX_DEPTH_SIZE      , 24,
            GLX_STENCIL_SIZE    , 8,
            GLX_DOUBLEBUFFER    , 1,
            0
    ]);

    let vi: *const xlib::XVisualInfo =
            glXGetVisualFromFBConfig(conn.get_raw_dpy(), fbc);

    let dri2_ev = {
        conn.prefetch_extension_data(dri2::id());
        match conn.get_extension_data(dri2::id()) {
            None => { panic!("could not load dri2 extension") },
            Some(r) => { r.first_event() }
        }
    };

    let (wm_protocols, wm_delete_window) = {
        let pc = xcb::intern_atom(&conn, false, "WM_PROTOCOLS");
        let dwc = xcb::intern_atom(&conn, false, "WM_DELETE_WINDOW");

        let p = match pc.get_reply() {
            Ok(p) => p.atom(),
            Err(_) => panic!("could not load WM_PROTOCOLS atom")
        };
        let dw = match dwc.get_reply() {
            Ok(dw) => dw.atom(),
            Err(_) => panic!("could not load WM_DELETE_WINDOW atom")
        };
        (p, dw)
    };

    let width = 1366;
    let height = 18;
    let cmap = conn.generate_id();
    let win = conn.generate_id();

    {
        let setup = conn.get_setup();
        let screen = setup.roots().nth((*vi).screen as usize).unwrap();


        xcb::create_colormap(&conn, xcb::COLORMAP_ALLOC_NONE as u8,
                cmap, screen.root(), (*vi).visualid as u32);

        let cw_values = [
            (xcb::CW_BACK_PIXEL, screen.white_pixel()),
            (xcb::CW_BORDER_PIXEL, screen.black_pixel()),
            (xcb::CW_EVENT_MASK, xcb::EVENT_MASK_EXPOSURE),
            (xcb::CW_COLORMAP, cmap)
        ];


        xcb::create_window(&conn, (*vi).depth as u8, win, screen.root(),
                           0, 0, width, height,
                           0, xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
                           (*vi).visualid as u32, &cw_values);
    }

    xlib::XFree(vi as *mut c_void);

    set_dock(&conn, win);

    let title = "ongybar";
    xcb::change_property(&conn,
            xcb::PROP_MODE_REPLACE as u8,
            win,
            xcb::ATOM_WM_NAME,
            xcb::ATOM_STRING,
            8, title.as_bytes());

    let protocols = [wm_delete_window];
    xcb::change_property(&conn, xcb::PROP_MODE_REPLACE as u8,
            win, wm_protocols, xcb::ATOM_ATOM, 32, &protocols);

    xcb::map_window(&conn, win);
    conn.flush();
    xlib::XSync(conn.get_raw_dpy(), xlib::False);

    let win = X11Window { conn: conn, win: win, dri2_ev: dri2_ev,
                          screen_num: screen_num,
                          height: height as u32, width: width as u32,
                          wm_protocols: wm_protocols, cmap: cmap,
                          wm_delete_window: wm_delete_window };
    return (win, fbc);
}

unsafe fn make_glcontext(win: &X11Window, fbc: *mut __GLXFBConfigRec) -> (*mut __GLXcontextRec, opengl_graphics::GlGraphics) {
    let glx_exts = CStr::from_ptr(
        glXQueryExtensionsString(win.conn.get_raw_dpy(), win.screen_num))
        .to_str().unwrap();

    if !check_glx_extension(&glx_exts, "GLX_ARB_create_context") {
        panic!("could not find GLX extension GLX_ARB_create_context");
    }

    // with glx, no need of a current context is needed to load symbols
    // otherwise we would need to create a temporary legacy GL context
    // for loading symbols (at least glXCreateContextAttribsARB)
    let glx_create_context_attribs: GlXCreateContextAttribsARBProc =
        std::mem::transmute(load_gl_func("glXCreateContextAttribsARB"));

    // loading all other symbols
    gl::load_with(|n| load_gl_func(&n));

    if !gl::GenVertexArrays::is_loaded() {
        panic!("no GL3 support available!");
    }

    // installing an event handler to check if error is generated
    CTX_ERROR_OCCURED = false;
    let old_handler = xlib::XSetErrorHandler(Some(ctx_error_handler));

    let context_attribs: [c_int; 5] = [
        GLX_CONTEXT_MAJOR_VERSION_ARB as c_int, 3,
        GLX_CONTEXT_MINOR_VERSION_ARB as c_int, 0,
        0
    ];
    let ctx = glx_create_context_attribs(win.conn.get_raw_dpy(), fbc, null_mut(),
            xlib::True, &context_attribs[0] as *const c_int);

    win.conn.flush();
    xlib::XSync(win.conn.get_raw_dpy(), xlib::False);
    xlib::XSetErrorHandler(std::mem::transmute(old_handler));

    if ctx.is_null() || CTX_ERROR_OCCURED {
        panic!("error when creating gl-3.0 context");
    }

    if glXIsDirect(win.conn.get_raw_dpy(), ctx) == 0 {
        panic!("obtained indirect rendering context")
    }

    glXMakeCurrent(win.conn.get_raw_dpy(), win.win as xlib::XID, ctx);

    let mut major = [1];
    let mut minor = [1];

    gl::GetIntegerv(gl::MAJOR_VERSION, major.as_mut_ptr());
    gl::GetIntegerv(gl::MINOR_VERSION, minor.as_mut_ptr());

    println!("{}.{}", major[0], minor[0]);

    return (ctx, opengl_graphics::GlGraphics::new(opengl_graphics::OpenGL::V3_0));
}

unsafe fn handle_event<F>(win: &X11Window,
                          graphics: &mut opengl_graphics::GlGraphics,
                          draw_window: &mut F,
                          ev: xcb::Event<xcb::ffi::xcb_generic_event_t>) -> bool
        where F: FnMut(&mut opengl_graphics::GlGraphics, u32, u32) {

    let ev_type = ev.response_type() & !0x80;
    match ev_type {
        xcb::EXPOSE => {
            draw_window(graphics, win.width, win.height);
            glXSwapBuffers(win.conn.get_raw_dpy(), win.win as xlib::XID);
        },
        xcb::KEY_PRESS => {
            return false;
        },
        xcb::CLIENT_MESSAGE => {
            let cmev = xcb::cast_event::<xcb::ClientMessageEvent>(&ev);
            if cmev.type_() == win.wm_protocols && cmev.format() == 32 {
                let protocol = cmev.data().data32()[0];
                if protocol == win.wm_delete_window {
                    return false;
                }
            }
        },
        _ => {
            // following stuff is not obvious at all, but is necessary
            // to handle GL when XCB owns the event queue
            if ev_type == win.dri2_ev || ev_type == win.dri2_ev+1 {
                // these are libgl dri2 event that need special handling
                // see https://bugs.freedesktop.org/show_bug.cgi?id=35945#c4
                // and mailing thread starting here:
                // http://lists.freedesktop.org/archives/xcb/2015-November/010556.html

                if let Some(proc_) =
                        xlib::XESetWireToEvent(win.conn.get_raw_dpy(),
                                ev_type as i32, None) {
                    xlib::XESetWireToEvent(win.conn.get_raw_dpy(),
                            ev_type as i32, Some(proc_));
                    let raw_ev = ev.ptr;
                    (*raw_ev).sequence =
                        xlib::XLastKnownRequestProcessed(
                                win.conn.get_raw_dpy()) as u16;
                    let mut dummy: xlib::XEvent = std::mem::zeroed();
                    proc_(win.conn.get_raw_dpy(),
                        &mut dummy as *mut xlib::XEvent,
                        raw_ev as *mut xlib::xEvent);
                }

            }
        }
    }
    win.conn.flush();

    return true
}

unsafe fn poll_event<F>(win: &X11Window,
                        graphics: &mut opengl_graphics::GlGraphics,
                        draw_window: &mut F) -> bool
        where F: FnMut(&mut opengl_graphics::GlGraphics, u32, u32) {
    if let Some(ev) = win.conn.poll_for_event() {
        return handle_event(win, graphics, draw_window, ev);
    }

    return true;
}

unsafe fn wait_event<F>(win: &X11Window,
                        graphics: &mut opengl_graphics::GlGraphics,
                        draw_window: &mut F) -> bool
        where F: FnMut(&mut opengl_graphics::GlGraphics, u32, u32) {
    if let Some(ev) = win.conn.wait_for_event() {
        return handle_event(win, graphics, draw_window, ev);
    }

    return false;
}

pub fn do_x11main<F>(mut draw_window: F)
    where F: FnMut(&mut opengl_graphics::GlGraphics, u32, u32) {
    unsafe {
        let (win, fbc) = create_window();
        let (ctx, mut graphics) = make_glcontext(&win, fbc);

        let xcb_fd: c_int = xcb::ffi::base::xcb_get_file_descriptor(win.conn.get_raw_conn());

        const XCB: Token = Token(0);
        let poll = Poll::new().unwrap();

        poll.register(&mio::unix::EventedFd(&xcb_fd),
                      XCB, Ready::readable(),
                      PollOpt::level()).unwrap();

        let mut events = Events::with_capacity(1024);

        let mut run = true;

        println!("Going into X loop");
        loop {
            poll_event(&win, &mut graphics, &mut draw_window);
            poll.poll(&mut events, None).unwrap();

            for event in events.iter() {
                println!("Got some event o.0");
                match event.token() {
                    XCB => {
                        if !wait_event(&win, &mut graphics, &mut draw_window) {
                            run = false;
                            break;
                        }
                    }
                    Token(_) => { run = false; break; }
                }
            }

            if !run {
                break;
            }
        }

        // only to make sure that rs_client generate correct names for DRI2
        // (used to be "*_DRI_2_*")
        // should be in a "compile tests" section instead of example
        let _ = xcb::ffi::dri2::XCB_DRI2_ATTACHMENT_BUFFER_ACCUM;

        glXDestroyContext(win.conn.get_raw_dpy(), ctx);

        xcb::unmap_window(&win.conn, win.win);
        xcb::destroy_window(&win.conn, win.win);
        xcb::free_colormap(&win.conn, win.cmap);
        win.conn.flush();
    }
}
