#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

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

	pub rendering_process: Option<Child>,
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
}

impl App for ManglerApp
{
	fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame)
	{
		if !ctx.input(|input| input.pointer.any_down()) {
			if self.settings != self.prev_settings {
				if self.settings.path.is_empty() {
					*CURRENT_PREVIEW_STATE.lock().unwrap() = PreviewState::NoFile;
				} else {
					*CURRENT_PREVIEW_STATE.lock().unwrap() = PreviewState::Loading(self.settings.clone());
				}
			}

			self.prev_settings = self.settings.clone();
		}
		
		SidePanel::left("settings").show(ctx, |ui| {
			// ui.set_enabled(self.rendering_process.is_none());
			
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

			
			// ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
			// 	if ui.add_sized([70., 30.], Button::new("Render")).clicked() {
			// 		if let Some(path) = rfd::FileDialog::new()
			// 			.set_title("Select Media")
			// 			.add_filter("Audio/Video Files", MEDIA_EXTENSIONS)
			// 			.pick_file()
			// 		{
			// 			Command::new("ffmpeg")
			// 			 .args(shlex!("-i {}  {}"))
			// 		}
			// 	}
			// });


			ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
				let spacing = 20.;

				ui.add_space(spacing);
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

		CentralPanel::default().show(ctx, |ui|
		{
			ui.centered_and_justified(|ui| {
				match &*CURRENT_PREVIEW_STATE.lock().unwrap() {
					PreviewState::NoFile => ui.heading("(no file)"),
					PreviewState::Loading(_) => ui.spinner(),
					PreviewState::Loaded(img_data) => ui.image(("preview", img_data.clone())),
					PreviewState::Failed => ui.heading("Failed"),
					PreviewState::InternalError(err) => ui.heading(format!("Error: {err}")),
				}
			});
		});

	}
}

fn height_from_width(width: impl Into<f32>) -> u16 {
	(width.into() / (16./9.)).round() as u16
}
fn width_from_height(height: impl Into<f32>) -> u16 {
	(height.into() * (16./9.)).round() as u16
}

#[derive(Default)]
pub enum PreviewState {
	#[default]
	NoFile,
	Loading(Settings),
	Loaded(Vec<u8>),
	Failed,
	InternalError(io::Error),
}

pub static CURRENT_PREVIEW_STATE: Lazy<Mutex<PreviewState>> = Lazy::new(Default::default);

fn run_preview_thread() {
	thread::spawn(|| {
		loop {
			thread::sleep(Duration::from_secs_f32(0.2));

			let mut state = CURRENT_PREVIEW_STATE.lock().unwrap();
			if let Err(err) = || -> io::Result<()> {
				if let PreviewState::Loading(settings) = &*state {
					if !Path::new(&settings.path).exists() { return Err(io::Error::new(io::ErrorKind::NotFound, "File does not exist")) }

					let stdout = Command::new("ffprobe")
						.args(shlex!(r#"-i "{}" -show_entries format=duration -v quiet -of csv="p=0""#, settings.path))
						.output()?.stdout;

					let len: f32 = std::str::from_utf8(&stdout).map_err(invalid_data)?.trim().parse().map_err(invalid_data)?;

					if settings.mangle_video {
						Command::new("ffmpeg")
							.args(shlex!(r#"-ss {} -i "{}" -t 1 -b:v {}k -vf "{}" tmp.mp4"#, (fastrand::f32() * len - 1.).max(0.), settings.path, settings.video_bitrate, settings.video_effects()))
							.status()?;
					}

					Command::new("ffmpeg")
						.args(shlex!("-i {} -r 1/1 -frames:v 1 preview.png", if settings.mangle_video { "tmp.mp4" } else { settings.path.as_str() }))
						.output()?;

					if settings.mangle_video {
						fs::remove_file("tmp.mp4")?;
					}

					*state = PreviewState::Loaded(fs::read("preview.png")?);
				}

				Ok(())
			}() {
				*state = PreviewState::InternalError(err);
			}
		}
	});
}

fn invalid_data(err: impl std::error::Error + Send + Sync + 'static) -> io::Error {
	io::Error::new(io::ErrorKind::InvalidData, err)
}
