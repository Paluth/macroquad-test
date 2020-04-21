use quad_gl::{QuadGl, Vertex};

pub use quad_gl::{colors::*, Color, Image, Texture2D};

pub enum ScreenCoordinates {
    Fixed(f32, f32, f32, f32),
    PixelPerfect,
}

pub struct DrawContext {
    pub(crate) font_texture: Texture2D,
    pub(crate) gl: QuadGl,
    pub(crate) screen_coordinates: ScreenCoordinates,
    pub ui: megaui::Ui,
    ui_draw_list: Vec<megaui::DrawList>,
}

impl DrawContext {
    pub fn new(ctx: &mut miniquad::Context) -> DrawContext {
        let ui = megaui::Ui::new();
        let texture_data = &ui.font_atlas.texture;
        let font_texture = Texture2D::from_rgba8(
            ctx,
            texture_data.width as u16,
            texture_data.height as u16,
            &texture_data.data,
        );
        let mut draw_context = DrawContext {
            screen_coordinates: ScreenCoordinates::PixelPerfect,
            gl: QuadGl::new(ctx),
            font_texture,
            ui,
            ui_draw_list: Vec::with_capacity(10000),
        };

        draw_context.update_projection_matrix(ctx);

        draw_context
    }

    fn draw_ui(&mut self, _: &mut miniquad::Context) {
        self.ui_draw_list.clear();

        self.ui.render(&mut self.ui_draw_list);
        self.ui.new_frame();

        let mut ui_draw_list = vec![];

        std::mem::swap(&mut ui_draw_list, &mut self.ui_draw_list);
        self.gl.texture(Some(self.font_texture));

        for draw_command in &ui_draw_list {
            self.gl.scissor(
                draw_command
                    .clipping_zone
                    .map(|rect| (rect.x as i32, rect.y as i32, rect.w as i32, rect.h as i32)),
            );
            self.gl
                .geometry(&draw_command.vertices, &draw_command.indices);
        }
        self.gl.texture(None);

        std::mem::swap(&mut ui_draw_list, &mut self.ui_draw_list);
    }

    pub fn draw_rectangle(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        #[rustfmt::skip]
        let vertices = [
            Vertex::new(x    , y    , 0., 0.0, 1.0, color),
            Vertex::new(x + w, y    , 0., 1.0, 0.0, color),
            Vertex::new(x + w, y + h, 0., 1.0, 1.0, color),
            Vertex::new(x    , y + h, 0., 0.0, 0.0, color),
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        self.gl.texture(None);
        self.gl.geometry(&vertices, &indices);
    }

    pub fn draw_texture(&mut self, texture: Texture2D, x: f32, y: f32, color: Color) {
        let w = texture.width();
        let h = texture.height();

        #[rustfmt::skip]
        let vertices = [
            Vertex::new(x    , y    , 0., 0.0, 0.0, color),
            Vertex::new(x + w, y    , 0., 1.0, 0.0, color),
            Vertex::new(x + w, y + h, 0., 1.0, 1.0, color),
            Vertex::new(x    , y + h, 0., 0.0, 1.0, color),
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        self.gl.texture(Some(texture));
        self.gl.geometry(&vertices, &indices);
    }

    pub fn draw_rectangle_lines(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.draw_rectangle(x, y, w, 1., color);
        self.draw_rectangle(x + w - 1., y + 1., 1., h - 2., color);
        self.draw_rectangle(x, y + h - 1., w, 1., color);
        self.draw_rectangle(x, y + 1., 1., h - 2., color);
    }

    /// Draw texture to x y w h position on the screen, using sx sy sw sh as a texture coordinates.
    /// Good use example: drawing an image from texture atlas.
    ///
    /// TODO: maybe introduce Rect type?
    pub fn draw_texture_rec(
        &mut self,
        texture: Texture2D,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sx: f32,
        sy: f32,
        sw: f32,
        sh: f32,
        color: Color,
    ) {
        #[rustfmt::skip]
        let vertices = [
            Vertex::new(x    , y    , 0.,  sx      /texture.width(),  sy      /texture.height(), color),
            Vertex::new(x + w, y    , 0., (sx + sw)/texture.width(),  sy      /texture.height(), color),
            Vertex::new(x + w, y + h, 0., (sx + sw)/texture.width(), (sy + sh)/texture.height(), color),
            Vertex::new(x    , y + h, 0.,  sx      /texture.width(), (sy + sh)/texture.height(), color),
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        self.gl.texture(Some(texture));
        self.gl.geometry(&vertices, &indices);
    }

    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let nx = -dy; // https://stackoverflow.com/questions/1243614/how-do-i-calculate-the-normal-vector-of-a-line-segment
        let ny = dx;

        let tlen = (nx * nx + ny * ny).sqrt() / (thickness * 0.5);
        if tlen < std::f32::EPSILON {
            return;
        }
        let tx = nx / tlen;
        let ty = ny / tlen;

        self.gl.texture(None);
        self.gl.geometry(
            &[
                Vertex::new(x1 + tx, y1 + ty, 0., 0., 0., color),
                Vertex::new(x1 - tx, y1 - ty, 0., 0., 0., color),
                Vertex::new(x2 + tx, y2 + ty, 0., 0., 0., color),
                Vertex::new(x2 - tx, y2 - ty, 0., 0., 0., color),
            ],
            &[0, 1, 2, 2, 1, 3],
        );
    }

    pub fn draw_circle(&mut self, x: f32, y: f32, r: f32, color: Color) {
        const NUM_DIVISIONS: u32 = 20;

        let mut vertices = Vec::<Vertex>::new();
        let mut indices = Vec::<u16>::new();

        vertices.push(Vertex::new(x, y, 0., 0., 0., color));
        for i in 0..NUM_DIVISIONS + 1 {
            let rx = (i as f32 / NUM_DIVISIONS as f32 * std::f32::consts::PI * 2.).cos();
            let ry = (i as f32 / NUM_DIVISIONS as f32 * std::f32::consts::PI * 2.).sin();

            let vertex = Vertex::new(x + r * rx, y + r * ry, 0., rx, ry, color);

            vertices.push(vertex);

            if i != NUM_DIVISIONS {
                indices.extend_from_slice(&[0, i as u16 + 1, i as u16 + 2]);
            }
        }

        self.gl.texture(None);
        self.gl.geometry(&vertices, &indices);
    }

    pub(crate) fn perform_render_passes(&mut self, ctx: &mut miniquad::Context) {
        self.draw_ui(ctx);
        self.gl.draw(ctx);
    }

    pub fn update_projection_matrix(&mut self, ctx: &mut miniquad::Context) {
        let (width, height) = ctx.screen_size();

        let projection = match self.screen_coordinates {
            ScreenCoordinates::PixelPerfect => {
                glam::Mat4::orthographic_rh_gl(0., width, height, 0., -1., 1.)
            }
            ScreenCoordinates::Fixed(left, right, bottom, top) => {
                glam::Mat4::orthographic_rh_gl(left, right, bottom, top, -1., 1.)
            }
        };

        self.gl.set_projection_matrix(projection);
    }
}
