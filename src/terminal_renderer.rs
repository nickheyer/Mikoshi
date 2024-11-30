use super::terminal_state::TerminalState;
use gl::types::*;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::surface::Surface;
use sdl2::ttf::Font;
use std::rc::Rc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const FONT_WIDTH: usize = 8;

pub struct TerminalRenderer<'a, 'b> {
    texture_id: GLuint,
    width: usize,
    height: usize,
    font: Rc<Font<'a, 'b>>,
    last_render_hash: u64,
}

impl<'a, 'b> TerminalRenderer<'a, 'b> {
    pub fn new(width: usize, height: usize, font: Rc<Font<'a, 'b>>) -> Self {
        let texture_id = create_terminal_texture(width, height);
        Self {
            texture_id,
            width,
            height,
            font,
            last_render_hash: 0,
        }
    }

    fn calculate_hash(content: &[(String, Color)]) -> u64 {
        let mut hasher = DefaultHasher::new();
        for (text, color) in content {
            text.hash(&mut hasher);
            color.r.hash(&mut hasher);
            color.g.hash(&mut hasher);
            color.b.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn render(&mut self, state: &TerminalState) -> Result<(), String> {
        let content = state.get_visible_content();
        let current_hash = Self::calculate_hash(&content);
        
        if current_hash == self.last_render_hash {
            return Ok(());
        }
        self.last_render_hash = current_hash;

        // Create background surface
        let mut surface = Surface::new(
            self.width as u32, 
            self.height as u32,
            PixelFormatEnum::RGBA32
        ).map_err(|e| e.to_string())?;
        
        // Fill with background color
        surface.fill_rect(None, state.get_settings().colors.background)
            .map_err(|e| e.to_string())?;

        let viewport = state.get_viewport();
        let line_height = viewport.line_height as i32;
        let mut y_offset = 5; // Small top padding
        
        // Render text and selection highlighting
        for (idx, (text, color)) in content.iter().enumerate() {
            // Skip if line would be below viewport
            if y_offset >= self.height as i32 {
                break;
            }

            // Create selection highlight if needed
            if let Some(selection) = state.get_selection() {
                let (start, end) = selection.normalize();
                if idx >= start.line && idx <= end.line {
                    let start_x = if idx == start.line { 
                        start.column * FONT_WIDTH 
                    } else { 
                        0 
                    };
                    let end_x = if idx == end.line { 
                        end.column * FONT_WIDTH
                    } else {
                        text.len() * FONT_WIDTH
                    };

                    let highlight_rect = sdl2::rect::Rect::new(
                        start_x as i32,
                        y_offset,
                        (end_x - start_x) as u32,
                        line_height as u32
                    );

                    surface.fill_rect(Some(highlight_rect), state.get_settings().colors.selection)
                        .map_err(|e| e.to_string())?;
                }
            }


            // Render text
            let text_surface = self.font.render(text)
                .blended(*color)  // Dereference the color
                .map_err(|_| format!("Failed to render text: {}", text))?;

            let text_rect = sdl2::rect::Rect::new(
                10, // Left margin
                y_offset,
                text_surface.width(),
                text_surface.height()
            );

            text_surface.blit(None, &mut surface, text_rect)
                .map_err(|e| e.to_string())?;

            y_offset += line_height;
        }

        // Update OpenGL texture
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
            
            let surface_rgba = surface.convert_format(PixelFormatEnum::RGBA32)
                .map_err(|e| e.to_string())?;
            
            let pixel_data = surface_rgba.without_lock()
                .ok_or_else(|| String::from("Failed to access surface pixel data"))?;
            
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                surface_rgba.width() as GLsizei,
                surface_rgba.height() as GLsizei,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                pixel_data.as_ptr() as *const _,
            );

            // Check for OpenGL errors
            let error = gl::GetError();
            if error != gl::NO_ERROR {
                return Err(format!("OpenGL error: 0x{:X}", error));
            }
        }
        
        Ok(())
    }

    pub fn get_texture_id(&self) -> GLuint {
        self.texture_id
    }
}

fn create_terminal_texture(width: usize, height: usize) -> GLuint {
    let mut texture_id: GLuint = 0;
    unsafe {
        gl::GenTextures(1, &mut texture_id);
        gl::BindTexture(gl::TEXTURE_2D, texture_id);
        
        let initial_data: Vec<u8> = vec![0; width * height * 4];
        
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as GLint,
            width as GLsizei,
            height as GLsizei,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            initial_data.as_ptr() as *const _,
        );
        
        // Use nearest-neighbor filtering for sharp text
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);
    }
    texture_id
}
