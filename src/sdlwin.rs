use sdl2::video::{Window, SwapInterval};
use sdl2::{Sdl, VideoSubsystem};

#[allow(dead_code)]
pub struct Sdlwin {
    pub sdl: Sdl,
    pub video_subsystem: VideoSubsystem,
    pub window: Window,
    gl_context: sdl2::video::GLContext,
}

impl Sdlwin {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let sdl = sdl2::init().map_err(|e| format!("SDL init failed: {}", e))?;
        let video_subsystem = sdl.video().map_err(|e| format!("Failed to get SDL video subsystem: {}", e))?;

        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(3, 3);

        let window = video_subsystem
            .window("Mikoshi", width, height)
            .opengl()
            .resizable()
            .build()
            .map_err(|e| format!("Failed to create window: {}", e))?;

        let gl_context = window.gl_create_context().map_err(|e| format!("Failed to create OpenGL context: {}", e))?;
        window
            .subsystem()
            .gl_set_swap_interval(SwapInterval::VSync)
            .map_err(|e| format!("Failed to set swap interval: {}", e))?;

        let _gl = gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const _);

        Ok(Sdlwin {
            sdl,
            video_subsystem,
            window,
            gl_context,
        })
    }
}
