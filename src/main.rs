use eframe::egui;
use learning_rust_code::{Status, Task, load_tasks, save_tasks};
// use rand::Rng; // Not needed with direct random() calls

// Confetti Particle System
struct Particle {
    pos: egui::Pos2,
    vel: egui::Vec2,
    color: egui::Color32,
    lifetime: f32,
}

struct TodoApp {
    tasks: Vec<Task>,
    new_task_input: String,
    particles: Vec<Particle>, // Confetti state
}

impl TodoApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 1. Custom Visuals (Theme)
        // Simplified visuals to avoid API mismatches in newer eframe versions
        let visuals = egui::Visuals::dark();
        // visuals.window_corner_radius = 10.0; // If supported
        cc.egui_ctx.set_visuals(visuals);

        // 2. Custom Fonts - using default for now to ensure compilation
        // cc.egui_ctx.set_pixels_per_point(1.2); // Scaling is safe

        let tasks = load_tasks().unwrap_or_else(|_| Vec::new());
        Self {
            tasks,
            new_task_input: String::new(),
            particles: Vec::new(),
        }
    }

    fn spawn_confetti(&mut self) {
        for _ in 0..50 {
            let x = 200.0 + rand::random::<f32>() * 400.0;
            let y = 100.0 + rand::random::<f32>() * 200.0;

            self.particles.push(Particle {
                pos: egui::pos2(x, y),
                // Faster velocity: -200..200
                vel: egui::vec2(
                    -200.0 + rand::random::<f32>() * 400.0,
                    -200.0 + rand::random::<f32>() * 400.0,
                ),
                color: egui::Color32::from_rgb(
                    (100.0 + rand::random::<f32>() * 155.0) as u8,
                    (100.0 + rand::random::<f32>() * 155.0) as u8,
                    (100.0 + rand::random::<f32>() * 155.0) as u8,
                ),
                lifetime: 1.5 + rand::random::<f32>(), // Slightly shorter life
            });
        }
    }

    fn update_particles(&mut self, ctx: &egui::Context) {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        self.particles.retain_mut(|p| {
            p.vel.y += 500.0 * dt; // Stronger Gravity (500 px/s^2)
            p.pos += p.vel * dt;
            p.lifetime -= dt;
            p.lifetime > 0.0
        });

        if !self.particles.is_empty() {
            ctx.request_repaint();
        }
    }

    fn render_particles(&self, ui: &mut egui::Ui) {
        let painter = ui.painter();
        for p in &self.particles {
            painter.rect_filled(
                egui::Rect::from_center_size(p.pos, egui::vec2(6.0, 6.0)),
                2.0,
                p.color,
            );
        }
    }

    fn render_dashboard(&self, ui: &mut egui::Ui) {
        let total = self.tasks.len();
        let done = self
            .tasks
            .iter()
            .filter(|t| matches!(t.status, Status::Done))
            .count();
        let todo = total - done;
        let percent = if total > 0 {
            (done as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        egui::Frame::new()
            .fill(egui::Color32::from_rgb(25, 25, 25))
            .corner_radius(8.0)
            .inner_margin(15.0)
            .show(ui, |ui| {
                ui.columns(4, |cols| {
                    cols[0].heading(
                        egui::RichText::new(format!("{}", total))
                            .size(24.0)
                            .strong(),
                    );
                    cols[0].label("Total Tasks");

                    cols[1].heading(
                        egui::RichText::new(format!("{}", done))
                            .size(24.0)
                            .strong()
                            .color(egui::Color32::GREEN),
                    );
                    cols[1].label("Completed");

                    cols[2].heading(
                        egui::RichText::new(format!("{}", todo))
                            .size(24.0)
                            .strong()
                            .color(egui::Color32::YELLOW),
                    );
                    cols[2].label("Pending");

                    cols[3].heading(
                        egui::RichText::new(format!("{:.0}%", percent))
                            .size(24.0)
                            .strong()
                            .color(egui::Color32::from_rgb(100, 149, 237)),
                    );
                    cols[3].label("Progress");
                });
            });
    }

    fn render_header(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading(
                egui::RichText::new("My Tasks")
                    .size(32.0)
                    .strong()
                    .color(egui::Color32::from_rgb(100, 149, 237)), // Cornflower Blue
            );
            ui.add_space(10.0);
        });
    }

    fn render_input(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Card-like background for input
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(30, 30, 30))
                .stroke(egui::Stroke::NONE)
                .inner_margin(10.0)
                .corner_radius(8.0) // Try corner_radius, fallback to rounding if fails
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        // Make input take available width minus button space
                        let available_width = ui.available_width() - 80.0;

                        let text_edit = ui.add(
                            egui::TextEdit::singleline(&mut self.new_task_input)
                                .hint_text("✨ Add a new amazing task...")
                                .desired_width(available_width)
                                .font(egui::FontId::proportional(18.0)), // Slightly larger font
                        );

                        ui.add_space(10.0);

                        let add_btn = ui.add(
                            egui::Button::new(egui::RichText::new("Add").size(16.0).strong())
                                .fill(egui::Color32::from_rgb(100, 149, 237))
                                .min_size(egui::vec2(60.0, 30.0)),
                        );

                        if add_btn.clicked()
                            || (text_edit.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            if !self.new_task_input.trim().is_empty() {
                                let id = self.tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                                let new_task = Task {
                                    id,
                                    description: self.new_task_input.clone(),
                                    status: Status::Todo,
                                };
                                self.tasks.push(new_task);
                                self.new_task_input.clear();
                                if let Err(e) = save_tasks(&self.tasks) {
                                    eprintln!("Failed to save tasks: {}", e);
                                }
                            }
                        }
                    });
                });
        });
    }

    fn render_tasks(&mut self, ui: &mut egui::Ui) {
        let mut status_changed = false;
        let mut confetti_triggered = false;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(10.0);
                for task in &mut self.tasks {
                    // Task Card should fill width naturally in the container
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(35, 35, 35))
                        .corner_radius(8.0) // Updated from rounding
                        .inner_margin(12.0)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50)))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.set_min_width(ui.available_width()); // Force full width
                                ui.set_min_height(30.0);
                                let mut is_done = matches!(task.status, Status::Done);

                                if ui.add(egui::Checkbox::new(&mut is_done, "")).changed() {
                                    task.status = if is_done { Status::Done } else { Status::Todo };
                                    status_changed = true;
                                    if is_done {
                                        confetti_triggered = true;
                                    }
                                }

                                let label = if is_done {
                                    egui::RichText::new(&task.description)
                                        .strikethrough()
                                        .color(egui::Color32::GRAY)
                                        .size(18.0) // Larger font
                                } else {
                                    egui::RichText::new(&task.description).size(18.0) // Larger font
                                };
                                ui.label(label);
                            });
                        });
                    ui.add_space(12.0); // More spacing
                }
            });

        if status_changed {
            if let Err(e) = save_tasks(&self.tasks) {
                eprintln!("Failed to save tasks: {}", e);
            }
        }

        if confetti_triggered {
            self.spawn_confetti();
        }
    }
}

impl eframe::App for TodoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Animation loop
        self.update_particles(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            // Paint confetti behind UI? Or in front? In front is better.
            // But we paint at end of frame to overlay.

            let width = f32::min(1000.0, ui.available_width() - 40.0); // Use up to 1000px, or full width minus padding

            ui.vertical_centered(|ui| {
                ui.set_max_width(width);

                self.render_header(ui);
                ui.add_space(10.0);

                self.render_dashboard(ui); // New Dashboard
                ui.add_space(20.0);

                self.render_input(ui);
                ui.add_space(30.0);

                ui.separator();

                self.render_tasks(ui);
            });

            // Draw valid particles
            self.render_particles(ui);
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([600.0, 800.0])
            .with_min_inner_size([400.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Rust Task Manager",
        native_options,
        Box::new(|cc| Ok(Box::new(TodoApp::new(cc)))),
    )
}
