use std::{path::PathBuf, sync::Arc};

use eframe::egui_glow;
use egui::{mutex::Mutex, Color32, DragValue, Slider, Stroke};
use egui_glow::glow;
use glam::Vec3;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct CalibratorGui {
    #[serde(skip)]
    scene_3d: Option<Arc<Mutex<Scene3d>>>,
    calb_root_path: PathBuf,
}

impl CalibratorGui {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Self {
        // Load from eframe's storage
        let mut instance: Self = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();

        instance.scene_3d = Some(Arc::new(Mutex::new(Scene3d::new(
            cc.gl.as_ref().expect("GL Enabled"),
        ))));

        instance
    }
}

impl eframe::App for CalibratorGui {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("Left panel").show(ctx, |ui| self.left_panel(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::default().fill(Color32::BLACK).show(ui, |ui| {
                self.paint_view3d(ui);
            });
        });
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            self.scene_3d.as_ref().unwrap().lock().destroy(gl);
        }
    }
}

impl CalibratorGui {
    fn left_panel(&mut self, ui: &mut egui::Ui) {
        if ui.button("Begin recording").clicked() {
            todo!()
        }

        let path_text = self
            .calb_root_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or(self.calb_root_path.display().to_string());
        let path_text = format!("Path: {path_text}");

        if ui.button(path_text).clicked() {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                self.calb_root_path = folder;
            }
        }
    }

    fn paint_view3d(&mut self, ui: &mut egui::Ui) {
        let available_size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available_size, egui::Sense::drag());

        // Clone locals so we can move them into the paint callback:
        let rotating_triangle = self.scene_3d.clone().unwrap();

        let cb = egui_glow::CallbackFn::new(move |_info, painter| {
            rotating_triangle
                .lock()
                .paint(painter.gl(), 0.);
        });

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
    }
}

struct Scene3d {
    program: glow::Program,
    vertex_array: glow::VertexArray,
}

#[allow(unsafe_code)] // we need unsafe code to use glow
impl Scene3d {
    fn new(gl: &glow::Context) -> Self {
        use glow::HasContext as _;

        let shader_version = if cfg!(target_arch = "wasm32") {
            "#version 300 es"
        } else {
            "#version 330"
        };

        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let (vertex_shader_source, fragment_shader_source) = (
                r#"
                    const vec2 verts[3] = vec2[3](
                        vec2(0.0, 1.0),
                        vec2(-1.0, -1.0),
                        vec2(1.0, -1.0)
                    );
                    const vec4 colors[3] = vec4[3](
                        vec4(1.0, 0.0, 0.0, 1.0),
                        vec4(0.0, 1.0, 0.0, 1.0),
                        vec4(0.0, 0.0, 1.0, 1.0)
                    );
                    out vec4 v_color;
                    uniform float u_angle;
                    void main() {
                        v_color = colors[gl_VertexID];
                        gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
                        gl_Position.x *= cos(u_angle);
                    }
                "#,
                r#"
                    precision mediump float;
                    in vec4 v_color;
                    out vec4 out_color;
                    void main() {
                        out_color = v_color;
                    }
                "#,
            );

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                    gl.compile_shader(shader);
                    if !gl.get_shader_compile_status(shader) {
                        panic!("{}", gl.get_shader_info_log(shader));
                    }
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            Self {
                program,
                vertex_array,
            }
        }
    }

    fn destroy(&self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    fn paint(&self, gl: &glow::Context, angle: f32) {
        use glow::HasContext as _;
        unsafe {
            gl.use_program(Some(self.program));
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "u_angle").as_ref(),
                angle,
            );
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }
}
