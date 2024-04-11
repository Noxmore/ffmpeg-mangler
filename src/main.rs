#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use wait_timeout::ChildExt;
keystone::flat! {
	prelude;
}

pub static MEDIA_EXTENSIONS: &[&str] = &["mp4", "mkv", "mov", "avi", "wmv", "webm", "wav", "aiff", "mp3", "ogg", "aac", "flac"];

macro_rules! shlex {($($arg:tt)*) => {
	shlex::split(&format!($($arg)*)).unwrap()
};}

fn main()
{
	let native_options = NativeOptions
	{
		// viewport: ViewportBuilder::default()
		// 	.with_icon(Arc::new(None))
		// ,
		..Default::default()
	};
	run_native("Media Mangler", native_options, Box::new(|cc| Box::new(ManglerApp::new(cc)))).unwrap();
}

#[derive(Debug, Clone, SmartDefault, PartialEq)]
pub struct Settings
{
	path: String,

	#[default(true)]
	mangle_video: bool,
	#[default(30)]
	fps: u16,
	#[default(640)]
	scale: u16,
	#[default(50)]
	noise_amount: u8,
	#[default(250)]
	video_bitrate: u16,

	#[default(true)]
	mangle_audio: bool,
	#[default(1)]
	audio_bitrate: u16,
	#[default(100.)]
	volume_multiplier: f32,
}

impl Settings {
	pub fn video_effects(&self) -> String {
		format!("scale={}:-1,fps={},noise=c0s={}:allf=t+u,unsharp=13:13:5", self.scale, self.fps, self.noise_amount)
	}

	pub fn audio_effects(&self) -> String {
		format!("volume={}", self.volume_multiplier)
	}
}

#[derive(Default)]
pub struct ManglerApp {
	pub settings: Settings,
	pub prev_settings: Settings,
}

impl ManglerApp
{
	fn new(cc: &CreationContext<'_>) -> Self
	{
		let mut visuals = Visuals::dark();
		
		//let rounding = Rounding::none();
		//visuals.widgets.active.rounding = rounding;
		//visuals.widgets.inactive.rounding = rounding;
		//visuals.widgets.hovered.rounding = rounding;
		//visuals.widgets.noninteractive.rounding = rounding;
		visuals.widgets.noninteractive.fg_stroke = Stroke::new(1., Color32::WHITE);
		
		//let mut fonts = FontDefinitions::default();
		//for (_name, data) in &mut fonts.font_data { data.tweak.scale *= 1.5; }
		
		//cc.egui_ctx.set_fonts(fonts);
		cc.egui_ctx.set_visuals(visuals);
		cc.egui_ctx.set_pixels_per_point(1.2);

		egui_extras::install_image_loaders(&cc.egui_ctx);
		run_preview_thread();

		Self::default()
	}
	
	pub fn render(&self, path: PathBuf) {
		let settings = self.settings.clone();
		thread::spawn(move || {
			let start = Instant::now();
			*RENDER_STATE.lock().unwrap() = RenderState::Rendering(start.clone());
			
			let mut args = format!("-y -i \"{}\" ", settings.path);
			
			if settings.mangle_video {
				args += &format!("-b:v {}k -vf {} ", settings.video_bitrate, settings.video_effects());
			}
			if settings.mangle_audio {
				args += &format!("-b:a {}k -af {} ", settings.audio_bitrate, settings.audio_effects());
			}
			
			args += &format!("\"{}\"", path.display());
			
			// TODO handle these errors better
			let mut render = Command::new("ffmpeg")
				.args(shlex::split(&args).unwrap())
				.spawn().unwrap();
			
			loop {
				match render.wait_timeout(Duration::from_secs_f32(0.2)) {
					Ok(None) => if let RenderState::Abort = *RENDER_STATE.lock().unwrap() {
						render.kill().unwrap();
					}
					Ok(Some(_)) => break,
					Err(err) => panic!("Problem occurred while rendering: {err}"),
				}
			}
			
			*RENDER_STATE.lock().unwrap() = RenderState::Done(start.elapsed());
		});
	}
}

impl App for ManglerApp
{
	fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame)
	{
		let render_state = *RENDER_STATE.lock().unwrap();

		if !ctx.input(|input| input.pointer.any_down()) {
			if self.settings != self.prev_settings {
				if self.settings.path.is_empty() {
					*PREVIEW_STATE.lock().unwrap() = PreviewState::NoFile;
				} else {
					*PREVIEW_STATE.lock().unwrap() = PreviewState::Loading(Arc::new(self.settings.clone()));
				}
			}

			self.prev_settings = self.settings.clone();
		}
		
		SidePanel::left("settings").resizable(false).show(ctx, |ui| {
			ui.set_enabled(matches!(render_state, RenderState::Idle));
			
			ui.horizontal(|ui| {
				ui.label("File");
				ui.text_edit_singleline(&mut self.settings.path);
				if ui.button("...").clicked() {
					if let Some(path) = rfd::FileDialog::new()
						.set_title("Select Media")
						.add_filter("Audio/Video Files", MEDIA_EXTENSIONS)
						.pick_file()
					{
						self.settings.path = path.display().to_string();
					}
				}
			});


			Frame::window(ui.style()).outer_margin(6.).show(ui, |ui| {
				ScrollArea::vertical().auto_shrink([false, false]).max_height(ui.available_height() - 35.).show(ui, |ui| {
					let spacing = 20.;
	
					ui.heading("Video");
					ui.checkbox(&mut self.settings.mangle_video, "Mangle");
					
					ui.add_space(spacing);
					ui.label("FPS");
					Slider::new(&mut self.settings.fps, 1..=60).ui(ui);
	
	
					ui.add_space(spacing);
					ui.label("Scale");
					Slider::new(&mut self.settings.scale, 1..=1920)
						.custom_formatter(|n, _| format!("{n}x{}", height_from_width(n as f32)))
						.ui(ui)
						.on_hover_text("(Based on 16/9 aspect ratio, may not be accurate for all resolutions)");
					ui.horizontal(|ui| {
						if ui.button("144p").clicked() { self.settings.scale = width_from_height(144.) as u16 }
						if ui.button("240p").clicked() { self.settings.scale = width_from_height(240.) as u16 }
						if ui.button("360p").clicked() { self.settings.scale = width_from_height(360.) as u16 }
						if ui.button("480p").clicked() { self.settings.scale = width_from_height(480.) as u16 }
						if ui.button("720p").clicked() { self.settings.scale = width_from_height(720.) as u16 }
					});
	
	
					ui.add_space(spacing);
					ui.label("Noise Amount");
					Slider::new(&mut self.settings.noise_amount, 0..=100).ui(ui);
	
					ui.add_space(spacing);
					ui.label("Video Bitrate (kbps)");
					DragValue::new(&mut self.settings.video_bitrate).ui(ui);
	
	
					ui.add_space(spacing);
					ui.heading("Audio");
					ui.checkbox(&mut self.settings.mangle_audio, "Mangle");
	
					ui.add_space(spacing);
					ui.label("Audio Bitrate (kbps)");
					DragValue::new(&mut self.settings.audio_bitrate).ui(ui);
	
					ui.add_space(spacing);
					ui.label("Volume Multiplier");
					DragValue::new(&mut self.settings.volume_multiplier).ui(ui);
					
				});
			});
			

			ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
				if ui.add_sized([70., 30.], Button::new("Render")).clicked() {
					if let Some(path) = rfd::FileDialog::new()
						.set_title("Render to File")
						.add_filter("Audio/Video Files", MEDIA_EXTENSIONS)
						.save_file()
					{
						self.render(path);
					}
				}
			});
		});

		CentralPanel::default().show(ctx, |ui|
		{
			ui.centered_and_justified(|ui| {
				match &*PREVIEW_STATE.lock().unwrap() {
					PreviewState::NoFile => ui.heading("(no file)"),
					PreviewState::Loading(_) => { ctx.forget_image("preview"); ui.spinner() },
					PreviewState::Loaded(img_data) => ui.image(("preview", img_data.clone())),
					PreviewState::Failed => ui.heading("Failed"),
					PreviewState::InternalError(err) => ui.heading(format!("Error: {err}")),
				}
			});
		});


		if matches!(render_state, RenderState::Rendering(_) | RenderState::Done(_)) {
			Window::new("Rendering...")
				.title_bar(false)
				.fixed_size([250., 200.])
				.anchor(Align2::CENTER_CENTER, Vec2::ZERO)
				.show(ctx, |ui| {
					match render_state {
						RenderState::Rendering(start) => {
							ui.vertical_centered(|ui| ui.heading("Rendering..."));
							ui.centered_and_justified(|ui| {
								ui.spinner();
							});
							ui.horizontal(|ui| {
								ui.label(format!("Elapsed time: {:.1}s", start.elapsed().as_secs_f32()));
								ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| if ui.button("Abort").clicked() {
									*RENDER_STATE.lock().unwrap() = RenderState::Abort;
								});
							});
						}
						RenderState::Done(elapsed) => {
							ui.vertical_centered(|ui| ui.heading("Render complete"));
							ui.centered_and_justified(|ui| {
								ui.label(RichText::new("✅").heading().color(Color32::GREEN));
							});
							ui.horizontal(|ui| {
								ui.label(format!("Elapsed time: {:.1}s", elapsed.as_secs_f32()));
								ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| if ui.button("Close").clicked() {
									*RENDER_STATE.lock().unwrap() = RenderState::Idle;
								});
							});
						}
						_ => unreachable!(),
					}
				});
		}
	}
}

pub const DEFAULT_ASPECT_RATIO: f32 = 16./9.;

fn height_from_width(width: impl Into<f32>) -> u16 {
	(width.into() / DEFAULT_ASPECT_RATIO).round() as u16
}
fn width_from_height(height: impl Into<f32>) -> u16 {
	(height.into() * DEFAULT_ASPECT_RATIO).round() as u16
}

#[derive(Default)]
pub enum PreviewState {
	#[default]
	NoFile,
	Loading(Arc<Settings>),
	Loaded(Arc<[u8]>),
	Failed,
	InternalError(io::Error),
}

#[derive(Default, Clone, Copy)]
pub enum RenderState {
	#[default]
	Idle,
	Rendering(Instant),
	Abort,
	Done(Duration),
}

pub static PREVIEW_STATE: Lazy<Mutex<PreviewState>> = Lazy::new(Default::default);
pub static RENDER_STATE: Lazy<Mutex<RenderState>> = Lazy::new(Default::default);

fn run_preview_thread() {
	thread::spawn(|| {
		loop {
			thread::sleep(Duration::from_secs_f32(0.2));

			let loading_settings = match &*PREVIEW_STATE.lock().unwrap() {
				PreviewState::Loading(settings) => Some(settings.clone()),
				_ => None,
			};
			if let Err(err) = (|| -> io::Result<()> {
				if let Some(settings) = loading_settings {
					if !Path::new(&settings.path).exists() { return Err(io::Error::new(io::ErrorKind::NotFound, "File does not exist")) }

					let stdout = Command::new("ffprobe")
						.args(shlex!(r#"-i "{}" -show_entries format=duration -v quiet -of csv="p=0""#, settings.path))
						.output()?.stdout;

					let len: f32 = std::str::from_utf8(&stdout).map_err(invalid_data)?.trim().parse().map_err(invalid_data)?;

					if settings.mangle_video {
						Command::new("ffmpeg")
							.args(shlex!(r#"-y -ss {} -i "{}" -t 1 -b:v {}k -vf "{}" tmp.mp4"#, (fastrand::f32() * len - 1.).max(0.), settings.path, settings.video_bitrate, settings.video_effects()))
							.status()?;
					}

					fs::remove_file("preview.png").ok();
					
					Command::new("ffmpeg")
						.args(shlex!("-y -i {} -r 1/1 -frames:v 1 preview.png", if settings.mangle_video { "tmp.mp4" } else { settings.path.as_str() }))
						.output()?;

					if settings.mangle_video {
						fs::remove_file("tmp.mp4")?;
					}

					*PREVIEW_STATE.lock().unwrap() = PreviewState::Loaded(fs::read("preview.png")?.into());
				}

				Ok(())
			})() {
				*PREVIEW_STATE.lock().unwrap() = PreviewState::InternalError(err);
			}
		}
	});
}

fn invalid_data(err: impl std::error::Error + Send + Sync + 'static) -> io::Error {
	io::Error::new(io::ErrorKind::InvalidData, err)
}
