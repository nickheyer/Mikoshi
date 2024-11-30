mod sdlwin;
mod shaders;
mod terminal;
mod terminal_state;
mod terminal_renderer;

use terminal::Terminal;
use terminal_state::TerminalState;
use terminal_renderer::TerminalRenderer;
use shaders::*;
use sdlwin::Sdlwin;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::MouseButton;
use std::rc::Rc;
use std::time::Instant;

const FONT_SIZE: u16 = 16;

fn handle_keyboard_input(
    keycode: Keycode,
    keymod: Mod,
    terminal_state: &mut TerminalState,
    terminal: &mut Terminal,
    video_subsystem: &sdl2::VideoSubsystem,
) {
    match (keycode, keymod) {
        (Keycode::Return, _) => {
            terminal_state.commit_input();
            let _ = terminal.write_input(b"\n");
        }
        (Keycode::Backspace, _) => {
            terminal_state.handle_backspace();
        }
        (Keycode::C, mod_combination) if mod_combination.contains(Mod::LCTRLMOD) && mod_combination.contains(Mod::LSHIFTMOD) => {
            let selected_text = terminal_state.get_selected_text();
            if !selected_text.is_empty() {
                let _ = video_subsystem.clipboard().set_clipboard_text(&selected_text);
            }
        }
        (Keycode::V, mod_combination) if mod_combination.contains(Mod::LCTRLMOD) => {
            if let Ok(text) = video_subsystem.clipboard().clipboard_text() {
                terminal_state.add_input(&text);
                let _ = terminal.write_input(text.as_bytes());
            }
        }
        (Keycode::C, mod_combination) if mod_combination.contains(Mod::LCTRLMOD) => {
            let _ = terminal.write_input(&[4]); // EOT
        }
        (Keycode::D, mod_combination) if mod_combination.contains(Mod::LCTRLMOD) => {
            let _ = terminal.write_input(&[4]); // EOT
        }
        (Keycode::L, mod_combination) if mod_combination.contains(Mod::LCTRLMOD) => {
            terminal_state.clear();
            let _ = terminal.write_input(b"\x0C");
        }
        (Keycode::Up, mod_combination) if mod_combination.contains(Mod::LCTRLMOD) => {
            terminal_state.scroll_up(1);
        }
        (Keycode::Down, mod_combination) if mod_combination.contains(Mod::LCTRLMOD) => {
            terminal_state.scroll_down(1);
        }
        (Keycode::Up, Mod::NOMOD) => {
            terminal_state.handle_key_up();
        }
        (Keycode::Down, Mod::NOMOD) => {
            terminal_state.handle_key_down();
        }
        _ => {}
    }
}

fn handle_mouse_input(
    event: &Event,
    terminal_state: &mut TerminalState,
    terminal: &mut Terminal,
    video_subsystem: &sdl2::VideoSubsystem,
    line_height: u32,
) {
    match event {
        Event::MouseButtonDown {
            mouse_btn: MouseButton::Left,
            x,
            y,
            ..
        } => {
            let line = *y as usize / line_height as usize;
            let col = *x as usize / (FONT_SIZE / 2) as usize;
            terminal_state.start_selection(line, col);
        }
        Event::MouseMotion { x, y, mousestate, .. } => {
            if mousestate.left() {
                let line = *y as usize / line_height as usize;
                let col = *x as usize / (FONT_SIZE / 2) as usize;
                terminal_state.update_selection(line, col);
            }
        }
        Event::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => {
            let selected_text = terminal_state.get_selected_text();
            if !selected_text.is_empty() {
                let _ = video_subsystem.clipboard().set_clipboard_text(&selected_text);
            }
        }
        Event::MouseWheel { y, .. } => {
            if *y > 0 {
                terminal_state.scroll_up(3);
            } else if *y < 0 {
                terminal_state.scroll_down(3);
            }
        }
        Event::TextInput { text, .. } => {
            if text.chars().all(|c| c.is_ascii_graphic() || c.is_whitespace()) {
                terminal_state.add_input(&text);
                let _ = terminal.write_input(text.as_bytes());
            }
        }
        _ => {}
    }
}

fn main() {
    let width: u32 = 1000;
    let height: u32 = 800;

    let sdlwin = Sdlwin::new(width, height).unwrap();
    let video_subsystem = &sdlwin.video_subsystem;
    let ttf_context = sdl2::ttf::init().unwrap();
    let font = Rc::new(ttf_context.load_font("/usr/share/fonts/TTF/DejaVuSansMono.ttf", FONT_SIZE).unwrap());

    let line_height = font.height() as u32;

    let mut terminal = Terminal::new();
    let mut terminal_state = TerminalState::new(width, height, line_height);
    let mut renderer = TerminalRenderer::new(width as usize, height as usize, Rc::clone(&font));

    let shader_program = ShaderProgram::new("shaders/terminal.vert", "shaders/terminal.frag")
        .expect("Failed to create shader program");
    let quad = create_screen_quad();

    let start_time = Instant::now();
    let mut event_pump = sdlwin.sdl.event_pump().unwrap();
    video_subsystem.text_input().start();

    'running: loop {
        let current_time = start_time.elapsed().as_secs_f32();

        if terminal.should_exit() {
            break 'running;
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,

                Event::KeyDown {
                    keycode: Some(keycode),
                    keymod,
                    ..
                } => handle_keyboard_input(keycode, keymod, &mut terminal_state, &mut terminal, video_subsystem),

                Event::TextInput { text, .. } => {
                    terminal_state.add_input(&text);
                    let _ = terminal.write_input(text.as_bytes());
                }

                Event::MouseButtonDown { .. }
                | Event::MouseMotion { .. }
                | Event::MouseButtonUp { .. }
                | Event::MouseWheel { .. } => {
                    handle_mouse_input(&event, &mut terminal_state, &mut terminal, video_subsystem, line_height);
                }

                Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(w, h),
                    ..
                } => {
                    unsafe { gl::Viewport(0, 0, w, h); }
                    terminal_state = TerminalState::new(w as u32, h as u32, line_height);
                    renderer = TerminalRenderer::new(w as usize, h as usize, Rc::clone(&font));
                }

                _ => {}
            }
        }

        let output = terminal.get_output();
        if !output.is_empty() {
            if let Ok(text) = String::from_utf8(output) {
                terminal_state.add_output(&text);
            }
        }

        if let Err(e) = renderer.render(&terminal_state) {
            eprintln!("Render error: {}", e);
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
            shader_program.set();
            shader_program.set_uniform_f32("time", current_time);
            shader_program.set_uniform_vec2("resolution", width as f32, height as f32);
            gl::BindTexture(gl::TEXTURE_2D, renderer.get_texture_id());
            quad.draw();
        }

        sdlwin.window.gl_swap_window();
    }
}

