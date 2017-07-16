extern crate mio;
extern crate x11;
extern crate xcb;
extern crate gl;
extern crate libc;
extern crate hostname;

use self::mio::*;
use self::x11::glx::*;
use self::x11::xlib;
use self::xcb::dri2;

use std;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::ops::DerefMut;
use std::os::raw::*;
use std::os::raw::{c_int, c_void};
use std::ptr::null_mut;


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
    randr_ev: u8,
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

unsafe fn set_struts(conn: &xcb::Connection, win: c_uint, x: i16, _: i16, width: u16, height: u16) {
    {
        let prop = xcb::intern_atom(conn, false, "_NET_WM_STRUT").get_reply().unwrap().atom();
        let value = [0, 0, height as u32, 0];

        xcb::change_property(conn, xcb::PROP_MODE_REPLACE as u8, win, prop, xcb::ATOM_CARDINAL, 32, &value);
    }

    {
        let prop = xcb::intern_atom(conn, false, "_NET_WM_STRUT_PARTIAL").get_reply().unwrap().atom();
        let value = [0, 0, height as u32, 0,  /* LEFT RIGHT TOP BOTTOM */
                     0, 0, /* LEFT */
                     0, 0, /* RIGHT */
                     x as u32, x as u32 + width as u32, /* TOP */
                     0, 0 /* BOTTOM */
                    ];

        xcb::change_property(conn, xcb::PROP_MODE_REPLACE as u8, win, prop, xcb::ATOM_CARDINAL, 32, &value);
    }

}

unsafe fn set_geometry(win: &X11Window, x: i32, y: i32, width: u32, height: u32) {
    let values = [(xlib::CWX, x as u32),
                  (xlib::CWY, y as u32),
                  (xlib::CWWidth, width),
                  (xlib::CWHeight, height)
                 ];
    let _ = xcb::xproto::configure_window(&win.conn, win.win, &values);

    set_struts(&win.conn, win.win, x as i16, y as i16, width as u16, height as u16);
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

    let mut width = 0;
    let mut height = 0;
    let cmap = conn.generate_id();
    let win = conn.generate_id();

    {
        let setup = conn.get_setup();
        let screen = setup.roots().nth((*vi).screen as usize).unwrap();

        let _ = xcb::randr::select_input(&conn, screen.root(), xcb::randr::NOTIFY_MASK_CRTC_CHANGE as u16).request_check();

        let screen_res_cookie = xcb::randr::get_screen_resources(&conn, screen.root());
        let screen_res_reply = screen_res_cookie.get_reply().unwrap();
        let outputs = screen_res_reply.outputs();

        let mut output_cookies = Vec::with_capacity(outputs.len());
        for output in outputs {
            output_cookies.push(xcb::randr::get_output_info(&conn, *output, 0));
        }

        let mut x = 0;
        let mut y = 0;

        for out_cookie in output_cookies.iter() {
            if let Ok(reply) = out_cookie.get_reply() {
                /* Filter out unplugged outputs
                 * and outputs that aren't set up (yet)
                 */
                if reply.connection() != 0  || reply.crtc() == 0 {
                    continue;
                }

                let name = String::from_utf8_lossy(reply.name());
                println!(" Output:\t{}", name);
                let crtc = xcb::randr::get_crtc_info(&conn, reply.crtc(), 0).get_reply().unwrap();
                println!("\t{}x{}+{}x{}", crtc.width(), crtc.height(), crtc.x(), crtc.y());

                width = crtc.width();
                //height = crtc.height();
                height = 16;
                x = crtc.x();
                y = crtc.y();
            }
        }

        println!("Picking last one for now, since that's easiest to implement :)");


        xcb::create_colormap(&conn, xcb::COLORMAP_ALLOC_NONE as u8,
                cmap, screen.root(), (*vi).visualid as u32);

        let cw_values = [
            (xcb::CW_BACK_PIXEL, screen.white_pixel()),
            (xcb::CW_BORDER_PIXEL, screen.black_pixel()),
            (xcb::CW_EVENT_MASK, xcb::EVENT_MASK_EXPOSURE),
            (xcb::CW_COLORMAP, cmap)
        ];


        xcb::create_window(&conn, (*vi).depth as u8, win, screen.root(),
                           x, y, width, height,
                           0, xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
                           (*vi).visualid as u32, &cw_values);

        set_struts(&conn, win, x, y, width, height);
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

    let randr_base = conn.get_extension_data(&mut xcb::randr::id()).unwrap().first_event();


    let win = X11Window { conn: conn, win: win, dri2_ev: dri2_ev,
                          screen_num: screen_num, randr_ev: randr_base,
                          height: height as u32, width: width as u32,
                          wm_protocols: wm_protocols, cmap: cmap,
                          wm_delete_window: wm_delete_window };
    return (win, fbc);
}

unsafe fn make_glcontext(win: &X11Window, fbc: *mut __GLXFBConfigRec) -> *mut __GLXcontextRec {
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

    return ctx;
}

static mut RUN: bool = true;

unsafe fn handle_event<F, G>(win: &X11Window,
                          graphics: &mut G,
                          draw_window: &mut F,
                          ev: xcb::Event<xcb::ffi::xcb_generic_event_t>)
        where F: FnMut(&mut G, u32, u32) {

    let ev_type = ev.response_type() & !0x80;
    match ev_type {
        xcb::EXPOSE => {
            draw_window(graphics, win.width, win.height);
            glXSwapBuffers(win.conn.get_raw_dpy(), win.win as xlib::XID);
        },
        xcb::KEY_PRESS => {
            RUN = false;
        },
        xcb::CLIENT_MESSAGE => {
            let cmev = xcb::cast_event::<xcb::ClientMessageEvent>(&ev);
            if cmev.type_() == win.wm_protocols && cmev.format() == 32 {
                let protocol = cmev.data().data32()[0];
                if protocol == win.wm_delete_window {
                    RUN = false;
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

            } else if ev_type == win.randr_ev + xcb::randr::NOTIFY {
                let v: &xcb::randr::NotifyEvent = xcb::cast_event(&ev);
                let d = v.u().cc();
                let crtc = xcb::randr::get_crtc_info(&win.conn, d.crtc(), 0).get_reply().unwrap();
                for output in crtc.outputs() {
                    let o = xcb::randr::get_output_info(&win.conn, *output, 0).get_reply().unwrap();
                    let name = String::from_utf8_lossy(o.name());
                    println!("Changed config for output: {}", name);
                    /* TODO: This should be from config, not hardcoded! */
                    if name == "LVDS1" {
                        set_geometry(win, d.x() as i32, d.y() as i32, d.width() as u32, 16);
                    }
                }
            } else {
                println!("Xrandr would be: {} + {}", win.randr_ev, xcb::randr::NOTIFY);
                println!("Got an unkown event!: {}", ev_type);
            }
        }
    }
    win.conn.flush();
}

unsafe fn poll_event<F, G>(win: &X11Window,
                        graphics: &mut G,
                        draw_window: &mut F)
        where F: FnMut(&mut G, u32, u32) {
    if let Some(ev) = win.conn.poll_for_event() {
        handle_event(win, graphics, draw_window, ev);
    }
}

unsafe fn wait_event<F, G>(win: &X11Window,
                        graphics: &mut G,
                        draw_window: &mut F) -> bool
        where F: FnMut(&mut G, u32, u32) {
    if let Some(ev) = win.conn.poll_for_event() {
        handle_event(win, graphics, draw_window, ev);
    }

    return false;
}

pub fn do_x11main<F, G, L, V>(draw_window: F, create: L, fun_list: G)
    where F: FnMut(&mut V, u32, u32),
          L: FnOnce() -> V,
          G: std::iter::IntoIterator<Item=(c_int, Box<FnMut() -> bool>)> {
    unsafe {
        let (win, fbc) = create_window();
        let ctx = make_glcontext(&win, fbc);
        let graphics = create();

        let graphics_cell = RefCell::new(graphics);
        let fun_cell = RefCell::new(draw_window);

        let xcb_fd: c_int = xcb::ffi::base::xcb_get_file_descriptor(win.conn.get_raw_conn());

        let xcbt: Token = Token(xcb_fd as usize);
        let poll = Poll::new().unwrap();

        poll.register(&mio::unix::EventedFd(&xcb_fd),
                      xcbt, Ready::readable(),
                      PollOpt::level()).unwrap();

        let mut map = HashMap::new();
        map.insert(xcbt, Box::new(||  {
            wait_event(&win, graphics_cell.borrow_mut().deref_mut(), fun_cell.borrow_mut().deref_mut());
            return false;
        }) as Box<FnMut() -> bool>);

        for x in fun_list {
            let tok = Token(x.0 as usize);
            poll.register(&mio::unix::EventedFd(&x.0), tok, Ready::readable(),
                          PollOpt::level()).unwrap();
            map.insert(tok, x.1);
        }


        let mut events = Events::with_capacity(map.len() + 2);
        RUN = true;

        loop {
            poll_event(&win, graphics_cell.borrow_mut().deref_mut(), fun_cell.borrow_mut().deref_mut());
            poll.poll(&mut events, None).unwrap();

            for event in events.iter() {
                let mut fun = map.get_mut(&event.token()).unwrap();
                if fun.deref_mut()() {
                    let mut draw_fun = fun_cell.borrow_mut();
                    draw_fun.deref_mut()(graphics_cell.borrow_mut().deref_mut(), win.width, win.height);
                    glXSwapBuffers(win.conn.get_raw_dpy(), win.win as xlib::XID);
                }
            }

            if !RUN {
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
