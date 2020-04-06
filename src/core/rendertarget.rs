use crate::core::*;

pub struct Screen {}

impl Screen {
    pub fn write(gl: &Gl, x: i32, y: i32, width: usize, height: usize,
                          clear_color: Option<&Vec4>, clear_depth: Option<f32>, render: &dyn Fn()) -> Result<(), Error>
    {
        gl.viewport(x, y, width, height);
        gl.bind_framebuffer(consts::DRAW_FRAMEBUFFER, None);
        RenderTarget::clear(gl, clear_color, clear_depth);
        render();
        Ok(())
    }

    // TODO: Possible to change format
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_color(gl: &Gl, x: i32, y: i32, width: usize, height: usize) -> Result<Vec<u8>, Error>
    {
        gl.viewport(x, y, width, height);
        let mut pixels = vec![0u8; width * height * 3];
        gl.bind_framebuffer(consts::READ_FRAMEBUFFER, None);
        gl.read_pixels_with_u8_data(x as u32, y as u32, width as u32, height as u32, consts::RGB,
                            consts::UNSIGNED_BYTE, &mut pixels);
        Ok(pixels)
    }

    // TODO: Possible to change format
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_depth(gl: &Gl, x: i32, y: i32, width: usize, height: usize) -> Result<Vec<f32>, Error>
    {
        gl.viewport(x, y, width, height);
        let mut pixels = vec![0f32; width * height];
        gl.bind_framebuffer(consts::READ_FRAMEBUFFER, None);
        gl.read_pixels_with_f32_data(x as u32, y as u32, width as u32, height as u32,
                        consts::DEPTH_COMPONENT, consts::FLOAT, &mut pixels);
        Ok(pixels)
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "image-io"))]
    pub fn save_color(path: &str, gl: &Gl, x: i32, y: i32, width: usize, height: usize) -> Result<(), Error>
    {
        let pixels = Self::read_color(gl, x, y, width, height)?;
        let mut pixels_out = vec![0u8; width * height * 3];
        for row in 0..height {
            for col in 0..width {
                for i in 0..3 {
                    pixels_out[3 * width * (height - row - 1) + 3 * col + i] =
                        pixels[3 * width * row + 3 * col + i];
                }
            }
        }

        image::save_buffer(&std::path::Path::new(path), &pixels_out, width as u32, height as u32, image::RGB(8))?;
        Ok(())
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "image-io"))]
fn save_image(path: &str, width: usize, height: usize, channels: usize, pixels: &[u8])
{
    let mut pixels_out = vec![0u8; width * height * channels];
    for row in 0..height {
        for col in 0..width {
            for i in 0..channels {
                pixels_out[channels * width * (height - row - 1) + channels * col + i] =
                    pixels[channels * width * row + channels * col + i];
            }
        }
    }

    image::save_buffer(&std::path::Path::new(path), &pixels_out, width as u32, height as u32, image::RGB(8)).unwrap();
}

struct Todo {
    path: String,
    sync: crate::gl::Sync,
    buffer: PixelPackBuffer,
    width: usize,
    height: usize,
    channels: usize
}

pub struct AsyncSaveScreen {
    gl: Gl,
    todos: Vec<Todo>
}

impl AsyncSaveScreen {

    pub fn new(gl: &Gl) -> Self {
        AsyncSaveScreen { gl: gl.clone(), todos: Vec::new() }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_screenshot_async(&mut self, path: &str, x: i32, y: i32, width: usize, height: usize) -> Result<(), Error>
    {
        self.gl.viewport(x, y, width, height);
        self.gl.bind_framebuffer(consts::READ_FRAMEBUFFER, None);

        let w = (width as i32 - x) as usize;
        let h = (height as i32 - y) as usize;
        let size_in_bytes = w * h * 3;
        let buffer = PixelPackBuffer::new(&self.gl, size_in_bytes)?;
        buffer.bind();
        self.gl.read_pixels(x as u32, y as u32, width as u32, height as u32, consts::RGB, consts::UNSIGNED_BYTE);
        buffer.unbind();

        let sync = self.gl.fence_sync();
        self.gl.flush();

        self.todos.push(Todo {
            path: path.to_string(), buffer,
            sync, width: w, height: h, channels: 3
        });
        self.save();
        Ok(())
    }

    fn save(&mut self) {
        let gl = self.gl.clone();
        self.todos.retain(|todo| {
            //println!("check {}", todo.path);
            if gl.client_wait_sync(&todo.sync, 0, 0) != consts::TIMEOUT_EXPIRED {
                gl.delete_sync(&todo.sync);
                let pixels = todo.buffer.read_pixels(todo.width * todo.height * todo.channels);
                image::save_buffer(&std::path::Path::new(&todo.path), &pixels, todo.width as u32, todo.height as u32, image::RGB(8)).unwrap();
                /*println!("save: {} {} {} -> {} (pixels: {})", todo.width, todo.height, todo.channels, todo.path, pixels.len());
                save_image(&todo.path, todo.width, todo.height, todo.channels, &pixels);*/
                false
            }
            else {true}
        });
    }
}

impl Drop for AsyncSaveScreen
{
    fn drop(&mut self)
    {
        println!("drop");
        self.save();
    }
}

pub struct RenderTarget {}

impl RenderTarget
{
    pub fn write_to_color(gl: &Gl, x: i32, y: i32, width: usize, height: usize,
                          clear_color: Option<&Vec4>, color_texture: Option<&Texture2D>, render: &dyn Fn()) -> Result<(), Error>
    {
        Self::write(gl, x, y, width, height, clear_color, None, color_texture, None, render)
    }

    pub fn write_to_depth(gl: &Gl, x: i32, y: i32, width: usize, height: usize,
                          clear_depth: Option<f32>, depth_texture: Option<&Texture2D>, render: &dyn Fn()) -> Result<(), Error>
    {
        Self::write(gl, x, y, width, height, None, clear_depth, None, depth_texture, render)
    }

    pub fn write(gl: &Gl, x: i32, y: i32, width: usize, height: usize,
                 clear_color: Option<&Vec4>, clear_depth: Option<f32>,
                 color_texture: Option<&Texture2D>, depth_texture: Option<&Texture2D>,
                 render: &dyn Fn()) -> Result<(), Error>
    {
        gl.viewport(x, y, width, height);
        let id = RenderTarget::new_framebuffer(gl, if color_texture.is_some() {1} else {0})?;

        if let Some(color_texture) = color_texture {
            color_texture.bind_as_color_target(0);
        }

        if let Some(depth_texture) = depth_texture {
            depth_texture.bind_as_depth_target();
        }

        #[cfg(feature = "debug")]
        {
            gl.check_framebuffer_status().or_else(|message| Err(Error::FailedToCreateFramebuffer {message}))?;
        }
        RenderTarget::clear(gl, clear_color, clear_depth);

        render();

        gl.delete_framebuffer(Some(&id));

        if let Some(color_texture) = color_texture {
            color_texture.generate_mip_maps();
        }

        if let Some(depth_texture) = depth_texture {
            depth_texture.generate_mip_maps();
        }
        Ok(())
    }

    pub fn write_to_color_array(gl: &Gl, x: i32, y: i32, width: usize, height: usize,
                                clear_color: Option<&Vec4>,
                       color_texture_array: Option<&Texture2DArray>,
                                color_channel_count: usize, color_channel_to_texture_layer: &dyn Fn(usize) -> usize,
                                render: &dyn Fn()) -> Result<(), Error>
    {
        Self::write_array(gl, x, y, width, height, clear_color, None, color_texture_array, None, color_channel_count, color_channel_to_texture_layer, 0, render)
    }

    pub fn write_to_depth_array(gl: &Gl, x: i32, y: i32, width: usize, height: usize, clear_depth: Option<f32>,
                       depth_texture_array: Option<&Texture2DArray>, depth_layer: usize,
                                render: &dyn Fn()) -> Result<(), Error>
    {
        Self::write_array(gl, x, y, width, height,None, clear_depth, None, depth_texture_array, 0, &|i| {i}, depth_layer, render)
    }

    pub fn write_array(gl: &Gl, x: i32, y: i32, width: usize, height: usize,
                       clear_color: Option<&Vec4>, clear_depth: Option<f32>,
                       color_texture_array: Option<&Texture2DArray>,
                       depth_texture_array: Option<&Texture2DArray>,
                       color_channel_count: usize, color_channel_to_texture_layer: &dyn Fn(usize) -> usize,
                       depth_layer: usize, render: &dyn Fn()) -> Result<(), Error>
    {
        gl.viewport(x, y, width, height);
        let id = RenderTarget::new_framebuffer(gl, color_channel_count)?;

        if let Some(color_texture) = color_texture_array {
            for channel in 0..color_channel_count {
                color_texture.bind_as_color_target(color_channel_to_texture_layer(channel), channel);
            }
        }

        if let Some(depth_texture) = depth_texture_array {
            depth_texture.bind_as_depth_target(depth_layer);
        }

        #[cfg(feature = "debug")]
        {
            gl.check_framebuffer_status().or_else(|message| Err(Error::FailedToCreateFramebuffer {message}))?;
        }
        RenderTarget::clear(gl, clear_color, clear_depth);

        render();

        gl.delete_framebuffer(Some(&id));
        if let Some(color_texture) = color_texture_array {
            color_texture.generate_mip_maps();
        }

        if let Some(depth_texture) = depth_texture_array {
            depth_texture.generate_mip_maps();
        }
        Ok(())
    }

    // TODO: Read color and depth from rendertarget to cpu
    /*#[cfg(not(target_arch = "wasm32"))]
    pub fn read_color(&self, x: i32, y: i32, width: usize, height: usize) -> Result<Vec<u8>, Error>
    {
        let color_texture = self.color_texture.as_ref().unwrap();
        self.gl.viewport(x, y, width, height);
        let mut pixels = vec![0u8; width * height * 3];
        let id = RenderTarget::new_framebuffer(&self.gl, 1)?;
        color_texture.bind_as_color_target(0);
        self.gl.check_framebuffer_status().or_else(|message| Err(Error::FailedToCreateFramebuffer {message}))?;

        self.gl.bind_framebuffer(consts::READ_FRAMEBUFFER, Some(&id));
        self.gl.read_pixels(x as u32, y as u32, width as u32, height as u32, consts::RGB,
                            consts::UNSIGNED_BYTE, &mut pixels);
        self.gl.delete_framebuffer(Some(&id));
        Ok(pixels)
    }*/

    /*#[cfg(not(target_arch = "wasm32"))]
    pub fn read_depth(&self, x: i32, y: i32, width: usize, height: usize) -> Result<Vec<f32>, Error>
    {
        let depth_texture = self.depth_texture.as_ref().unwrap();
        self.gl.viewport(x, y, width, height);
        let mut pixels = vec![0f32; width * height];
        let id = RenderTarget::new_framebuffer(&self.gl, 0)?;
        depth_texture.bind_as_depth_target();
        self.gl.check_framebuffer_status().or_else(|message| Err(Error::FailedToCreateFramebuffer {message}))?;

        self.gl.bind_framebuffer(consts::READ_FRAMEBUFFER, Some(&id));
        self.gl.read_depths(x as u32, y as u32, width as u32, height as u32,
                        consts::DEPTH_COMPONENT, consts::FLOAT, &mut pixels);
        self.gl.delete_framebuffer(Some(&id));
        Ok(pixels)
    }*/

    fn new_framebuffer(gl: &Gl, no_color_channels: usize) -> Result<crate::gl::Framebuffer, Error>
    {
        let id = gl.create_framebuffer()
            .ok_or_else(|| Error::FailedToCreateFramebuffer {message: "Failed to create framebuffer".to_string()} )?;
        gl.bind_framebuffer(consts::DRAW_FRAMEBUFFER, Some(&id));

        if no_color_channels > 0 {
            let mut draw_buffers = Vec::new();
            for i in 0..no_color_channels {
                draw_buffers.push(consts::COLOR_ATTACHMENT0 + i as u32);
            }
            gl.draw_buffers(&draw_buffers);
        }
        Ok(id)
    }

    fn clear(gl: &Gl, clear_color: Option<&Vec4>, clear_depth: Option<f32>) {
        if let Some(color) = clear_color {
            if let Some(depth) = clear_depth {
                gl.clear_color(color.x, color.y, color.z, color.w);
                depth_write(gl,true);
                gl.clear_depth(depth);
                gl.clear(consts::COLOR_BUFFER_BIT | consts::DEPTH_BUFFER_BIT);
            }
            else {
                gl.clear_color(color.x, color.y, color.z, color.w);
                gl.clear(consts::COLOR_BUFFER_BIT);
            }
        } else if let Some(depth) = clear_depth {
            gl.clear_depth(depth);
            depth_write(gl, true);
            gl.clear(consts::DEPTH_BUFFER_BIT);
        }
    }

    /*pub fn blit_color_and_depth_to(&self, target: &RenderTarget, target_color_texture: &Texture2D, target_depth_texture: &Texture2D)
    {
        self.read();
        target.write_to_color_and_depth(target_color_texture, target_depth_texture).unwrap();
        depth_write(&self.gl, true);
        self.gl.blit_framebuffer(0, 0, target_color_texture.width as u32, target_color_texture.height as u32,
                                 0, 0, target_color_texture.width as u32, target_color_texture.height as u32,
                                 consts::COLOR_BUFFER_BIT | consts::DEPTH_BUFFER_BIT, consts::NEAREST);
    }

    pub fn blit_color_to(&self, target: &RenderTarget, target_texture: &Texture2D)
    {
        self.read();
        target.write_to_color(target_texture).unwrap();
        self.gl.blit_framebuffer(0, 0, target_texture.width as u32, target_texture.height as u32,
                                 0, 0, target_texture.width as u32, target_texture.height as u32,
                                 consts::COLOR_BUFFER_BIT, consts::NEAREST);
    }

    pub fn blit_depth_to(&self, target: &RenderTarget, target_texture: &Texture2D)
    {
        self.read();
        target.write_to_depth(target_texture).unwrap();
        depth_write(&self.gl, true);
        self.gl.blit_framebuffer(0, 0, target_texture.width as u32, target_texture.height as u32,
                                 0, 0, target_texture.width as u32, target_texture.height as u32,
                                 consts::DEPTH_BUFFER_BIT, consts::NEAREST);
    }*/
}